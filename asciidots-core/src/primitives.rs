use std::ops::Add;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn orientation(&self) -> Orientation {
        match self {
            Direction::Up | Direction::Down => Orientation::Vertical,
            Direction::Left | Direction::Right => Orientation::Horizontal,
        }
    }

    pub fn reverse(&self) -> Direction {
        match *self {
            Self::Up => Self::Down,
            Self::Right => Self::Left,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
        }
    }

    pub fn orthogonal(&self, dir: &Direction) -> bool {
        self.orientation() != dir.orientation()
    }

    pub fn parallel(&self, dir: &Direction) -> bool {
        self.orientation() == dir.orientation()
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
#[derive(Default)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Position { x, y }
    }
}


impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}
impl Eq for Position {}

impl From<(i32, i32)> for Position {
    fn from(value: (i32, i32)) -> Self {
        Position::new(value.0, value.1)
    }
}

impl From<(usize, usize)> for Position {
    fn from(value: (usize, usize)) -> Self {
        Position::new(value.0 as i32, value.1 as i32)
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

impl From<Position> for (usize, usize) {
    fn from(value: Position) -> Self {
        (value.x as usize, value.y as usize)
    }
}

impl From<&Position> for (usize, usize) {
    fn from(value: &Position) -> Self {
        (value.x as usize, value.y as usize)
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
