use std::{
    collections::BTreeSet,
    env,
    io::stdout,
    path::PathBuf,
    time::{Duration, Instant},
};

use asciidots_core::{
    display::DisplayBackend, interpreter::InterpreterResult, linker,
    native::nativelib::default_native_library_for, state_machine::ValueSelector,
};
use asciidots_display_ratatui::{DisplayWidget, RatatuiDisplayBackend};
use asciidots_graph::{
    compiler::tracer::Tracer,
    ir::{
        dot::GraphDot,
        graph::{Graph, NodeId, NodeType},
    },
    runtime::executor::{GraphExecutor, PendingDotSnapshot},
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Widget},
};
use rustc_hash::FxHashSet;

const FRAME_TIME: Duration = Duration::from_millis(50);

fn build_executor(path: &PathBuf) -> GraphExecutor {
    let available = BTreeSet::new();
    let linked = linker::link_file(path, &available).expect("link_file failed");
    let native_names: FxHashSet<String> = linked.native_imports.keys().cloned().collect();
    let mut tracer = Tracer::new(linked.module, linked.native_imports, native_names);
    tracer.trace();
    let graph = tracer.into_graph();
    let native_registry = default_native_library_for::<GraphDot>()
        .into_iter()
        .map(|f| (f.name().to_string(), f))
        .collect();
    GraphExecutor::new(graph, native_registry)
}

struct App {
    path: PathBuf,
    executor: GraphExecutor,
    display: RatatuiDisplayBackend,
    paused: bool,
    step_count: u64,
    last_tick: Instant,
    dot_list_scroll: usize,
    graph_scroll: (i32, i32),
}

impl App {
    fn new(path: PathBuf, executor: GraphExecutor) -> Self {
        Self {
            path,
            executor,
            display: RatatuiDisplayBackend::new(),
            paused: true,
            step_count: 0,
            last_tick: Instant::now(),
            dot_list_scroll: 0,
            graph_scroll: (0, 0),
        }
    }

    fn tick(&mut self) {
        if self.paused || self.executor.is_finished() {
            return;
        }
        self.step_inner();
    }

    fn force_step(&mut self) {
        if self.executor.is_finished() {
            return;
        }
        self.step_inner();
    }

    fn step_inner(&mut self) {
        let results = self.executor.step().to_vec();
        for result in results {
            if let InterpreterResult::Display { command, .. } = result {
                self.display.handle(command);
            }
        }
        self.step_count += 1;
        self.last_tick = Instant::now();
    }

    fn reset(&mut self) {
        self.executor = build_executor(&self.path);
        self.display = RatatuiDisplayBackend::new();
        self.paused = true;
        self.step_count = 0;
        self.last_tick = Instant::now();
        self.dot_list_scroll = 0;
        self.graph_scroll = (0, 0);
    }
}

struct DotDebugTableModel<'a> {
    graph: &'a Graph,
    dots: &'a [PendingDotSnapshot],
}

impl<'a> DotDebugTableModel<'a> {
    const HEADERS: [&'static str; 8] = ["#", "From", "To", "Dir", "Value", "Addr", "Prog", "Ticks"];

    fn rows(&self, scroll: usize, height: usize) -> Vec<Row<'static>> {
        self.dots
            .iter()
            .enumerate()
            .skip(scroll)
            .take(height)
            .map(|(idx, dot)| {
                let from = &self.graph.nodes[dot.from_node];
                let to = &self.graph.nodes[dot.dest_node];
                Row::new(vec![
                    Cell::from(idx.to_string()),
                    Cell::from(format!(
                        "{}({},{})",
                        node_glyph(dot.from_node, &from.t),
                        from.position.x,
                        from.position.y
                    )),
                    Cell::from(format!(
                        "{}({},{})",
                        node_glyph(dot.dest_node, &to.t),
                        to.position.x,
                        to.position.y
                    )),
                    Cell::from(format!("{:?}", dot.direction)),
                    Cell::from(format!("{:.3}", dot.selected)),
                    Cell::from(format!("{:.3}", dot.addr)),
                    Cell::from(format!("{:.2}", progress(dot))),
                    Cell::from(format!("{}/{}", dot.ticks_left, dot.total_ticks)),
                ])
            })
            .collect()
    }
}

struct SettingsTableModel {
    rows: Vec<(String, String)>,
}

impl SettingsTableModel {
    const HEADERS: [&'static str; 2] = ["Setting", "Value"];

    fn from_app(app: &App) -> Self {
        Self {
            rows: vec![
                ("path".to_string(), app.path.display().to_string()),
                ("frame_ms".to_string(), FRAME_TIME.as_millis().to_string()),
                ("paused".to_string(), app.paused.to_string()),
                (
                    "finished".to_string(),
                    app.executor.is_finished().to_string(),
                ),
                ("step".to_string(), app.step_count.to_string()),
                (
                    "pending".to_string(),
                    app.executor.pending_len().to_string(),
                ),
                (
                    "graph_scroll".to_string(),
                    format!("{}, {}", app.graph_scroll.0, app.graph_scroll.1),
                ),
                ("controls".to_string(), "space pause/resume".to_string()),
                ("".to_string(), "n step  r reset  j/k list".to_string()),
                ("".to_string(), "w/a/s/d graph pan".to_string()),
            ],
        }
    }

    fn rows(&self) -> Vec<Row<'static>> {
        self.rows
            .iter()
            .map(|(k, v)| Row::new(vec![Cell::from(k.clone()), Cell::from(v.clone())]))
            .collect()
    }
}

struct GraphWidget<'a> {
    graph: &'a Graph,
    pending: &'a [PendingDotSnapshot],
    scroll: (i32, i32),
}

impl<'a> Widget for GraphWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let bounds = graph_bounds(self.graph);
        let inner_w = area.width.saturating_sub(2) as f32;
        let inner_h = area.height.saturating_sub(2) as f32;

        for (node_id, node) in self.graph.nodes.iter().enumerate() {
            let (x, y) = project(
                bounds,
                node.position.x as f32,
                node.position.y as f32,
                inner_w,
                inner_h,
                self.scroll,
            );
            if let Some(cell) = buf.cell_mut((area.x + 1 + x, area.y + 1 + y)) {
                cell.set_char(node_glyph(node_id, &node.t));
                cell.set_style(Style::default().fg(node_color(&node.t)));
            }
        }

        for dot in self.pending {
            let from = &self.graph.nodes[dot.from_node];
            let to = &self.graph.nodes[dot.dest_node];
            let (fx, fy) = interpolate_manhattan(
                from.position.x as f32,
                from.position.y as f32,
                to.position.x as f32,
                to.position.y as f32,
                progress(dot),
            );
            let (x, y) = project(bounds, fx, fy, inner_w, inner_h, self.scroll);
            if let Some(cell) = buf.cell_mut((area.x + 1 + x, area.y + 1 + y)) {
                cell.set_style(Style::default().bg(Color::Red));
            }
        }
    }
}

fn progress(dot: &PendingDotSnapshot) -> f32 {
    if dot.total_ticks == 0 {
        1.0
    } else {
        1.0 - (dot.ticks_left as f32 / dot.total_ticks as f32)
    }
}

fn interpolate_manhattan(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> (f32, f32) {
    let dx = (x2 - x1).abs();
    let dy = (y2 - y1).abs();
    let total = dx + dy;
    if total == 0.0 {
        return (x2, y2);
    }
    let travel = total * t.clamp(0.0, 1.0);
    if travel <= dx {
        let sx = if x2 >= x1 { 1.0 } else { -1.0 };
        (x1 + sx * travel, y1)
    } else {
        let sy = if y2 >= y1 { 1.0 } else { -1.0 };
        (x2, y1 + sy * (travel - dx))
    }
}

fn graph_bounds(graph: &Graph) -> (i32, i32, i32, i32) {
    let min_x = graph.nodes.iter().map(|n| n.position.x).min().unwrap_or(0);
    let max_x = graph.nodes.iter().map(|n| n.position.x).max().unwrap_or(0);
    let min_y = graph.nodes.iter().map(|n| n.position.y).min().unwrap_or(0);
    let max_y = graph.nodes.iter().map(|n| n.position.y).max().unwrap_or(0);
    (min_x, max_x, min_y, max_y)
}

fn project(
    bounds: (i32, i32, i32, i32),
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    scroll: (i32, i32),
) -> (u16, u16) {
    let (min_x, max_x, min_y, max_y) = bounds;
    let span_x = ((max_x.saturating_sub(min_x)).max(1)) as f32;
    let span_y = ((max_y.saturating_sub(min_y)).max(1)) as f32;
    let px = (((x - min_x as f32) / span_x * width) - scroll.0 as f32)
        .round()
        .clamp(0.0, width) as u16;
    let py = (((y - min_y as f32) / span_y * height) - scroll.1 as f32)
        .round()
        .clamp(0.0, height) as u16;
    (px, py)
}

fn node_glyph(_node_id: NodeId, node_type: &NodeType) -> char {
    match node_type {
        NodeType::CreateDot(_) => '.',
        NodeType::Display(_) => 'D',
        NodeType::Duplicate => '*',
        NodeType::Call(_) => '%',
        NodeType::CallEnter => '@',
        NodeType::Return => '@',
        NodeType::Conditional(_) => '~',
        NodeType::Operator(ch, _) => *ch,
        NodeType::Set(_) => '#',
        NodeType::Print(_) => '$',
        NodeType::Select(kind) => match kind {
            ValueSelector::Addr => '@',
            ValueSelector::Value => '#',
        },
        NodeType::Filter(kind) => match kind {
            true => ':',
            false => ';',
        },
        NodeType::Assertion(_) => '=',
        NodeType::EndProgram => '&',
    }
}

fn node_color(node_type: &NodeType) -> Color {
    match node_type {
        NodeType::CreateDot(_) => Color::Green,
        NodeType::Display(_) => Color::Cyan,
        NodeType::Duplicate => Color::Yellow,
        NodeType::Call(_) | NodeType::CallEnter | NodeType::Return => Color::Magenta,
        NodeType::Conditional(_) => Color::LightBlue,
        NodeType::Operator(_, _) => Color::White,
        _ => Color::DarkGray,
    }
}

fn panel_block<'a>(title: &'a str, color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Span::styled(
            title.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let vertical =
        Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(frame.area());
    let status = format!(
        "step={} pending={} paused={} finished={}  q quit  space pause  n step  r reset  j/k list  wasd graph",
        app.step_count,
        app.executor.pending_len(),
        app.paused,
        app.executor.is_finished()
    );
    frame.render_widget(
        Paragraph::new(status).block(panel_block("Graph Runtime", Color::LightMagenta)),
        vertical[0],
    );

    let panes = Layout::horizontal([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(vertical[1]);

    let graph_block = panel_block("Graph", Color::LightBlue);
    let graph_inner = graph_block.inner(panes[0]);
    frame.render_widget(graph_block, panes[0]);
    frame.render_widget(
        GraphWidget {
            graph: app.executor.graph(),
            pending: &app.executor.pending_snapshots(),
            scroll: app.graph_scroll,
        },
        graph_inner,
    );

    let right = Layout::vertical([
        Constraint::Percentage(35),
        Constraint::Length(9),
        Constraint::Min(8),
    ])
    .split(panes[1]);

    let display_block = panel_block("Display", Color::LightCyan);
    let display_inner = display_block.inner(right[0]);
    frame.render_widget(display_block, right[0]);
    frame.render_widget(DisplayWidget::new(app.display.buffer()), display_inner);

    let settings_model = SettingsTableModel::from_app(app);
    let settings_block = panel_block("Settings", Color::LightGreen);
    let settings_inner = settings_block.inner(right[1]);
    frame.render_widget(settings_block, right[1]);
    let settings_header = Row::new(SettingsTableModel::HEADERS.into_iter().map(Cell::from))
        .style(Style::default().fg(Color::Yellow));
    let settings_table = Table::new(
        settings_model.rows(),
        [Constraint::Length(12), Constraint::Min(10)],
    )
    .header(settings_header);
    frame.render_widget(settings_table, settings_inner);

    let pending = app.executor.pending_snapshots();
    let model = DotDebugTableModel {
        graph: app.executor.graph(),
        dots: &pending,
    };
    let dots_block = panel_block("Pending Dots", Color::Yellow);
    let dots_inner = dots_block.inner(right[2]);
    frame.render_widget(dots_block, right[2]);
    let header = Row::new(DotDebugTableModel::HEADERS.into_iter().map(Cell::from))
        .style(Style::default().fg(Color::Yellow));
    let visible_rows = dots_inner.height.saturating_sub(1) as usize;
    let table = Table::new(
        model.rows(app.dot_list_scroll, visible_rows),
        [
            Constraint::Length(3),
            Constraint::Length(11),
            Constraint::Length(11),
            Constraint::Length(5),
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Length(9),
        ],
    )
    .header(header);
    frame.render_widget(table, dots_inner);
}

fn run(path: PathBuf) -> std::io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let executor = build_executor(&path);
    let mut app = App::new(path, executor);

    loop {
        if event::poll(Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(' ') => app.paused = !app.paused,
                        KeyCode::Char('n') => app.force_step(),
                        KeyCode::Char('r') => app.reset(),
                        KeyCode::Char('j') => {
                            app.dot_list_scroll = app.dot_list_scroll.saturating_add(1);
                        }
                        KeyCode::Char('k') => {
                            app.dot_list_scroll = app.dot_list_scroll.saturating_sub(1);
                        }
                        KeyCode::Char('w') => {
                            app.graph_scroll.1 = app.graph_scroll.1.saturating_sub(1);
                        }
                        KeyCode::Char('s') => {
                            app.graph_scroll.1 = app.graph_scroll.1.saturating_add(1);
                        }
                        KeyCode::Char('a') => {
                            app.graph_scroll.0 = app.graph_scroll.0.saturating_sub(2);
                        }
                        KeyCode::Char('d') => {
                            app.graph_scroll.0 = app.graph_scroll.0.saturating_add(2);
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.last_tick.elapsed() >= FRAME_TIME {
            app.tick();
        }

        terminal.draw(|frame| draw(frame, &app))?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let path = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("../asciidots-graph/tests/fixtures/graph_juliaset_small.dots")
    });
    let result = run(path);
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen);
    result
}
