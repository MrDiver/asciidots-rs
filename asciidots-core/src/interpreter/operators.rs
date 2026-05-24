use crate::{
    engine::operator::{ArithOp, ConditionalOperator, Operator},
    primitives::Orientation,
    state_machine::DotMachine,
};
use std::marker::PhantomData;

enum OperatorKind {
    Arithmetic(Operator<DotMachine, f64>),
    Conditional(ConditionalOperator<DotMachine>),
}

pub struct OperatorQueues {
    kind: OperatorKind,
}

impl OperatorQueues {
    pub(super) fn new(orientation: Orientation, op: char, negated: bool) -> Option<Self> {
        let kind = if op == '~' {
            OperatorKind::Conditional(ConditionalOperator {
                queue: Default::default(),
                negated,
            })
        } else {
            OperatorKind::Arithmetic(Operator {
                op: ArithOp::from_char(op)?,
                queue: Default::default(),
                kind: orientation,
                _marker: PhantomData,
            })
        };
        Some(Self { kind })
    }

    pub(super) fn push_and_get_result(&mut self, dot: DotMachine) -> Option<DotMachine> {
        match &mut self.kind {
            OperatorKind::Arithmetic(op) => op.enter(dot),
            OperatorKind::Conditional(op) => op.enter(dot),
        }
    }

    pub fn dots(&self) -> impl Iterator<Item = &DotMachine> {
        let queue = match &self.kind {
            OperatorKind::Arithmetic(op) => &op.queue,
            OperatorKind::Conditional(op) => &op.queue,
        };
        queue.iter()
    }

    pub fn waiting_len(&self) -> usize {
        match &self.kind {
            OperatorKind::Arithmetic(op) => op.queue.len(),
            OperatorKind::Conditional(op) => op.queue.len(),
        }
    }
}
