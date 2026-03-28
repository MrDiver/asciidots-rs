use asciidots_core::{
    engine::operator::{ConditionalOperator, Operator},
    primitives::Position,
};

use crate::dot::GraphDot;

pub type NodeId = usize;

pub enum NodeType {
    Print(String),
    PrintStream(String),
    Duplicate,
    Dead,
    Operator(Operator<GraphDot, f64>),
    Conditonal(ConditionalOperator<GraphDot>),
}

pub struct Node {
    position: Position,
    t: NodeType,
}

pub enum PortType {
    NORTH,
    EAST,
    SOUTH,
    WEST,
}

pub struct Edge {
    from: (usize, PortType),
    to: (usize, PortType),
    ticks: u64,
}

pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}
