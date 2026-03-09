use crate::state::{self, Direction, DotState, Position};

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
        self.position = self.position + self.direction.offset();
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
}
