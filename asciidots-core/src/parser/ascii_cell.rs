use miette::SourceSpan;
use std::sync::Arc;

use crate::primitives::{Direction, Orientation, Position};

#[derive(Clone, Debug)]
pub struct AsciiFunc {
    name: String,
    original_name: Option<String>,
    position: (usize, usize),
}

impl AsciiFunc {
    pub fn new(name: String, position: (usize, usize)) -> Self {
        Self {
            name,
            original_name: None,
            position,
        }
    }

    pub fn with_original_name(
        name: String,
        original_name: Option<String>,
        position: (usize, usize),
    ) -> Self {
        Self {
            name,
            original_name,
            position,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn original_name(&self) -> Option<&str> {
        self.original_name.as_deref()
    }

    pub fn is_aliased(&self) -> bool {
        self.original_name.is_some()
    }

    pub fn position(&self) -> &(usize, usize) {
        &self.position
    }

    pub fn unpack(self) -> (String, Position) {
        (self.name, self.position.into())
    }

    pub fn unpack_with_original(self) -> (String, Option<String>, Position) {
        (self.name, self.original_name, self.position.into())
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum MirrorKind {
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
pub enum AsciiCell {
    Empty,
    Dot,                   // .
    Pipe(Orientation),     // - and |
    Mirror(MirrorKind),    // Forward / and Backward \
    Reflector(MirrorKind), // Forward `(` and Backward `)`
    ValueMarker,           // #
    AddrMarker,            // @
    Read,                  // ?
    Write,                 // $
    Conditional(bool),     // ~
    Negate,                // ! Does nothing
    Quote,                 // "
    QuoteStream,           // '
    Filter(bool),          // False : and True ;
    // - Same char could be used as operator -
    CrossPipe,                 // Standalone +
    StopProgram,               // Standalone &
    ForceDirection(Direction), // > < ^ v
    Duplicate,                 // Standalone *
    Digit(u8),                 // Any digit 1-9
    Char(char),                // Any Char
    Comment(char),
    Warp {
        repr: char,
        other: Position,
    },
    OperatorBegin(Orientation), // Horizontal { and Vertical [
    Operator {
        orientation: Orientation,
        repr: char,
    }, // Any char + - / * and so on
    OperatorEnd(Orientation),   // Horizontal } and Vertical ]
    Call(Arc<str>),
    CallById(u32),
    Display(u32),
    DefRet(String),
    Assertion, // Standalone =, Can be used for assertions which throw an error if not met, but otherwise do nothing.
}

impl std::fmt::Display for AsciiCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsciiCell::Empty => write!(f, " "),
            AsciiCell::Dot => write!(f, "."),
            AsciiCell::Pipe(Orientation::Horizontal) => write!(f, "-"),
            AsciiCell::Pipe(Orientation::Vertical) => write!(f, "|"),
            AsciiCell::Mirror(MirrorKind::Forward) => write!(f, "/"),
            AsciiCell::Mirror(MirrorKind::Backward) => write!(f, "\\"),
            AsciiCell::OperatorBegin(Orientation::Horizontal) => write!(f, "{{"),
            AsciiCell::OperatorBegin(Orientation::Vertical) => write!(f, "["),
            AsciiCell::Operator { repr, .. } => write!(f, "{}", repr),
            AsciiCell::OperatorEnd(Orientation::Horizontal) => write!(f, "}}"),
            AsciiCell::OperatorEnd(Orientation::Vertical) => write!(f, "]"),
            AsciiCell::Reflector(MirrorKind::Forward) => write!(f, "("),
            AsciiCell::Reflector(MirrorKind::Backward) => write!(f, ")"),
            AsciiCell::ValueMarker => write!(f, "#"),
            AsciiCell::AddrMarker => write!(f, "@"),
            AsciiCell::Read => write!(f, "?"),
            AsciiCell::Write => write!(f, "$"),
            AsciiCell::Conditional(_) => write!(f, "~"),
            AsciiCell::Negate => write!(f, "!"),
            AsciiCell::Quote => write!(f, "\""),
            AsciiCell::QuoteStream => write!(f, "'"),
            AsciiCell::Filter(b) => {
                if *b {
                    write!(f, ";")
                } else {
                    write!(f, ":")
                }
            }
            AsciiCell::CrossPipe => write!(f, "+"),
            AsciiCell::StopProgram => write!(f, "&"),
            AsciiCell::ForceDirection(Direction::Up) => write!(f, "^"),
            AsciiCell::ForceDirection(Direction::Down) => write!(f, "v"),
            AsciiCell::ForceDirection(Direction::Left) => write!(f, "<"),
            AsciiCell::ForceDirection(Direction::Right) => write!(f, ">"),
            AsciiCell::Duplicate => write!(f, "*"),
            AsciiCell::Digit(d) => write!(f, "{}", d),
            AsciiCell::Char(c) => write!(f, "{}", c),
            AsciiCell::Comment(c) => write!(f, "{}", c),
            AsciiCell::Assertion => write!(f, "="),
            AsciiCell::Warp { repr, .. } => write!(f, "{}", repr),
            AsciiCell::Call(_) => write!(f, "%"),
            AsciiCell::CallById(_) => write!(f, "%"),
            AsciiCell::Display(_) => write!(f, "%"),
            AsciiCell::DefRet(_) => write!(f, "@"),
        }
    }
}

impl From<&AsciiCell> for char {
    fn from(value: &AsciiCell) -> Self {
        match value {
            AsciiCell::Empty => ' ',
            AsciiCell::Dot => '.',
            AsciiCell::Pipe(Orientation::Horizontal) => '-',
            AsciiCell::Pipe(Orientation::Vertical) => '|',
            AsciiCell::Mirror(MirrorKind::Forward) => '/',
            AsciiCell::Mirror(MirrorKind::Backward) => '\\',
            AsciiCell::OperatorBegin(Orientation::Horizontal) => '{',
            AsciiCell::OperatorBegin(Orientation::Vertical) => '[',
            AsciiCell::Operator { repr, .. } => *repr,
            AsciiCell::OperatorEnd(Orientation::Horizontal) => '}',
            AsciiCell::OperatorEnd(Orientation::Vertical) => ']',
            AsciiCell::Reflector(MirrorKind::Forward) => '(',
            AsciiCell::Reflector(MirrorKind::Backward) => ')',
            AsciiCell::ValueMarker => '#',
            AsciiCell::AddrMarker => '@',
            AsciiCell::Read => '?',
            AsciiCell::Write => '$',
            AsciiCell::Conditional(_) => '~',
            AsciiCell::Negate => '!',
            AsciiCell::Quote => '\"',
            AsciiCell::QuoteStream => '\'',
            AsciiCell::Filter(b) => {
                if *b {
                    ';'
                } else {
                    ':'
                }
            }
            AsciiCell::CrossPipe => '+',
            AsciiCell::StopProgram => '&',
            AsciiCell::ForceDirection(Direction::Up) => '^',
            AsciiCell::ForceDirection(Direction::Down) => 'v',
            AsciiCell::ForceDirection(Direction::Left) => '<',
            AsciiCell::ForceDirection(Direction::Right) => '>',
            AsciiCell::Duplicate => '*',
            AsciiCell::Digit(d) => char::from_digit(*d as u32, 10).unwrap_or('0'),
            AsciiCell::Char(c) => *c,
            AsciiCell::Comment(c) => *c,
            AsciiCell::Assertion => '=',
            AsciiCell::Warp { repr, .. } => *repr,
            AsciiCell::Call(_) => '%',
            AsciiCell::CallById(_) => '%',
            AsciiCell::Display(_) => '%',
            AsciiCell::DefRet(_) => '@',
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Spanned<T> {
    inner: T,
    repr: char,
    /// Byte offset of this token in the original source string.
    span: SourceSpan,
}

impl<T> Spanned<T> {
    pub fn new(t: T, c: char, span: SourceSpan) -> Self {
        Self {
            inner: t,
            repr: c,
            span,
        }
    }

    pub fn temporary(token: AsciiCell) -> Spanned<AsciiCell> {
        Spanned::<AsciiCell> {
            repr: char::from(&token),
            inner: token,
            span: (0, 1).into(),
        }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn repr(&self) -> char {
        self.repr
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

pub type SpanToken = Spanned<AsciiCell>;
