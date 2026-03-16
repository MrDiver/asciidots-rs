#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TableCellKind {
    Default,
    Quote,
    Changeable,
    Changing,
    Active,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TableCell {
    pub text: String,
    pub kind: TableCellKind,
    pub bold: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl TableCell {
    pub fn new(text: impl Into<String>, kind: TableCellKind) -> Self {
        Self {
            text: text.into(),
            kind,
            bold: false,
            underline: false,
            strikethrough: false,
        }
    }

    pub fn with_bold(mut self, value: bool) -> Self {
        self.bold = value;
        self
    }

    pub fn with_underline(mut self, value: bool) -> Self {
        self.underline = value;
        self
    }

    pub fn with_strikethrough(mut self, value: bool) -> Self {
        self.strikethrough = value;
        self
    }
}

pub trait TableModel {
    fn row_count(&self) -> usize;
    fn column_count(&self) -> usize;
    fn header(&self, column: usize) -> Option<&str>;
    fn get_cell(&self, row: usize, column: usize) -> Option<TableCell>;
}
