use asciidots_core::primitives::{Directed, Direction, Orientation, Oriented};

use crate::{
    callstack::Callstack,
    dot_trait::{EngineDot, IsTrue},
    graph::NodeId,
    operator::Operand,
    queues::Pairable,
    selector::{AddrValue, ValueSelector},
};

pub struct GraphDot {
    value: AddrValue,
    direction: Direction,
    callstack: Callstack<NodeId>,
}

impl EngineDot for GraphDot {
    type Value = f64;
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
        if self.value.get().abs() < f64::EPSILON {
            false
        } else {
            true
        }
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
