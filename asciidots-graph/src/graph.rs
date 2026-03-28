use asciidots_core::primitives::{Direction, Orientation, Oriented, Position};

use crate::{
    callstack::Callstack,
    dot::GraphDot,
    operator::{ConditionalOperator, Operand, Operator, OperatorPair, OperatorQueue},
    queues::Pairable,
    selector::{AddrValue, ValueSelector},
};

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
