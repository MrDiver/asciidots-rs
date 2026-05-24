use std::cell::OnceCell;

use rustc_hash::FxHashSet;

use crate::{
    interpreter::AsciiField, parser::AsciiCell, primitives::Position, state_machine::DotMachine,
};

use crate::presentation::model_traits::{GridCell, GridCellKind, GridModel};

pub struct AsciiMapModel<'a> {
    field: &'a AsciiField,
    dots: &'a [DotMachine],
    operator_with_dots: FxHashSet<(i32, i32)>,
    /// Built on first call to get_cell — zero cost if the grid is never rendered.
    dot_positions: OnceCell<FxHashSet<(i32, i32)>>,
}

impl<'a> AsciiMapModel<'a> {
    pub fn new(
        field: &'a AsciiField,
        dots: &'a [DotMachine],
        operator_with_dots: FxHashSet<(i32, i32)>,
    ) -> Self {
        Self {
            field,
            dots,
            operator_with_dots,
            dot_positions: OnceCell::new(),
        }
    }

    fn dot_positions(&self) -> &FxHashSet<(i32, i32)> {
        self.dot_positions.get_or_init(|| {
            self.dots
                .iter()
                .map(|d| (d.position().x, d.position().y))
                .collect()
        })
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

        let pos = (x as i32, y as i32);
        let kind = if self.dot_positions().contains(&pos) {
            GridCellKind::ActiveDot
        } else {
            match token {
                AsciiCell::Quote => GridCellKind::Quote,
                AsciiCell::QuoteStream => GridCellKind::Quote,
                AsciiCell::OperatorBegin(_) | AsciiCell::OperatorEnd(_) => {
                    GridCellKind::OperatorDelimiter
                }
                AsciiCell::Operator { .. } if self.operator_with_dots.contains(&pos) => {
                    GridCellKind::ActiveOperatorWithDots
                }
                AsciiCell::Operator { .. } => GridCellKind::Operator,
                AsciiCell::Digit(_) => GridCellKind::Quote,
                AsciiCell::Char(_) => GridCellKind::Quote,
                AsciiCell::Duplicate => GridCellKind::Command,
                AsciiCell::Conditional(_) => GridCellKind::Command,
                AsciiCell::ValueMarker => GridCellKind::Command,
                AsciiCell::Write => GridCellKind::Command,
                AsciiCell::Read => GridCellKind::Command,
                AsciiCell::Call(_)
                | AsciiCell::CallById(_)
                | AsciiCell::Display(_)
                | AsciiCell::DefRet(_)
                | AsciiCell::Warp { .. } => GridCellKind::ActiveOperator,
                _ => GridCellKind::Default,
            }
        };

        Some(GridCell::new(char::from(token), kind))
    }
}
