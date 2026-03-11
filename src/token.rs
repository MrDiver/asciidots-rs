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

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum MirrorKind {
    Forward,
    Backward,
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Token {
    Empty,
    Dot,                        // .
    Pipe(Orientation),          // - and |
    Mirror(MirrorKind),         // Forward / and Backward \
    OperatorBegin(Orientation), // Horizontal { and Vertical [
    Operator(char),             // Any char + - / * and so on
    OperatorEnd(Orientation),   // Horizontal } and Vertical ]
    Reflector(MirrorKind),      // Forward `(` and Backward `)`
    ValueMarker,                // #
    IdMarker,                   // @
    Read,                       // ?
    Write,                      // $
    Conditonal(bool),           // ~
    Negate,                     // ! Does nothing
    Quote,                      // "
    Filter(bool),               // False : and True ;
    // - Same char could be used as operator -
    CrossPipe,                 // Standalone +
    StopProgram,               // Standalone &
    ForceDirection(Direction), // > < ^ v
    Duplicate,                 // Standalone *
    Digit(u8),                 // Any digit 1-9
    Char(char),                // Any Char
}

#[derive(Debug, Copy, Clone)]
pub struct Spanned<T> {
    col: usize,
    line: usize,
    inner: T,
    repr: char,
}

impl<T> Spanned<T> {
    pub fn new(t: T, col: usize, line: usize, c: char) -> Self {
        Self {
            col,
            line,
            inner: t,
            repr: c,
        }
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn repr(&self) -> char {
        self.repr
    }
}

pub type SpanToken = Spanned<Token>;
