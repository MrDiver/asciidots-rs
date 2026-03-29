use asciidots_core::{primitives::{Direction, Orientation, Position}, state_machine::ValueSelector};

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
    Assertion,
    /// Pass-through node that updates the dot's active register (value or addr)
    Select(ValueSelector),
    /// Sink node — consumes dots from all directions and emits display events at runtime
    Display(u32),
}

pub struct Node {
    pub position: Position,
    pub t: NodeType,
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
    pub from: (usize, PortType),
    pub to: (usize, PortType),
    pub ticks: u64,
}

pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
