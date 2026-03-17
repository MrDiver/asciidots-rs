use std::cell::RefCell;

use crate::primitives::{Direction, Orientation, Position};

#[derive(Debug, Clone, Copy)]
pub struct Dot {
    pub direction: Direction,
    pub position: Position,
    pub value: f64,
    pub addr: u64,
    pub uuid: u64,
}

thread_local! {
    static DOT_COUNTER: RefCell<u64> = RefCell::new(0);
}

fn next_dot_id() -> u64 {
    DOT_COUNTER.with(|x| {
        let mut x = x.borrow_mut();
        *x += 1;
        *x
    })
}

impl Dot {
    pub fn new(direction: Direction, position: Position, value: f64) -> Self {
        Dot {
            uuid: next_dot_id(),
            direction,
            position,
            value,
            addr: 0,
        }
    }

    pub fn step(&mut self) -> &mut Self {
        self.position = self.position + self.direction.into();
        self
    }

    pub fn set_dir(&mut self, new_dir: Direction) -> &mut Self {
        self.direction = new_dir;
        self
    }

    pub fn set_value(&mut self, new_value: f64) -> &mut Self {
        self.value = new_value;
        self
    }

    pub fn set_addr(&mut self, new_id: u64) -> &mut Self {
        self.addr = new_id;
        self
    }

    pub fn orientation(&self) -> Orientation {
        self.direction.orientation()
    }

    pub(crate) fn direction(&self) -> Direction {
        self.direction
    }
}
