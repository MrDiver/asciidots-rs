use crate::ir::graph::{Graph, Node, PortType};
use serde::Serialize;

#[derive(Serialize)]
struct JsonNode {
    id: usize,
    label: String,
    x: i32,
    y: i32,
}

#[derive(Serialize)]
struct JsonEdge {
    from: usize,
    to: usize,
    from_port: PortType,
    to_port: PortType,
    ticks: u64,
}

#[derive(Serialize)]
struct JsonGraph {
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
}

pub fn graph_to_json(graph: &Graph) -> String {
    let nodes = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| JsonNode {
            id: i,
            label: format!("{:?}", n.t),
            x: n.position.x,
            y: n.position.y,
        })
        .collect();

    let edges = graph
        .edges
        .iter()
        .map(|e| JsonEdge {
            from: e.from.0,
            to: e.to.0,
            from_port: e.from.1,
            to_port: e.to.1,
            ticks: e.ticks,
        })
        .collect();

    serde_json::to_string_pretty(&JsonGraph { nodes, edges }).unwrap()
}
