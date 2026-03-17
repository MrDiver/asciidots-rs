use log::trace;

use crate::{
    primitives::{Direction, Orientation},
    state_machine::{DotMachine, SetKind},
};
use std::{collections::VecDeque, ops::Rem};

/// A small queue for dots waiting at one side of an operator.
///
/// Optimised for the common case where at most one dot is queued at a time:
/// the first arrival is stored inline as `Option<DotMachine>`, so no heap
/// allocation occurs until a second dot arrives before the first has been
/// consumed. Overflow beyond the first entry spills into a `VecDeque`.
#[derive(Debug, Default)]
struct DotQueue {
    first: Option<DotMachine>,
    overflow: VecDeque<DotMachine>,
}

impl DotQueue {
    fn push(&mut self, dot: DotMachine) {
        if self.first.is_none() {
            self.first = Some(dot);
        } else {
            self.overflow.push_back(dot);
        }
    }

    fn pop_front(&mut self) -> Option<DotMachine> {
        let dot = self.first.take()?;
        self.first = self.overflow.pop_front();
        Some(dot)
    }

    fn is_empty(&self) -> bool {
        self.first.is_none()
    }
}

#[derive(Debug)]
pub struct OperatorQueues {
    orientation: Orientation,
    op: char,
    kind: SetKind,
    negated: bool,
    horizontal: DotQueue,
    vertical: DotQueue,
}

impl OperatorQueues {
    pub(super) fn new(orientation: Orientation, op: char, kind: SetKind, negated: bool) -> Self {
        Self {
            orientation,
            op,
            kind,
            negated,
            horizontal: DotQueue::default(),
            vertical: DotQueue::default(),
        }
    }

    pub(super) fn push(&mut self, dot: DotMachine) {
        match dot.direction().orientation() {
            Orientation::Horizontal => self.horizontal.push(dot),
            Orientation::Vertical => self.vertical.push(dot),
        }
    }

    pub(super) fn get_result(&mut self) -> Option<DotMachine> {
        if !self.horizontal.is_empty() && !self.vertical.is_empty() {
            let (mut primary, secondary) = {
                let doth = self.horizontal.pop_front().unwrap();
                let dotv = self.vertical.pop_front().unwrap();
                if matches!(self.orientation, Orientation::Horizontal) {
                    (doth, dotv)
                } else {
                    (dotv, doth)
                }
            };

            if self.op == '~' {
                let condition = secondary;
                let mut new_dot = primary.dot();
                if condition.value() as u64 == 0 {
                    new_dot.set_dir(Direction::Up);
                }
                primary.replace_dot(new_dot);
                trace!("Returning from operator: {:?}", primary);
                return Some(primary);
            }

            let a = if self.kind == SetKind::Value {
                primary.value()
            } else {
                primary.addr() as f64
            };
            let b = secondary.value();
            let new_val = match self.op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                '%' => a.rem(b),
                '^' => a.powf(b),
                '=' => {
                    if a == b {
                        1.0
                    } else {
                        0.0
                    }
                }
                '!' => {
                    if a != b {
                        1.0
                    } else {
                        0.0
                    }
                }
                '>' => {
                    if a > b {
                        1.0
                    } else {
                        0.0
                    }
                }
                '<' => {
                    if a < b {
                        1.0
                    } else {
                        0.0
                    }
                }
                'G' => {
                    if a >= b {
                        1.0
                    } else {
                        0.0
                    }
                }
                'L' => {
                    if a <= b {
                        1.0
                    } else {
                        0.0
                    }
                }
                '&' => (a as u64 & b as u64) as f64,
                '|' => (a as u64 | b as u64) as f64,
                'x' => (a as u64 ^ b as u64) as f64,
                _ => a,
            };
            let mut new_dot = primary.dot();
            match self.kind {
                SetKind::Value => new_dot.set_value(new_val),
                SetKind::Addr => new_dot.set_addr(new_val as u64),
            };
            primary.replace_dot(new_dot);
            return Some(primary);
        }
        None
    }
}
