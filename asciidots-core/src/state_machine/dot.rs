use std::cell::RefCell;

use crate::primitives::{Direction, Orientation, Position};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ValueSelector {
    Addr,
    Value,
}

#[derive(Debug, Clone, Copy)]
pub struct Dot {
    direction: Direction,
    position: Position,
    value: f64,
    addr: f64,
    uuid: u64,
    selector: ValueSelector,
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
            addr: 0.0,
            selector: ValueSelector::Value,
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

    pub fn set_addr(&mut self, new_id: f64) -> &mut Self {
        self.addr = new_id;
        self
    }

    pub fn set_selected(&mut self, value: f64) -> &mut Self {
        if self.selector == ValueSelector::Value {
            self.value = value;
        } else {
            self.addr = value;
        }
        self.selector = ValueSelector::Value;
        self
    }

    pub fn selected(&self) -> f64 {
        if self.selector == ValueSelector::Value {
            self.value
        } else {
            self.addr
        }
    }

    pub fn select(&mut self, value: ValueSelector) -> &mut Self {
        self.selector = value;
        self
    }

    pub fn reset_selector(&mut self) -> &mut Self {
        self.selector = ValueSelector::Value;
        self
    }

    pub fn orientation(&self) -> Orientation {
        self.direction.orientation()
    }

    pub fn uuid(&self) -> u64 {
        self.uuid
    }

    pub fn direction(&self) -> &Direction {
        &self.direction
    }

    pub fn addr(&self) -> f64 {
        self.addr
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn selector(&self) -> ValueSelector {
        self.selector
    }

    pub fn set_position(&mut self, other: Position) -> &mut Self {
        self.position = other;
        self
    }
}
