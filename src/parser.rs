use crate::token::{Direction, MirrorKind, Orientation, SpanToken, Token};

#[derive(Clone, Copy, Debug)]
pub enum Severity {
    Error,
    Warn,
    Info,
    Hint,
}

#[derive(Clone, Copy, Debug)]
pub enum DiagnosticKind {
    Empty,
    MissingOpeningBrace,
    MissingClosingBrace,
    ConditionPathMissing,
}

#[derive(Clone, Copy, Debug)]
pub struct Diagnostic {
    x: usize,
    y: usize,
    kind: DiagnosticKind,
    severity: Severity,
}

impl Diagnostic {
    pub fn new(x: usize, y: usize, kind: DiagnosticKind, severity: Severity) -> Self {
        Self {
            x,
            y,
            kind,
            severity,
        }
    }
}

pub struct Lexer {
    grid: Vec<Vec<char>>,
    tokens: Vec<Vec<SpanToken>>,
    diagnostics: Vec<Diagnostic>,
}

impl Lexer {
    pub fn new(grid: Vec<Vec<char>>) -> Self {
        let tokens = grid
            .iter()
            .enumerate()
            .map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .map(|(x, character)| SpanToken::new(Token::Empty, x, y, *character))
                    .collect()
            })
            .collect();

        Self {
            grid,
            tokens,
            diagnostics: Vec::new(),
        }
    }

    fn get_operator(&self, x: usize, y: usize) -> Option<Orientation> {
        if let Some('[') = self
            .grid
            .get(y)
            .and_then(|row| row.get(x.overflowing_sub(1).0))
            && let Some(']') = self
                .grid
                .get(y)
                .and_then(|row| row.get(x.overflowing_add(1).0))
        {
            Some(Orientation::Vertical)
        } else if let Some('{') = self
            .grid
            .get(y)
            .and_then(|row| row.get(x.overflowing_sub(1).0))
            && let Some('}') = self
                .grid
                .get(y)
                .and_then(|row| row.get(x.overflowing_add(1).0))
        {
            Some(Orientation::Horizontal)
        } else {
            None
        }
    }

    fn lex_char(&self, character: char, x: usize, y: usize) -> Result<Token, Diagnostic> {
        match character {
            ' ' => Ok(Token::Empty),
            '.' => Ok(Token::Dot),
            '|' => Ok(Token::Pipe(Orientation::Vertical)),
            '-' => Ok(Token::Pipe(Orientation::Horizontal)),
            '/' => Ok(Token::Mirror(MirrorKind::Forward)),
            '\\' => Ok(Token::Mirror(MirrorKind::Backward)),
            '[' => match self.grid.get(y).and_then(|row| row.get(x + 2)) {
                Some(']') => Ok(Token::OperatorBegin),
                _ => Err(Diagnostic::new(
                    x,
                    y,
                    DiagnosticKind::MissingClosingBrace,
                    Severity::Error,
                )),
            },
            ']' => match self.grid.get(y).and_then(|row| row.get(x - 2)) {
                Some('[') => Ok(Token::OperatorEnd),
                _ => Err(Diagnostic::new(
                    x,
                    y,
                    DiagnosticKind::MissingOpeningBrace,
                    Severity::Error,
                )),
            },
            '{' => match self.grid.get(y).and_then(|row| row.get(x + 2)) {
                Some('}') => Ok(Token::OperatorBegin),
                _ => Err(Diagnostic::new(
                    x,
                    y,
                    DiagnosticKind::MissingClosingBrace,
                    Severity::Error,
                )),
            },
            '}' => match self.grid.get(y).and_then(|row| row.get(x - 2)) {
                Some('{') => Ok(Token::OperatorEnd),
                _ => Err(Diagnostic::new(
                    x,
                    y,
                    DiagnosticKind::MissingOpeningBrace,
                    Severity::Error,
                )),
            },
            '(' => Ok(Token::Reflector(MirrorKind::Forward)),
            ')' => Ok(Token::Reflector(MirrorKind::Backward)),
            '#' => Ok(Token::ValueMarker),
            '@' => Ok(Token::IdMarker),
            '?' => Ok(Token::Read),
            '$' => Ok(Token::Write),
            '~' => match self.grid.get(y + 1).and_then(|row| row.get(x)) {
                Some('!') => Ok(Token::Conditonal(true)),
                Some(_) => Ok(Token::Conditonal(false)),
                _ => Err(Diagnostic::new(
                    x,
                    y,
                    DiagnosticKind::ConditionPathMissing,
                    Severity::Warn,
                )),
            },
            '!' => Ok(Token::Negate),
            '"' => Ok(Token::Quote),
            ':' => Ok(Token::Filter(false)),
            ';' => Ok(Token::Filter(true)),
            c => {
                if let Some(orientation) = self.get_operator(x, y) {
                    Ok(Token::Operator {
                        orientation,
                        repr: c,
                    })
                } else {
                    match c {
                        '>' => Ok(Token::ForceDirection(Direction::Right)),
                        '<' => Ok(Token::ForceDirection(Direction::Left)),
                        'v' => Ok(Token::ForceDirection(Direction::Down)),
                        '^' => Ok(Token::ForceDirection(Direction::Up)),
                        '+' => Ok(Token::CrossPipe),
                        '&' => Ok(Token::StopProgram),
                        '*' => Ok(Token::Duplicate),
                        x if x.is_ascii_digit() => {
                            Ok(Token::Digit(x.to_string().parse().unwrap_or_default()))
                        }
                        x if x.is_ascii_alphanumeric() => Ok(Token::Char(x)),
                        _ => Ok(Token::Empty),
                    }
                }
            }
        }
    }

    pub fn run(&mut self) {
        for (y, row) in self.grid.iter().enumerate() {
            for (x, character) in row.iter().enumerate() {
                match self.lex_char(*character, x, y) {
                    Ok(t) => {
                        self.tokens[y][x] = SpanToken::new(t, x, y, *character);
                    }
                    Err(e) => {
                        self.diagnostics.push(e);
                    }
                }
            }
        }
    }

    pub fn finish(self) -> Result<Vec<Vec<SpanToken>>, Vec<Diagnostic>> {
        if self.diagnostics.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.diagnostics)
        }
    }
}
