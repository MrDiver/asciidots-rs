use std::ops::Add;

use crate::token::{Direction, Orientation};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DotState {
    Travel,
    Dead,
    EndProgram,
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Position { x, y }
    }
}

impl From<Direction> for Position {
    fn from(dir: Direction) -> Self {
        match dir {
            Direction::Up => Self { x: 0, y: -1 },
            Direction::Down => Self { x: 0, y: 1 },
            Direction::Left => Self { x: -1, y: 0 },
            Direction::Right => Self { x: 1, y: 0 },
        }
    }
}

impl From<Position> for Option<Direction> {
    fn from(pos: Position) -> Option<Direction> {
        match pos {
            Position { x: 0, y } => {
                if y > 0 {
                    Some(Direction::Down)
                } else if y < 0 {
                    Some(Direction::Up)
                } else {
                    None
                }
            }
            Position { x, y: 0 } => {
                if x > 0 {
                    Some(Direction::Right)
                } else if x < 0 {
                    Some(Direction::Left)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Add for Position {
    type Output = Position;

    fn add(self, other: Position) -> Position {
        Position {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Dot {
    pub direction: Direction,
    pub position: Position,
    pub value: f64,
    pub state: DotState,
}

impl Dot {
    pub fn new(direction: Direction, position: Position, value: f64) -> Self {
        Dot {
            direction,
            position,
            value,
            state: DotState::Travel,
        }
    }

    pub fn step(&mut self) -> &mut Self {
        if self.state == DotState::Dead {
            return self;
        }
        self.position = self.position + self.direction.into();
        self
    }

    pub fn change_dir(&mut self, new_dir: Direction) -> &mut Self {
        self.direction = new_dir;
        self
    }

    pub fn change_value(&mut self, new_value: f64) -> &mut Self {
        self.value = new_value;
        self
    }

    pub fn change_state(&mut self, new_state: DotState) -> &mut Self {
        self.state = new_state;
        self
    }

    pub fn orientation(&self) -> Orientation {
        self.direction.orientation()
    }
}
