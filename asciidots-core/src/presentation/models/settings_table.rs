use std::time::Duration;

use crate::presentation::model_traits::{TableCell, TableCellKind, TableModel};

pub struct SettingsTableModel {
    headers: [String; 2],
    rows: Vec<[TableCell; 2]>,
}

impl SettingsTableModel {
    pub fn from_runtime_state(
        running: bool,
        tick_speed: Duration,
        steps_per_tick: usize,
        tick_count: usize,
        last_step_duration: Duration,
        _dots: usize,
    ) -> Self {
        let running_value = if running { "Yes" } else { "No" };
        let running_cell = TableCell::new(
            running_value,
            if running {
                TableCellKind::Active
            } else {
                TableCellKind::Default
            },
        )
        .with_bold(true);

        let rows = vec![
            [
                TableCell::new("Running", TableCellKind::Changeable).with_bold(true),
                running_cell,
            ],
            [
                TableCell::new("Tick Speed", TableCellKind::Changeable),
                TableCell::new(
                    format!("{} ms", tick_speed.as_millis()),
                    TableCellKind::Default,
                ),
            ],
            [
                TableCell::new("Steps/Tick", TableCellKind::Changeable),
                TableCell::new(format!("{steps_per_tick}"), TableCellKind::Default),
            ],
            [
                TableCell::new("Ticks", TableCellKind::Changeable),
                TableCell::new(format!("{tick_count}"), TableCellKind::Changing),
            ],
            [
                TableCell::new("Last Step", TableCellKind::Changeable),
                TableCell::new(format!("{:?}", last_step_duration), TableCellKind::Default),
            ],
            [
                TableCell::new("Space", TableCellKind::Quote),
                TableCell::new("Run/Pause", TableCellKind::Default),
            ],
            [
                TableCell::new("n", TableCellKind::Quote),
                TableCell::new("Single Step", TableCellKind::Default),
            ],
            [
                TableCell::new("r", TableCellKind::Quote),
                TableCell::new("Reset", TableCellKind::Default),
            ],
            [
                TableCell::new("j / k", TableCellKind::Quote),
                TableCell::new("Delay +/-10 ms", TableCellKind::Default),
            ],
            [
                TableCell::new("u / i", TableCellKind::Quote),
                TableCell::new("Steps/Tick -/+1", TableCellKind::Default),
            ],
            [
                TableCell::new("↑ / ↓", TableCellKind::Quote),
                TableCell::new("Output Scroll", TableCellKind::Default),
            ],
            [
                TableCell::new("PgUp/PgDn", TableCellKind::Quote),
                TableCell::new("Output Scroll +/-10", TableCellKind::Default),
            ],
            [
                TableCell::new("wasd", TableCellKind::Quote),
                TableCell::new("Scroll", TableCellKind::Default),
            ],
            [
                TableCell::new("Esc", TableCellKind::Quote),
                TableCell::new("Quit", TableCellKind::Default).with_strikethrough(false),
            ],
        ];

        Self {
            headers: ["Setting".to_string(), "Value".to_string()],
            rows,
        }
    }
}

impl TableModel for SettingsTableModel {
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
        self.rows.get(row).and_then(|line| line.get(column)).cloned()
    }
}
