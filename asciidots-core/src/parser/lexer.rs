use super::error::LexError;
use super::token::{MirrorKind, SpanToken, Token};
use crate::primitives::{Direction, Orientation};
use crate::state_machine::SetKind;
use miette::SourceSpan;

pub struct Lexer {
    grid: Vec<Vec<char>>,
    tokens: Vec<Vec<SpanToken>>,
    offsets: Vec<Vec<usize>>,
    source_len: usize,
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
                    .map(|(x, character)| {
                        SpanToken::new(Token::Empty, x, y, *character, offsets[y][x])
                    })
                    .collect()
            })
            .collect();

        Self {
            grid,
            tokens,
            offsets,
            source_len: source.len(),
            diagnostics: Vec::new(),
        }
    }

    fn span(&self, x: usize, y: usize, len: usize) -> SourceSpan {
        let fallback = self.source_len.saturating_sub(1);
        let offset = if let Some(row_offsets) = self.offsets.get(y) {
            if let Some(offset) = row_offsets.get(x).copied() {
                offset
            } else if let (Some(last_offset), Some(last_char)) = (
                row_offsets.last().copied(),
                self.grid.get(y).and_then(|row| row.last()).copied(),
            ) {
                last_offset + last_char.len_utf8()
            } else {
                fallback
            }
        } else {
            fallback
        };
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

    fn at(&self, x: usize, y: usize) -> Option<&char> {
        self.grid.get(y).and_then(|row| row.get(x))
    }

    fn get_kind(&self, x: usize, y: usize) -> SetKind {
        match self.at(x, y) {
            Some('@') => SetKind::Addr,
            _ => SetKind::Value,
        }
    }

    fn lex_char(&self, character: char, x: usize, y: usize) -> Result<Token, LexError> {
        if !matches!(character, '[' | ']' | '{' | '}')
            && let Some(orientation) = self.get_operator(x, y)
        {
            let kind = x
                .checked_sub(1)
                .map(|x| self.get_kind(x, y))
                .unwrap_or(SetKind::Value);
            return Ok(Token::Operator {
                orientation,
                repr: character,
                kind,
            });
        }

        match character {
            ' ' => Ok(Token::Empty),
            '.' => Ok(Token::Dot),
            '|' => Ok(Token::Pipe(Orientation::Vertical)),
            '-' => Ok(Token::Pipe(Orientation::Horizontal)),
            '/' => Ok(Token::Mirror(MirrorKind::Forward)),
            '\\' => Ok(Token::Mirror(MirrorKind::Backward)),
            '[' => match self.at(x + 2, y) {
                Some(']') => Ok(Token::OperatorBegin(Orientation::Vertical)),
                c => Err(LexError::missing_closing_brace(
                    *c.unwrap_or(&' '),
                    self.span(x + 2, y, 1),
                    self.span(x, y, 2),
                    Orientation::Vertical,
                )),
            },
            ']' => match x.checked_sub(2).and_then(|idx| self.at(idx, y)) {
                Some('[') => Ok(Token::OperatorEnd(Orientation::Vertical)),
                c => Err(LexError::missing_opening_brace(
                    *c.unwrap_or(&' '),
                    self.span(x.saturating_sub(2), y, 1),
                    self.span(x.saturating_sub(1), y, 2),
                    Orientation::Vertical,
                )),
            },
            '{' => match self.at(x + 2, y) {
                Some('}') => Ok(Token::OperatorBegin(Orientation::Horizontal)),
                c => Err(LexError::missing_closing_brace(
                    *c.unwrap_or(&' '),
                    self.span(x + 2, y, 1),
                    self.span(x, y, 2),
                    Orientation::Horizontal,
                )),
            },
            '}' => match x.checked_sub(2).and_then(|idx| self.at(idx, y)) {
                Some('{') => Ok(Token::OperatorEnd(Orientation::Horizontal)),
                c => Err(LexError::missing_opening_brace(
                    *c.unwrap_or(&' '),
                    self.span(x.saturating_sub(2), y, 1),
                    self.span(x.saturating_sub(1), y, 2),
                    Orientation::Horizontal,
                )),
            },
            '(' => Ok(Token::Reflector(MirrorKind::Forward)),
            ')' => Ok(Token::Reflector(MirrorKind::Backward)),
            '#' => Ok(Token::ValueMarker),
            '@' => Ok(Token::AddrMarker),
            '?' => Ok(Token::Read),
            '$' => Ok(Token::Write),
            '~' => {
                let kind = x
                    .checked_sub(1)
                    .map(|x| self.get_kind(x, y))
                    .unwrap_or(SetKind::Value);
                match self.at(x, y + 1) {
                    Some('!') => Ok(Token::Conditional(true, kind)),
                    Some(_) => Ok(Token::Conditional(false, kind)),
                    _ => Err(LexError::missing_conditional_path(self.span(x, y, 1))),
                }
            }
            '!' => Ok(Token::Negate),
            '"' => Ok(Token::Quote),
            '\'' => Ok(Token::QuoteStream),
            ':' => Ok(Token::Filter(false)),
            ';' => Ok(Token::Filter(true)),
            '>' => Ok(Token::ForceDirection(Direction::Right)),
            '<' => Ok(Token::ForceDirection(Direction::Left)),
            'v' => Ok(Token::ForceDirection(Direction::Down)),
            '^' => Ok(Token::ForceDirection(Direction::Up)),
            '+' => Ok(Token::CrossPipe),
            '&' => Ok(Token::StopProgram),
            '*' => Ok(Token::Duplicate),
            '=' => Ok(Token::Assertion),
            x if x.is_ascii_digit() => Ok(Token::Digit((x as u8) - b'0')),
            x if x.is_ascii_alphanumeric() => Ok(Token::Char(x)),
            _ => Ok(Token::Empty),
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
