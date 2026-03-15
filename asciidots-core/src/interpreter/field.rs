use crate::{
    parser::{LexError, Lexer, SpanToken, Token},
    primitives::Position,
};

pub type TokenField = Vec<Vec<SpanToken>>;

#[derive(Debug)]
pub struct AsciiField {
    field: TokenField,
    offsets: Vec<Vec<usize>>,
    width: usize,
    height: usize,
}

impl AsciiField {
    pub fn from_string(content: &str) -> Result<Self, Vec<LexError>> {
        let mut lexer = Lexer::new(content.into());
        lexer.run();
        let offsets = lexer.offsets();
        let field = lexer.finish()?;
        let height = field.len();
        let width = field
            .iter()
            .map(|line| line.len())
            .max()
            .unwrap_or_default();
        Ok(AsciiField {
            field,
            offsets,
            height,
            width,
        })
    }

    pub fn get_field(&self) -> &TokenField {
        &self.field
    }

    pub fn at(&self, position: &Position) -> Token {
        let &Position { x, y } = position;
        if x < 0 || y < 0 {
            return Token::Empty;
        }
        self.field
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .map(|token| *token.inner())
            .unwrap_or_else(|| Token::Empty)
    }

    pub fn offset(&self, position: &Position) -> usize {
        self.offsets[position.y as usize][position.x as usize]
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}
