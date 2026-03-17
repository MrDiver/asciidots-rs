use crate::{
    parser::{LexError, Lexer, SpanToken, Token},
    primitives::Position,
};

/// A flat, rectangular token grid. Rows shorter than `width` are padded with
/// [`Token::Empty`] so the layout is always `width × height` cells stored in
/// row-major order. One contiguous allocation improves cache locality for the
/// interpreter's hot `at()` call compared to `Vec<Vec<SpanToken>>`.
pub type TokenField = Vec<SpanToken>;

#[derive(Debug)]
pub struct AsciiField {
    /// Token at `(x, y)` lives at index `y * width + x`.
    field: TokenField,
    width: usize,
    height: usize,
}

impl AsciiField {
    pub fn from_string(content: &str) -> Result<Self, Vec<LexError>> {
        let mut lexer = Lexer::new(content);
        lexer.run();
        let rows: Vec<Vec<SpanToken>> = lexer.finish()?;
        let height = rows.len();
        let width = rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut field = Vec::with_capacity(width * height);
        for (y, row) in rows.into_iter().enumerate() {
            let row_len = row.len();
            field.extend(row);
            // Pad rows shorter than `width` so the grid is fully rectangular.
            for x in row_len..width {
                field.push(SpanToken::new(Token::Empty, x, y, ' ', 0));
            }
        }
        Ok(AsciiField {
            field,
            width,
            height,
        })
    }

    pub fn get_field(&self) -> &[SpanToken] {
        &self.field
    }

    /// Returns the token at `(x, y)`, or [`Token::Empty`] for out-of-bounds.
    ///
    /// Uses a single `y * width + x` index into the flat backing array for
    /// better cache utilisation compared to a `Vec<Vec<_>>` lookup.
    #[inline]
    pub fn at(&self, position: &Position) -> Token {
        let &Position { x, y } = position;
        if x < 0 || y < 0 {
            return Token::Empty;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return Token::Empty;
        }
        *self.field[y * self.width + x].inner()
    }

    #[inline]
    pub fn at_spanned(&self, position: &Position) -> SpanToken {
        let &Position { x, y } = position;
        if x < 0 || y < 0 {
            return SpanToken::new(Token::Empty, x as usize, y as usize, ' ', 0);
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return SpanToken::new(Token::Empty, x, y, ' ', 0);
        }
        self.field[y * self.width + x]
    }

    pub fn offset(&self, position: &Position) -> usize {
        let x = position.x as usize;
        let y = position.y as usize;
        self.field[y * self.width + x].offset()
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}
