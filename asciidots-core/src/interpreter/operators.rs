use crate::{
    primitives::{Direction, Orientation},
    state_machine::DotMachine,
};
use std::{collections::VecDeque, ops::Rem};

/// A small queue for dots waiting at one side of an operator.
#[derive(Debug, Default)]
struct DotQueue {
    entries: VecDeque<DotMachine>,
}

impl DotQueue {
    fn enqueue(&mut self, dot: DotMachine) {
        self.entries.push_back(dot);
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn remove_matching(&mut self, pred: impl Fn(&DotMachine) -> bool) -> Option<DotMachine> {
        let idx = self.entries.iter().position(pred)?;
        self.entries.remove(idx)
    }

    fn iter(&self) -> impl Iterator<Item = &DotMachine> {
        self.entries.iter()
    }
}

#[derive(Debug)]
pub struct OperatorQueues {
    orientation: Orientation,
    op: char,
    negated: bool,
    horizontal: DotQueue,
    vertical: DotQueue,
}

impl OperatorQueues {
    pub(super) fn new(orientation: Orientation, op: char, negated: bool) -> Self {
        Self {
            orientation,
            op,
            negated,
            horizontal: DotQueue::default(),
            vertical: DotQueue::default(),
        }
    }

    fn same_call(dot1: &DotMachine, dot2: &DotMachine) -> bool {
        let call1 = dot1.dot().callstack();
        let call2 = dot2.dot().callstack();
        call1 == call2
    }

    pub(super) fn push_and_get_result(&mut self, dot: DotMachine) -> Option<DotMachine> {
        let incoming_orientation = dot.direction().orientation();
        let (same_queue, search_queue) = match incoming_orientation {
            Orientation::Horizontal => (&mut self.horizontal, &mut self.vertical),
            Orientation::Vertical => (&mut self.vertical, &mut self.horizontal),
        };

        if search_queue.is_empty() {
            same_queue.enqueue(dot);
            return None;
        }

        let other = search_queue.remove_matching(|other| Self::same_call(&dot, other));
        let Some(other) = other else {
            same_queue.enqueue(dot);
            return None;
        };

        let (mut primary, secondary) = if dot.direction().orientation() == self.orientation {
            (dot, other)
        } else {
            (other, dot)
        };

        if self.op == '~' {
            let mut new_dot = primary.dot().clone();
            let condition = {
                let tmp = secondary.dot().selected() as u64 == 0;
                if self.negated { !tmp } else { tmp }
            };
            if !condition {
                new_dot.set_dir(Direction::Up);
            }
            new_dot.reset_selector();
            primary.replace_dot(new_dot);
            return Some(primary);
        }

        let a = primary.dot().selected();
        let b = secondary.dot().selected();
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
        let mut new_dot = primary.dot().clone();
        new_dot.set_selected(new_val);
        primary.replace_dot(new_dot);
        Some(primary)
    }

    pub fn dots(&self) -> impl Iterator<Item = &DotMachine> {
        self.horizontal.iter().chain(self.vertical.iter())
    }
}
