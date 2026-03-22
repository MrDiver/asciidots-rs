use miette::SourceSpan;

use crate::{
    parser::{AsciiCell, SpanToken, Spanned},
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
    default_token: SpanToken,
    field: TokenField,
    width: usize,
    height: usize,
}

impl AsciiField {
    pub fn new(width: usize, height: usize) -> Self {
        let mut field = Vec::with_capacity(width * height);
        field.resize_with(width * height, || {
            Spanned::new(AsciiCell::Empty, ' ', (0, 1).into())
        });
        AsciiField {
            default_token: Spanned::new(AsciiCell::Empty, ' ', (0, 1).into()),
            field,
            width,
            height,
        }
    }

    pub fn get_field(&self) -> &[SpanToken] {
        &self.field
    }

    /// Returns the token at `(x, y)`, or [`Token::Empty`] for out-of-bounds.
    ///
    /// Uses a single `y * width + x` index into the flat backing array for
    /// better cache utilisation compared to a `Vec<Vec<_>>` lookup.
    #[inline]
    pub fn at(&self, position: &Position) -> &AsciiCell {
        let &Position { x, y } = position;
        if x < 0 || y < 0 {
            return &AsciiCell::Empty;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return &AsciiCell::Empty;
        }
        self.field[y * self.width + x].inner()
    }

    #[inline]
    pub fn at_spanned(&self, position: &Position) -> &SpanToken {
        let &Position { x, y } = position;
        if x < 0 || y < 0 {
            return &self.default_token;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return &self.default_token;
        }
        &self.field[y * self.width + x]
    }

    #[inline]
    pub fn at_spanned_mut(&mut self, position: &Position) -> Option<&mut SpanToken> {
        let &Position { x, y } = position;
        if x < 0 || y < 0 {
            return None;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(&mut self.field[y * self.width + x])
    }

    pub fn set_at(&mut self, value: SpanToken, position: &Position) {
        let &Position { x, y } = position;
        let (x, y) = (x as usize, y as usize);
        self.field[y * self.width + x] = value;
    }

    pub fn span(&self, position: &Position) -> SourceSpan {
        let x = position.x as usize;
        let y = position.y as usize;
        self.field[y * self.width + x].span()
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

pub struct FieldIterator<'a> {
    field: &'a AsciiField,
    idx_x: usize,
    idx_y: usize,
}

impl<'a> IntoIterator for &'a AsciiField {
    type Item = (usize, usize, &'a SpanToken);

    type IntoIter = FieldIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        FieldIterator {
            field: &self,
            idx_x: 0,
            idx_y: 0,
        }
    }
}

impl<'a> Iterator for FieldIterator<'a> {
    type Item = (usize, usize, &'a SpanToken);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx_y >= self.field.height() {
            return None;
        }
        let res = self.field.at_spanned(&(self.idx_x, self.idx_y).into());
        let (x, y) = (self.idx_x, self.idx_y);
        self.idx_x += 1;
        if self.idx_x >= self.field.width() {
            self.idx_x = 0;
            self.idx_y += 1;
        }
        Some((x, y, res))
    }
}
