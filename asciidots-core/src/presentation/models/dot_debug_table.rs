use crate::{
    presentation::model_traits::{TableCell, TableCellKind, TableModel},
    state_machine::DotMachine,
};

pub struct DotDebugTableModel {
    headers: [String; 5],
    rows: Vec<[TableCell; 5]>,
}

impl DotDebugTableModel {
    pub fn from_dots(dots: &[DotMachine]) -> Self {
        let rows = dots
            .iter()
            .map(|dot| {
                [
                    TableCell::new(format!("{}", dot.addr()), TableCellKind::Changing),
                    TableCell::new(format!("{:.2}", dot.value()), TableCellKind::Default),
                    TableCell::new(format!("{}", dot.position().x), TableCellKind::Quote),
                    TableCell::new(format!("{}", dot.position().y), TableCellKind::Quote),
                    TableCell::new(dot.state_label(), TableCellKind::Active)
                        .with_bold(!dot.is_dead())
                        .with_strikethrough(dot.is_dead()),
                ]
            })
            .collect();

        Self {
            headers: [
                "ID".to_string(),
                "Value".to_string(),
                "X".to_string(),
                "Y".to_string(),
                "State".to_string(),
            ],
            rows,
        }
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
