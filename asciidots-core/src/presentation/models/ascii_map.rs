use rustc_hash::FxHashSet;

use crate::{
    interpreter::AsciiField, parser::Token, primitives::Position, state_machine::DotMachine,
};

use crate::presentation::model_traits::{GridCell, GridCellKind, GridModel};

pub struct AsciiMapModel<'a> {
    field: &'a AsciiField,
    dot_positions: FxHashSet<(usize, usize)>,
}

impl<'a> AsciiMapModel<'a> {
    pub fn new(field: &'a AsciiField, dots: &'a [DotMachine]) -> Self {
        let dot_positions = dots
            .iter()
            .map(|dot| (dot.position().x as usize, dot.position().y as usize))
            .collect();
        Self {
            field,
            dot_positions,
        }
    }
}

impl GridModel for AsciiMapModel<'_> {
    fn width(&self) -> usize {
        self.field.width()
    }

    fn height(&self) -> usize {
        self.field.height()
    }

    fn get_cell(&self, x: usize, y: usize) -> Option<GridCell> {
        if x >= self.width() || y >= self.height() {
            return None;
        }

        let token = self.field.at(&Position {
            x: x as i32,
            y: y as i32,
        });

        let kind = if self.dot_positions.contains(&(x, y)) {
            GridCellKind::ActiveDot
        } else {
            match token {
                Token::Quote => GridCellKind::Quote,
                Token::QuoteStream => GridCellKind::Quote,
                Token::OperatorBegin(_) | Token::OperatorEnd(_) => GridCellKind::OperatorDelimiter,
                Token::Operator { .. } => GridCellKind::Operator,
                Token::Digit(_) => GridCellKind::ActiveOperator,
                Token::Char(_) => GridCellKind::ActiveOperator,
                Token::Duplicate => GridCellKind::Command,
                Token::Conditonal(_) => GridCellKind::Command,
                Token::ValueMarker => GridCellKind::Command,
                Token::Write => GridCellKind::Command,
                Token::Read => GridCellKind::Command,
                _ => GridCellKind::Default,
            }
        };

        Some(GridCell::new(char::from(token), kind))
    }
}
