use asciidots_core::{
    primitives::{Direction, Orientation, Position},
    state_machine::ValueSelector,
};
use rustc_hash::FxHashMap;
use serde::Serialize;

pub type NodeId = usize;

#[derive(Debug)]
pub enum PrintKind {
    Normal(String),
    Stream(String),
    Addr,
    Value,
}

#[derive(Debug)]
pub enum SetKind {
    Input,
    Addr(f64),
    Value(f64),
}

#[derive(Debug)]
pub enum CallKind {
    NativeCall(String),
    AsciiCall(u32),
}

#[derive(Debug)]
pub enum NodeType {
    CreateDot(Direction),
    /// Per-direction print behaviour; index via dir_index()
    Print([Option<PrintKind>; 4]),
    /// Per-direction set behaviour; index via dir_index()
    Set([Option<SetKind>; 4]),
    Duplicate,
    EndProgram,
    Call(CallKind),
    CallEnter,
    Return,
    /// repr is the operator character, orientation is the primary axis
    Operator(char, Orientation),
    /// bool: true if negated (a `!` is positioned below the conditional)
    Conditional(bool),
    Filter(bool),
    /// Expected value to assert against `dot.selected()`; `None` means no literal follows.
    Assertion(Option<f64>),
    /// Pass-through node that updates the dot's active register (value or addr)
    Select(ValueSelector),
    /// Sink node — consumes dots from all directions and emits display events at runtime
    Display(u32),
}

#[derive(Debug)]
pub struct Node {
    pub position: Position,
    pub t: NodeType,
}

impl Node {
    pub fn new(position: Position, t: NodeType) -> Self {
        Self { position, t }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum PortType {
    NORTH,
    EAST,
    SOUTH,
    WEST,
}

#[derive(Debug)]
pub struct Edge {
    pub from: (usize, PortType),
    pub to: (usize, PortType),
    pub ticks: u64,
}

#[derive(Debug)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Maps function id (u32) to the CallEnter NodeId for that function
    pub func_entry_nodes: FxHashMap<u32, NodeId>,
}
