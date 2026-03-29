use crate::ir::graph::{Edge, Graph, Node, PortType};

#[derive(Clone, Copy, Debug)]
struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    fn normalize(self) -> Self {
        let l = self.length().max(1e-4);
        Self {
            x: self.x / l,
            y: self.y / l,
        }
    }

    fn clamp(self, max: f32) -> Self {
        let l = self.length();
        if l > max {
            self.normalize() * max
        } else {
            self
        }
    }
}

use std::ops::{Add, AddAssign, Mul, Sub};

impl Add for Vec2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self {
            x: self.x + o.x,
            y: self.y + o.y,
        }
    }
}
impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self {
            x: self.x - o.x,
            y: self.y - o.y,
        }
    }
}
impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
        }
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Self) {
        self.x += o.x;
        self.y += o.y;
    }
}

// ===== Layout Node =====

struct LayoutNode {
    pos: Vec2,
    vel: Vec2,
    size: Vec2,
    original: Vec2,
}

// ===== TEXT SIZE (approx but good enough) =====

fn measure_node(node: &Node) -> Vec2 {
    let title = format!("{:?}", node.t);
    let coord = format!("({}, {})", node.position.x, node.position.y);

    let max_len = title.len().max(coord.len()) as f32;

    let char_w = 6.5;
    let line_h = 12.0;

    let width = max_len * char_w + 16.0;
    let height = line_h * 2.0 + 10.0;

    Vec2 {
        x: width,
        y: height,
    }
}

// ===== INIT =====

fn init_layout(graph: &Graph) -> Vec<LayoutNode> {
    let scale = 60.0;

    graph
        .nodes
        .iter()
        .map(|n| {
            let p = Vec2 {
                x: n.position.x as f32 * scale,
                y: n.position.y as f32 * scale,
            };

            LayoutNode {
                pos: p,
                vel: Vec2::zero(),
                size: measure_node(n),
                original: p,
            }
        })
        .collect()
}

// ===== FORCES =====

fn repulsion(a: Vec2, sa: Vec2, b: Vec2, sb: Vec2) -> Vec2 {
    let d = a - b;
    let dist2 = d.x * d.x + d.y * d.y + 1.0; // avoid div 0

    // sum of half-widths/half-heights squared (rough)
    let influence = (sa.x + sb.x) * 0.5 + (sa.y + sb.y) * 0.5;

    d.normalize() * (influence * 2000.0 / dist2)
}

fn collision(a: Vec2, sa: Vec2, b: Vec2, sb: Vec2) -> Vec2 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;

    let overlap_x = (sa.x + sb.x) * 0.5 - dx.abs();
    let overlap_y = (sa.y + sb.y) * 0.5 - dy.abs();

    if overlap_x > 0.0 && overlap_y > 0.0 {
        if overlap_x < overlap_y {
            Vec2 {
                x: -dx.signum() * overlap_x * 0.5,
                y: 0.0,
            }
        } else {
            Vec2 {
                x: 0.0,
                y: -dy.signum() * overlap_y * 0.5,
            }
        }
    } else {
        Vec2::zero()
    }
}

fn spring(a: Vec2, b: Vec2) -> Vec2 {
    let d = b - a;
    let dist = d.length().max(1.0);

    let rest = 120.0;
    let k = 0.01;

    d.normalize() * (k * (dist - rest))
}

fn directional(a: Vec2, b: Vec2, port: PortType) -> Vec2 {
    let d = b - a;
    let s = 0.03;

    match port {
        PortType::NORTH => {
            Vec2 {
                x: -d.x,
                y: -d.y.abs(),
            } * s
        }
        PortType::SOUTH => {
            Vec2 {
                x: -d.x,
                y: d.y.abs(),
            } * s
        }
        PortType::EAST => {
            Vec2 {
                x: d.x.abs(),
                y: -d.y,
            } * s
        }
        PortType::WEST => {
            Vec2 {
                x: -d.x.abs(),
                y: -d.y,
            } * s
        }
    }
}

fn anchor(a: Vec2, orig: Vec2) -> Vec2 {
    (orig - a) * 0.01
}

// ===== SIM =====

fn run_layout(graph: &Graph) -> Vec<LayoutNode> {
    let mut nodes = init_layout(graph);
    let dt = 0.1;

    for _ in 0..400 {
        let mut forces = vec![Vec2::zero(); nodes.len()];

        // pairwise
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let f_rep = repulsion(nodes[i].pos, nodes[i].size, nodes[j].pos, nodes[j].size);
                let f_col = collision(nodes[i].pos, nodes[i].size, nodes[j].pos, nodes[j].size);

                let f = f_rep + f_col;

                forces[i] += f;
                forces[j] += f * -1.0;
            }
        }

        // edges
        for e in &graph.edges {
            let (a, port) = e.from;
            let (b, _) = e.to;

            let f = spring(nodes[a].pos, nodes[b].pos);

            forces[a] += f;
            forces[b] += f * -1.0;

            forces[a] += directional(nodes[a].pos, nodes[b].pos, port);
        }

        // anchor
        for i in 0..nodes.len() {
            forces[i] += anchor(nodes[i].pos, nodes[i].original);
        }

        // integrate
        for (i, n) in nodes.iter_mut().enumerate() {
            let f = forces[i].clamp(100.0);

            n.vel += f.clamp(50.0) * dt;
            n.vel = n.vel * 0.85; // damping

            n.pos += n.vel * dt;
        }
    }

    nodes
}

// ===== PORT OFFSET =====

fn port_offset(size: Vec2, port: PortType) -> Vec2 {
    match port {
        PortType::NORTH => Vec2 {
            x: 0.0,
            y: -size.y * 0.5,
        },
        PortType::SOUTH => Vec2 {
            x: 0.0,
            y: size.y * 0.5,
        },
        PortType::EAST => Vec2 {
            x: size.x * 0.5,
            y: 0.0,
        },
        PortType::WEST => Vec2 {
            x: -size.x * 0.5,
            y: 0.0,
        },
    }
}
// ===== SVG =====

pub fn graph_to_svg(graph: &Graph) -> String {
    let nodes = run_layout(graph);

    // 1️⃣ Compute bounding box from nodes and edges
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    // Include nodes with their sizes
    for n in &nodes {
        let half_w = n.size.x * 0.5;
        let half_h = n.size.y * 0.5;
        min_x = min_x.min(n.pos.x - half_w);
        min_y = min_y.min(n.pos.y - half_h);
        max_x = max_x.max(n.pos.x + half_w);
        max_y = max_y.max(n.pos.y + half_h);
    }

    // Include edge endpoints (port offsets)
    for e in &graph.edges {
        let na = &nodes[e.from.0];
        let nb = &nodes[e.to.0];
        let pa = na.pos + port_offset(na.size, e.from.1);
        let pb = nb.pos + port_offset(nb.size, e.to.1);

        min_x = min_x.min(pa.x.min(pb.x));
        min_y = min_y.min(pa.y.min(pb.y));
        max_x = max_x.max(pa.x.max(pb.x));
        max_y = max_y.max(pa.y.max(pb.y));
    }

    // 2️⃣ Add proportional padding based on **node sizes**
    let pad_x = (max_x - min_x) * 0.05; // 5% horizontal padding
    let pad_y = (max_y - min_y) * 0.05; // 5% vertical padding

    min_x -= pad_x;
    min_y -= pad_y;
    max_x += pad_x;
    max_y += pad_y;

    let width = max_x - min_x;
    let height = max_y - min_y;

    // 3️⃣ Output the SVG with correct viewBox and consistent coordinate shift
    let mut out = String::new();
    out.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="{vx} {vy} {w} {h}" style="background:#0b0f14">"##,
        w = width*2.0,
        h = height*2.0,
        vx = min_x,
        vy = min_y
    ));

    // edges
    for e in &graph.edges {
        let (a, ap) = e.from;
        let (b, bp) = e.to;

        let na = &nodes[a];
        let nb = &nodes[b];

        let pa = na.pos + port_offset(na.size, ap);
        let pb = nb.pos + port_offset(nb.size, bp);

        let x1 = pa.x;
        let y1 = pa.y;
        let x2 = pb.x;
        let y2 = pb.y;

        out.push_str(&format!(
            r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#6aa9ff" stroke-width="1.2" opacity="0.4"/>"##,
            x1, y1, x2, y2
        ));

        let mx = (x1 + x2) * 0.5;
        let my = (y1 + y2) * 0.5;

        if e.ticks != 0 {
            out.push_str(&format!(
                r##"<text x="{:.1}" y="{:.1}" font-size="9" fill="#ffffff" text-anchor="middle">{}</text>"##,
                mx, my, e.ticks
            ));
        }
    }

    // nodes
    for (i, n) in nodes.iter().enumerate() {
        let x = n.pos.x;
        let y = n.pos.y;

        let w = n.size.x;
        let h = n.size.y;

        out.push_str(&format!(
            r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="6" fill="#1a2230" stroke="#5a7bd8"/>"##,
            x - w * 0.5,
            y - h * 0.5,
            w,
            h
        ));

        let title = format!("{:?}", graph.nodes[i].t);
        let coord = format!(
            "({}, {})",
            graph.nodes[i].position.x, graph.nodes[i].position.y
        );

        out.push_str(&format!(
            r##"<text x="{:.1}" y="{:.1}" font-size="10" fill="#ffd166" text-anchor="middle">{}</text>"##,
            x,
            y - 4.0,
            title
        ));

        out.push_str(&format!(
            r##"<text x="{:.1}" y="{:.1}" font-size="9" fill="#9fb3ff" text-anchor="middle">{}</text>"##,
            x,
            y + 10.0,
            coord
        ));
    }

    out.push_str("</svg>");
    out
}
