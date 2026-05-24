use crate::{
    presentation::model_traits::{TableCell, TableCellKind, TableModel},
    state_machine::DotMachine,
};

const HEADERS: [&str; 9] = [
    "Addr",
    "Value",
    "X",
    "Y",
    "State",
    "Selector",
    "Location",
    "Callstack Size",
    "Callstack Last",
];

/// Lazy dots table — borrows the dot slices directly and computes each cell
/// only when [`TableModel::get_cell`] is called. Building this model is O(1)
/// (aside from collecting the operator-dots iterator, which is usually tiny).
pub struct DotDebugTableModel<'a> {
    dots: &'a [DotMachine],
    operator_dots: Vec<&'a DotMachine>,
}

impl<'a> DotDebugTableModel<'a> {
    pub fn from_dots<I>(dots: &'a [DotMachine], operator_dots: I) -> Self
    where
        I: IntoIterator<Item = &'a DotMachine>,
    {
        Self {
            dots,
            operator_dots: operator_dots.into_iter().collect(),
        }
    }

    fn get_dot_and_kind(&self, row: usize) -> Option<(&DotMachine, bool)> {
        if row < self.dots.len() {
            Some((&self.dots[row], false))
        } else {
            self.operator_dots
                .get(row - self.dots.len())
                .map(|d| (*d, true))
        }
    }
}

impl TableModel for DotDebugTableModel<'_> {
    fn row_count(&self) -> usize {
        self.dots.len() + self.operator_dots.len()
    }

    fn column_count(&self) -> usize {
        HEADERS.len()
    }

    fn header(&self, column: usize) -> Option<&str> {
        HEADERS.get(column).copied()
    }

    fn get_cell(&self, row: usize, column: usize) -> Option<TableCell> {
        let (dot, in_operator) = self.get_dot_and_kind(row)?;
        let callstack = dot.dot().callstack();
        Some(match column {
            0 => TableCell::new(format!("{}", dot.dot().addr()), TableCellKind::Changing),
            1 => TableCell::new(format!("{:.2}", dot.dot().value()), TableCellKind::Changing),
            2 => TableCell::new(format!("{}", dot.position().x), TableCellKind::Quote),
            3 => TableCell::new(format!("{}", dot.position().y), TableCellKind::Quote),
            4 => TableCell::new(dot.state_label(), TableCellKind::Active)
                .with_bold(!dot.is_dead())
                .with_strikethrough(dot.is_dead()),
            5 => TableCell::new(format!("{:?}", dot.selector()), TableCellKind::Changeable),
            6 => TableCell::new(
                if in_operator { "Operator" } else { "Map" },
                TableCellKind::Changeable,
            ),
            7 => TableCell::new(format!("{}", callstack.len()), TableCellKind::Changeable),
            8 => TableCell::new(
                callstack
                    .last()
                    .map(|pos| format!("({}, {})", pos.x, pos.y))
                    .unwrap_or_else(|| "-".to_string()),
                TableCellKind::Quote,
            ),
            _ => return None,
        })
    }
}
