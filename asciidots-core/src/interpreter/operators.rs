use crate::{
    primitives::{Direction, Orientation},
    state_machine::DotMachine,
};
use rustc_hash::FxHashMap;
use std::{collections::VecDeque, ops::Rem};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct CallsiteKey {
    has_last: bool,
    last_x: i32,
    last_y: i32,
    depth: usize,
}

impl CallsiteKey {
    fn from_dot(dot: &DotMachine) -> Self {
        let callstack = dot.dot().callstack();
        if let Some(last) = callstack.last() {
            Self {
                has_last: true,
                last_x: last.x,
                last_y: last.y,
                depth: callstack.len(),
            }
        } else {
            Self {
                has_last: false,
                last_x: 0,
                last_y: 0,
                depth: 0,
            }
        }
    }
}

/// A small queue for dots waiting at one side of an operator.
#[derive(Debug, Default)]
struct DotQueue {
    entries: FxHashMap<CallsiteKey, VecDeque<DotMachine>>,
    total_len: usize,
}

impl DotQueue {
    fn enqueue(&mut self, dot: DotMachine) {
        let key = CallsiteKey::from_dot(&dot);
        self.entries.entry(key).or_default().push_back(dot);
        self.total_len += 1;
    }

    fn is_empty(&self) -> bool {
        self.total_len == 0
    }

    pub fn remove_matching_callstack(&mut self, dot: &DotMachine) -> Option<DotMachine> {
        let key = CallsiteKey::from_dot(dot);
        let mut should_remove_bucket = false;
        let removed = {
            let bucket = self.entries.get_mut(&key)?;
            let idx = bucket.iter().position(|other| OperatorQueues::same_call(dot, other))?;
            let removed = bucket.remove(idx);
            if bucket.is_empty() {
                should_remove_bucket = true;
            }
            removed
        };

        if should_remove_bucket {
            self.entries.remove(&key);
        }

        if removed.is_some() {
            self.total_len -= 1;
        }

        removed
    }

    fn iter(&self) -> impl Iterator<Item = &DotMachine> {
        self.entries.values().flat_map(VecDeque::iter)
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

        let other = search_queue.remove_matching_callstack(&dot);
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
            let condition = {
                let tmp = secondary.dot().selected() as u64 == 0;
                if self.negated { !tmp } else { tmp }
            };
            if !condition {
                primary.dot_mut().set_dir(Direction::Up);
            }
            primary.dot_mut().reset_selector();
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
        primary.dot_mut().set_selected(new_val);
        Some(primary)
    }

    pub fn dots(&self) -> impl Iterator<Item = &DotMachine> {
        self.horizontal.iter().chain(self.vertical.iter())
    }

    pub fn waiting_len(&self) -> usize {
        self.horizontal.total_len + self.vertical.total_len
    }
}
