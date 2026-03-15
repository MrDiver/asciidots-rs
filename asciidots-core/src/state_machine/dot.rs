use crate::primitives::{Direction, Orientation, Position};

#[derive(Debug, Clone, Copy)]
pub struct Dot {
    pub direction: Direction,
    pub position: Position,
    pub value: f64,
    pub id: u64,
}

impl Dot {
    pub fn new(direction: Direction, position: Position, value: f64) -> Self {
        Dot {
            direction,
            position,
            value,
            id: 0,
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

    pub fn set_id(&mut self, new_id: u64) -> &mut Self {
        self.id = new_id;
        self
    }

    pub fn orientation(&self) -> Orientation {
        self.direction.orientation()
    }
}
