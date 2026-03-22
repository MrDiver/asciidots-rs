use crate::{
    primitives::{Direction, Orientation},
    state_machine::DotMachine,
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

    fn peek(&self) -> &Option<DotMachine> {
        &self.first
    }

    pub fn remove_matching(&mut self, pred: impl Fn(&DotMachine) -> bool) -> Option<DotMachine> {
        if let Some(ref dot) = self.first {
            if pred(dot) {
                let dot = self.first.take().unwrap();
                self.first = self.overflow.pop_front();
                return Some(dot);
            }
        }

        let idx = self.overflow.iter().position(|d| pred(d))?;
        self.overflow.remove(idx)
    }

    pub fn has_matching(&self, pred: impl Fn(&DotMachine) -> bool) -> bool {
        self.first.as_ref().map_or(false, |d| pred(d)) || self.overflow.iter().any(pred)
    }
}

#[derive(Debug)]
pub struct OperatorQueues {
    orientation: Orientation,
    op: char,
    negated: bool,
    horizontal: DotQueue,
    vertical: DotQueue,
    last_push: Orientation,
}

impl OperatorQueues {
    pub(super) fn new(orientation: Orientation, op: char, negated: bool) -> Self {
        Self {
            orientation,
            op,
            negated,
            horizontal: DotQueue::default(),
            vertical: DotQueue::default(),
            last_push: Orientation::Vertical,
        }
    }

    pub(super) fn push(&mut self, dot: DotMachine) {
        let orient = dot.direction().orientation();
        match orient {
            Orientation::Horizontal => self.horizontal.push(dot),
            Orientation::Vertical => self.vertical.push(dot),
        }
        self.last_push = orient;
    }

    fn try_fire(&mut self) -> Option<(DotMachine, DotMachine)> {
        let (peek_queue, search_queue) = match self.last_push {
            Orientation::Horizontal => (&mut self.horizontal, &mut self.vertical),
            Orientation::Vertical => (&mut self.vertical, &mut self.horizontal),
        };
        if let Some(first) = peek_queue.peek() {
            let other = search_queue.remove_matching(|other| Self::same_call(&first, other))?;
            let first = peek_queue.pop_front().expect("Already peeked before");
            if first.direction().orientation() == self.orientation {
                Some((first, other))
            } else {
                Some((other, first))
            }
        } else {
            None
        }
    }

    fn same_call(dot1: &DotMachine, dot2: &DotMachine) -> bool {
        let call1 = dot1.dot().callstack();
        let call2 = dot2.dot().callstack();
        call1 == call2
    }

    pub(super) fn get_result(&mut self) -> Option<DotMachine> {
        if !self.horizontal.is_empty() && !self.vertical.is_empty() {
            let (mut primary, secondary) = self.try_fire()?;

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
            return Some(primary);
        }
        None
    }
}
