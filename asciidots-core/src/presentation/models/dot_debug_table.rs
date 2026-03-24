use crate::{
    presentation::model_traits::{TableCell, TableCellKind, TableModel},
    state_machine::DotMachine,
};

pub struct DotDebugTableModel {
    headers: [String; 9],
    rows: Vec<[TableCell; 9]>,
}

impl DotDebugTableModel {
    pub fn from_dots<'a, I>(dots: &'a [DotMachine], operator_dots: I) -> Self
    where
        I: IntoIterator<Item = &'a DotMachine>,
    {
        let mut rows = Vec::new();
        rows.extend(dots.iter().map(|dot| Self::row(dot, false)));
        rows.extend(operator_dots.into_iter().map(|dot| Self::row(dot, true)));

        Self {
            headers: [
                "Addr".to_string(),
                "Value".to_string(),
                "X".to_string(),
                "Y".to_string(),
                "State".to_string(),
                "Selector".to_string(),
                "Location".to_string(),
                "Callstack Size".to_string(),
                "Callstack Last".to_string(),
            ],
            rows,
        }
    }

    fn row(dot: &DotMachine, in_operator: bool) -> [TableCell; 9] {
        let callstack = dot.dot().callstack();
        let callstack_size = callstack.len();
        let location = if in_operator { "Operator" } else { "Map" };
        let callstack_last = callstack
            .last()
            .map(|pos| format!("({}, {})", pos.x, pos.y))
            .unwrap_or_else(|| "-".to_string());

        [
            TableCell::new(format!("{}", dot.dot().addr()), TableCellKind::Changing),
            TableCell::new(format!("{:.2}", dot.dot().value()), TableCellKind::Changing),
            TableCell::new(format!("{}", dot.position().x), TableCellKind::Quote),
            TableCell::new(format!("{}", dot.position().y), TableCellKind::Quote),
            TableCell::new(dot.state_label(), TableCellKind::Active)
                .with_bold(!dot.is_dead())
                .with_strikethrough(dot.is_dead()),
            TableCell::new(format!("{:?}", dot.selector()), TableCellKind::Changeable),
            TableCell::new(location, TableCellKind::Changeable),
            TableCell::new(format!("{}", callstack_size), TableCellKind::Changeable),
            TableCell::new(callstack_last, TableCellKind::Quote),
        ]
    }
}

impl TableModel for DotDebugTableModel {
    fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn column_count(&self) -> usize {
        self.headers.len()
    }

    fn header(&self, column: usize) -> Option<&str> {
        self.headers.get(column).map(|value| value.as_str())
    }

    fn get_cell(&self, row: usize, column: usize) -> Option<TableCell> {
        self.rows
            .get(row)
            .and_then(|line| line.get(column))
            .cloned()
    }
}
