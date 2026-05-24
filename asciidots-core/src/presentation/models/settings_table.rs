use std::time::Duration;

use crate::presentation::model_traits::{TableCell, TableCellKind, TableModel};

const HEADERS: [&str; 2] = ["Setting", "Value"];

const HINTS: &[(&str, &str)] = &[
    ("Space", "Run/Pause"),
    ("n", "Single Step"),
    ("r", "Reset"),
    ("j / k", "Delay +/-10 ms"),
    ("u / i", "Steps/Tick -/+1"),
    ("↑ / ↓", "Output Scroll"),
    ("PgUp/PgDn", "Output Scroll +/-10"),
    ("wasd", "Scroll Map"),
    ("[ / ]", "Left Panel Width"),
    ("- / =", "Side Split"),
    ("Tab", "Switch Tab"),
    ("Esc", "Quit"),
];

const STAT_ROWS: usize = 5;

pub struct SettingsTableModel {
    running: bool,
    tick_speed: Duration,
    steps_per_tick: usize,
    tick_count: usize,
    num_dots: usize,
}

impl SettingsTableModel {
    pub fn from_runtime_state(
        running: bool,
        tick_speed: Duration,
        steps_per_tick: usize,
        tick_count: usize,
        num_dots: usize,
    ) -> Self {
        Self {
            running,
            tick_speed,
            steps_per_tick,
            tick_count,
            num_dots,
        }
    }
}

impl TableModel for SettingsTableModel {
    fn row_count(&self) -> usize {
        STAT_ROWS + HINTS.len()
    }

    fn column_count(&self) -> usize {
        2
    }

    fn header(&self, column: usize) -> Option<&str> {
        HEADERS.get(column).copied()
    }

    fn get_cell(&self, row: usize, column: usize) -> Option<TableCell> {
        if column > 1 {
            return None;
        }

        if row < STAT_ROWS {
            return Some(match (row, column) {
                (0, 0) => TableCell::new("Num Dots", TableCellKind::Changeable).with_bold(true),
                (0, 1) => TableCell::new(format!("{}", self.num_dots), TableCellKind::Default),
                (1, 0) => TableCell::new("Running", TableCellKind::Changeable).with_bold(true),
                (1, 1) => TableCell::new(
                    if self.running { "Yes" } else { "No" },
                    if self.running {
                        TableCellKind::Active
                    } else {
                        TableCellKind::Default
                    },
                )
                .with_bold(true),
                (2, 0) => TableCell::new("Tick Speed", TableCellKind::Changeable),
                (2, 1) => TableCell::new(
                    format!("{} ms", self.tick_speed.as_millis()),
                    TableCellKind::Default,
                ),
                (3, 0) => TableCell::new("Steps/Tick", TableCellKind::Changeable),
                (3, 1) => {
                    TableCell::new(format!("{}", self.steps_per_tick), TableCellKind::Default)
                }
                (4, 0) => TableCell::new("Ticks", TableCellKind::Changeable),
                (4, 1) => TableCell::new(format!("{}", self.tick_count), TableCellKind::Changing),
                _ => return None,
            });
        }

        let hint_row = row - STAT_ROWS;
        let (key, action) = HINTS.get(hint_row)?;
        Some(match column {
            0 => TableCell::new(*key, TableCellKind::Quote),
            1 => TableCell::new(*action, TableCellKind::Default),
            _ => return None,
        })
    }
}
