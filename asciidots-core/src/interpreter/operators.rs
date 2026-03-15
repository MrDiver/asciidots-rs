use crate::{primitives::Orientation, state_machine::DotMachine};
use std::collections::VecDeque;

#[derive(Debug)]
pub struct OperatorQueues {
    orientation: Orientation,
    op: char,
    horizontal: VecDeque<DotMachine>,
    vertical: VecDeque<DotMachine>,
}

impl OperatorQueues {
    pub(super) fn new(orientation: Orientation, op: char) -> Self {
        Self {
            orientation,
            op,
            horizontal: VecDeque::with_capacity(1),
            vertical: VecDeque::with_capacity(1),
        }
    }

    pub(super) fn push(&mut self, dot: DotMachine) {
        match dot.direction().orientation() {
            Orientation::Horizontal => self.horizontal.push_back(dot),
            Orientation::Vertical => self.vertical.push_back(dot),
        }
    }

    pub(super) fn get_result(&mut self) -> Option<DotMachine> {
        if self.horizontal.len() > 0 && self.vertical.len() > 0 {
            let (mut primary, secondary) = {
                let doth = self.horizontal.pop_front().unwrap();
                let dotv = self.vertical.pop_front().unwrap();
                if matches!(self.orientation, Orientation::Horizontal) {
                    (doth, dotv)
                } else {
                    (dotv, doth)
                }
            };

            let a = primary.value();
            let b = secondary.value();
            let new_val = match self.op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                _ => a,
            };
            let mut new_dot = primary.dot();
            new_dot.set_value(new_val);
            primary.replace_dot(new_dot);
            return Some(primary);
        }
        None
    }
}
