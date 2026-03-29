use asciidots_core::{
    engine::{
        callstack::Callstack,
        dot_trait::{EngineDot, IsTrue},
        operator::Operand,
        queues::Pairable,
    },
    primitives::{Directed, Direction, Orientation, Oriented},
    state_machine::ValueSelector as CoreValueSelector,
};

use crate::ir::{
    graph::NodeId,
    selector::{AddrValue, AddrValueSelector, ValueSelector},
};

#[derive(Clone)]
pub struct GraphDot {
    value: AddrValue,
    direction: Direction,
    callstack: Callstack<NodeId>,
}

impl GraphDot {
    pub fn new(direction: Direction) -> Self {
        Self {
            value: AddrValue::default(),
            direction,
            callstack: Callstack::default(),
        }
    }

    pub fn set_addr(&mut self, v: f64) {
        self.value.set_addr(v);
    }

    /// Switches the active register without resetting.
    pub fn apply_select(&mut self, sel: CoreValueSelector) {
        match sel {
            CoreValueSelector::Value => self.value.select(AddrValueSelector::Value),
            CoreValueSelector::Addr  => self.value.select(AddrValueSelector::Addr),
        }
    }
}

impl EngineDot for GraphDot {
    type Value = f64;
    type CallstackKey = NodeId;

    fn selected(&self) -> f64 {
        self.value.get()
    }

    fn set_selected(&mut self, value: f64) {
        self.value.set(value);
        self.value.reset_selection();
    }

    fn addr(&self) -> f64 {
        self.value.addr()
    }

    fn current_location(&self) -> NodeId {
        0 // TODO: track current node
    }

    fn callstack_key_from_addr(&self) -> NodeId {
        self.value.addr() as NodeId
    }

    fn pop_callstack(&mut self) -> Option<NodeId> {
        self.callstack.pop()
    }

    fn push_callstack(&mut self, key: NodeId) {
        self.callstack.push(&key);
    }
}

impl Directed for GraphDot {
    fn direction(&self) -> Direction {
        self.direction
    }

    fn set_direction(&mut self, direction: Direction) {
        self.direction = direction
    }
}

impl IsTrue for GraphDot {
    fn is_true(&self) -> bool {
        self.value.get().abs() >= f64::EPSILON
    }
}

impl Oriented for GraphDot {
    fn orientation(&self) -> Orientation {
        self.direction.orientation()
    }
}

impl Pairable for GraphDot {
    fn key(&self) -> u64 {
        self.callstack.current_hash()
    }
}

impl Operand<f64> for GraphDot {
    fn apply_operator(mut self, other: Self, op: impl Fn(f64, f64) -> f64) -> Self {
        let a = self.value.get();
        let b = other.value.get();
        let c = op(a, b);
        self.value.set(c);
        self.value.reset_selection();
        self
    }
}
