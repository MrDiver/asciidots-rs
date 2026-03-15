use super::error::LexError;
use super::token::{MirrorKind, SpanToken, Token};
use crate::primitives::{Direction, Orientation};
use miette::SourceSpan;

pub struct Lexer {
    grid: Vec<Vec<char>>,
    tokens: Vec<Vec<SpanToken>>,
    offsets: Vec<Vec<usize>>,
    diagnostics: Vec<LexError>,
}

impl From<&str> for Lexer {
    fn from(value: &str) -> Self {
        Lexer::new(value)
    }
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        let mut offset = 0;
        let mut offsets: Vec<Vec<usize>> = Vec::new();
        let grid: Vec<Vec<char>> = source
            .lines()
            .map(|line| {
                offsets.push(line.char_indices().map(|(i, _)| offset + i).collect());
                offset += line.len() + 1;
                line.chars().collect()
            })
            .collect();
        let tokens = grid
            .iter()
            .enumerate()
            .map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .map(|(x, character)| SpanToken::new(Token::Empty, x, y, *character, offsets[y][x]))
                    .collect()
            })
            .collect();

        Self {
            grid,
            tokens,
            offsets,
            diagnostics: Vec::new(),
        }
    }

    fn span(&self, x: usize, y: usize, len: usize) -> SourceSpan {
        let offset = self.offsets[y][x];
        (offset, len).into()
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

    fn lex_char(&self, character: char, x: usize, y: usize) -> Result<Token, LexError> {
        if !matches!(character, '[' | ']' | '{' | '}')
            && let Some(orientation) = self.get_operator(x, y)
        {
            return Ok(Token::Operator {
                orientation,
                repr: character,
            });
        }

        match character {
            ' ' => Ok(Token::Empty),
            '.' => Ok(Token::Dot),
            '|' => Ok(Token::Pipe(Orientation::Vertical)),
            '-' => Ok(Token::Pipe(Orientation::Horizontal)),
            '/' => Ok(Token::Mirror(MirrorKind::Forward)),
            '\\' => Ok(Token::Mirror(MirrorKind::Backward)),
            '[' => match self.grid.get(y).and_then(|row| row.get(x + 2)) {
                Some(']') => Ok(Token::OperatorBegin(Orientation::Vertical)),
                c => Err(LexError::missing_closing_brace(
                    *c.unwrap_or(&' '),
                    self.span(x + 2, y, 1),
                    self.span(x, y, 2),
                    Orientation::Vertical,
                )),
            },
            ']' => match self.grid.get(y).and_then(|row| row.get(x - 2)) {
                Some('[') => Ok(Token::OperatorEnd(Orientation::Vertical)),
                c => Err(LexError::missing_opening_brace(
                    *c.unwrap_or(&' '),
                    self.span(x - 2, y, 1),
                    self.span(x - 1, y, 2),
                    Orientation::Vertical,
                )),
            },
            '{' => match self.grid.get(y).and_then(|row| row.get(x + 2)) {
                Some('}') => Ok(Token::OperatorBegin(Orientation::Horizontal)),
                c => Err(LexError::missing_closing_brace(
                    *c.unwrap_or(&' '),
                    self.span(x + 2, y, 1),
                    self.span(x, y, 2),
                    Orientation::Horizontal,
                )),
            },
            '}' => match self.grid.get(y).and_then(|row| row.get(x - 2)) {
                Some('{') => Ok(Token::OperatorEnd(Orientation::Horizontal)),
                c => Err(LexError::missing_opening_brace(
                    *c.unwrap_or(&' '),
                    self.span(x - 2, y, 1),
                    self.span(x - 1, y, 2),
                    Orientation::Horizontal,
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
                _ => Err(LexError::missing_conditional_path(self.span(x, y + 1, 1))),
            },
            '!' => Ok(Token::Negate),
            '"' => Ok(Token::Quote),
            '\'' => Ok(Token::QuoteStream),
            ':' => Ok(Token::Filter(false)),
            ';' => Ok(Token::Filter(true)),
            c => match c {
                '>' => Ok(Token::ForceDirection(Direction::Right)),
                '<' => Ok(Token::ForceDirection(Direction::Left)),
                'v' => Ok(Token::ForceDirection(Direction::Down)),
                '^' => Ok(Token::ForceDirection(Direction::Up)),
                '+' => Ok(Token::CrossPipe),
                '&' => Ok(Token::StopProgram),
                '*' => Ok(Token::Duplicate),
                x if x.is_ascii_digit() => Ok(Token::Digit((x as u8) - b'0')),
                x if x.is_ascii_alphanumeric() => Ok(Token::Char(x)),
                _ => Ok(Token::Empty),
            },
        }
    }

    pub fn run(&mut self) {
        for (y, row) in self.grid.iter().enumerate() {
            for (x, character) in row.iter().enumerate() {
                match self.lex_char(*character, x, y) {
                    Ok(t) => {
                        self.tokens[y][x] = SpanToken::new(t, x, y, *character, self.offsets[y][x]);
                    }
                    Err(e) => {
                        self.diagnostics.push(e);
                    }
                }
            }
        }
    }

    pub fn finish(self) -> Result<Vec<Vec<SpanToken>>, Vec<LexError>> {
        if self.diagnostics.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.diagnostics)
        }
    }
}
