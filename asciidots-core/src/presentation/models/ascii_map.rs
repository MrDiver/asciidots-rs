use rustc_hash::FxHashSet;

use crate::{
    interpreter::AsciiField, parser::AsciiCell, primitives::Position, state_machine::DotMachine,
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
                AsciiCell::Quote => GridCellKind::Quote,
                AsciiCell::QuoteStream => GridCellKind::Quote,
                AsciiCell::OperatorBegin(_) | AsciiCell::OperatorEnd(_) => {
                    GridCellKind::OperatorDelimiter
                }
                AsciiCell::Operator { .. } => GridCellKind::Operator,
                AsciiCell::Digit(_) => GridCellKind::ActiveOperator,
                AsciiCell::Char(_) => GridCellKind::ActiveOperator,
                AsciiCell::Duplicate => GridCellKind::Command,
                AsciiCell::Conditional(_) => GridCellKind::Command,
                AsciiCell::ValueMarker => GridCellKind::Command,
                AsciiCell::Write => GridCellKind::Command,
                AsciiCell::Read => GridCellKind::Command,
                _ => GridCellKind::Default,
            }
        };

        Some(GridCell::new(char::from(token), kind))
    }
}
