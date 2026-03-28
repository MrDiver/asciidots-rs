use asciidots_core::{
    engine::operator::{ConditionalOperator, Operator},
    primitives::Position,
};

use crate::ir::dot::GraphDot;

pub type NodeId = usize;

pub enum PrintKind {
    Normal(String),
    Stream(String),
    Addr,
    Value,
}

pub enum SetKind {
    Input,
    Addr(f64),
    Value(f64),
}

pub enum CallKind {
    NativeCall(String),
    AsciiCall(String),
}

pub enum NodeType {
    Print(PrintKind),
    Set(SetKind),
    Duplicate,
    Dead,
    Call(CallKind),
    Return,
    Operator(Operator<GraphDot, f64>),
    Conditonal(ConditionalOperator<GraphDot>),
}

pub struct Node {
    position: Position,
    t: NodeType,
}

impl Node {
    pub fn new(position: Position, t: NodeType) -> Self {
        Self { position, t }
    }
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
