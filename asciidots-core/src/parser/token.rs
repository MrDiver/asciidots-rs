use crate::primitives::{Direction, Orientation};

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
    Operator {
        orientation: Orientation,
        repr: char,
    }, // Any char + - / * and so on
    OperatorEnd(Orientation),   // Horizontal } and Vertical ]
    Reflector(MirrorKind),      // Forward `(` and Backward `)`
    ValueMarker,                // #
    IdMarker,                   // @
    Read,                       // ?
    Write,                      // $
    Conditonal(bool),           // ~
    Negate,                     // ! Does nothing
    Quote,                      // "
    QuoteStream,                // '
    Filter(bool),               // False : and True ;
    // - Same char could be used as operator -
    CrossPipe,                 // Standalone +
    StopProgram,               // Standalone &
    ForceDirection(Direction), // > < ^ v
    Duplicate,                 // Standalone *
    Digit(u8),                 // Any digit 1-9
    Char(char),                // Any Char
    Assertion, // Standalone =, Can be used for assertions which throw an error if not met, but otherwise do nothing.
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Empty => write!(f, " "),
            Token::Dot => write!(f, "."),
            Token::Pipe(Orientation::Horizontal) => write!(f, "-"),
            Token::Pipe(Orientation::Vertical) => write!(f, "|"),
            Token::Mirror(MirrorKind::Forward) => write!(f, "/"),
            Token::Mirror(MirrorKind::Backward) => write!(f, "\\"),
            Token::OperatorBegin(Orientation::Horizontal) => write!(f, "{{"),
            Token::OperatorBegin(Orientation::Vertical) => write!(f, "["),
            Token::Operator { repr, .. } => write!(f, "{}", repr),
            Token::OperatorEnd(Orientation::Horizontal) => write!(f, "}}"),
            Token::OperatorEnd(Orientation::Vertical) => write!(f, "]"),
            Token::Reflector(MirrorKind::Forward) => write!(f, "("),
            Token::Reflector(MirrorKind::Backward) => write!(f, ")"),
            Token::ValueMarker => write!(f, "#"),
            Token::IdMarker => write!(f, "@"),
            Token::Read => write!(f, "?"),
            Token::Write => write!(f, "$"),
            Token::Conditonal(_) => write!(f, "~"),
            Token::Negate => write!(f, "!"),
            Token::Quote => write!(f, "\""),
            Token::QuoteStream => write!(f, "'"),
            Token::Filter(b) => {
                if *b {
                    write!(f, ";")
                } else {
                    write!(f, ":")
                }
            }
            Token::CrossPipe => write!(f, "+"),
            Token::StopProgram => write!(f, "&"),
            Token::ForceDirection(Direction::Up) => write!(f, "^"),
            Token::ForceDirection(Direction::Down) => write!(f, "v"),
            Token::ForceDirection(Direction::Left) => write!(f, "<"),
            Token::ForceDirection(Direction::Right) => write!(f, ">"),
            Token::Duplicate => write!(f, "*"),
            Token::Digit(d) => write!(f, "{}", d),
            Token::Char(c) => write!(f, "{}", c),
            Token::Assertion => write!(f, "="),
        }
    }
}

impl From<Token> for char {
    fn from(value: Token) -> Self {
        match value {
            Token::Empty => ' ',
            Token::Dot => '.',
            Token::Pipe(Orientation::Horizontal) => '-',
            Token::Pipe(Orientation::Vertical) => '|',
            Token::Mirror(MirrorKind::Forward) => '/',
            Token::Mirror(MirrorKind::Backward) => '\\',
            Token::OperatorBegin(Orientation::Horizontal) => '{',
            Token::OperatorBegin(Orientation::Vertical) => '[',
            Token::Operator { repr, .. } => repr,
            Token::OperatorEnd(Orientation::Horizontal) => '}',
            Token::OperatorEnd(Orientation::Vertical) => ']',
            Token::Reflector(MirrorKind::Forward) => '(',
            Token::Reflector(MirrorKind::Backward) => ')',
            Token::ValueMarker => '#',
            Token::IdMarker => '@',
            Token::Read => '?',
            Token::Write => '$',
            Token::Conditonal(_) => '~',
            Token::Negate => '!',
            Token::Quote => '\"',
            Token::QuoteStream => '\'',
            Token::Filter(b) => {
                if b {
                    ';'
                } else {
                    ':'
                }
            }
            Token::CrossPipe => '+',
            Token::StopProgram => '&',
            Token::ForceDirection(Direction::Up) => '^',
            Token::ForceDirection(Direction::Down) => 'v',
            Token::ForceDirection(Direction::Left) => '<',
            Token::ForceDirection(Direction::Right) => '>',
            Token::Duplicate => '*',
            Token::Digit(d) => char::from_digit(d as u32, 10).unwrap_or('0'),
            Token::Char(c) => c,
            Token::Assertion => '=',
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Spanned<T> {
    col: usize,
    line: usize,
    inner: T,
    repr: char,
    /// Byte offset of this token in the original source string.
    offset: usize,
}

impl<T> Spanned<T> {
    pub fn new(t: T, col: usize, line: usize, c: char, offset: usize) -> Self {
        Self {
            col,
            line,
            inner: t,
            repr: c,
            offset,
        }
    }

    pub fn temporary(token: Token) -> Spanned<Token> {
        Spanned::<Token> {
            col: 0,
            line: 0,
            inner: token,
            repr: char::from(token),
            offset: 0,
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

    pub fn offset(&self) -> usize {
        self.offset
    }
}

pub type SpanToken = Spanned<Token>;
