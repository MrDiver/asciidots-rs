use crate::token::{Direction, MirrorKind, Orientation, SpanToken, Token};
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic, Clone)]
pub enum LexError {
    #[error("missing opening brace for operator found '{ch_expected}' instead ")]
    #[diagnostic(help("operators need braces: try '{expected_full}'"))]
    MissingOpeningBrace {
        ch_expected: char,
        ch_found: char,
        expected_full: String,
        #[label("Expected '{ch_expected}' here")]
        at_span: SourceSpan,
        #[label("operator")]
        op_span: SourceSpan,
    },
    #[error("missing closing brace for operator found '{ch_expected}' instead ")]
    #[diagnostic(help("operators need braces: try '{expected_full}'"))]
    MissingClosingBrace {
        ch_expected: char,
        ch_found: char,
        expected_full: String,
        #[label("Expected '{ch_expected}' here")]
        at_span: SourceSpan,
        #[label("operator")]
        op_span: SourceSpan,
    },
    #[error("missing condition path (guaranteed deadlock)")]
    #[diagnostic(help(""))]
    MissingConditionalPath {
        #[label("There needs to be a path here")]
        at_span: SourceSpan,
    },
}

impl LexError {
    pub fn missing_opening_brace(
        found: char,
        at_span: SourceSpan,
        op_span: SourceSpan,
        orientation: Orientation,
    ) -> Self {
        LexError::MissingOpeningBrace {
            ch_expected: Token::OperatorBegin(orientation).into(),
            ch_found: found,
            expected_full: format!(
                "{}?{}",
                char::from(Token::OperatorBegin(orientation)),
                char::from(Token::OperatorEnd(orientation))
            ),
            at_span,
            op_span,
        }
    }

    pub fn missing_closing_brace(
        found: char,
        at_span: SourceSpan,
        op_span: SourceSpan,
        orientation: Orientation,
    ) -> Self {
        LexError::MissingClosingBrace {
            ch_expected: Token::OperatorEnd(orientation).into(),
            ch_found: found,
            expected_full: format!(
                "{}?{}",
                char::from(Token::OperatorBegin(orientation)),
                char::from(Token::OperatorEnd(orientation))
            ),
            at_span,
            op_span,
        }
    }

    pub fn missing_conditional_path(at_span: SourceSpan) -> Self {
        LexError::MissingConditionalPath { at_span }
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("{} problem(s) found", errors.len())]
pub struct LexErrors {
    #[source_code]
    pub src: miette::NamedSource<String>,
    #[related]
    pub errors: Vec<LexError>,
}

pub struct Lexer {
    source: String,
    grid: Vec<Vec<char>>,
    tokens: Vec<Vec<SpanToken>>,
    offsets: Vec<Vec<usize>>,
    diagnostics: Vec<LexError>,
}

impl From<String> for Lexer {
    fn from(value: String) -> Self {
        Lexer::new(value)
    }
}

impl Lexer {
    pub fn new(source: String) -> Self {
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
                    .map(|(x, character)| SpanToken::new(Token::Empty, x, y, *character))
                    .collect()
            })
            .collect();

        Self {
            source,
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

    pub fn finish(self) -> Result<Vec<Vec<SpanToken>>, LexErrors> {
        if self.diagnostics.is_empty() {
            Ok(self.tokens)
        } else {
            Err(LexErrors {
                src: NamedSource::new("", self.source),
                errors: self.diagnostics,
            })
        }
    }

    pub(crate) fn offsets(&self) -> Vec<Vec<usize>> {
        self.offsets.clone()
    }
}
