#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ListCellKind {
    Default,
    Quote,
    Changeable,
    Changing,
    Active,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ListCell {
    pub text: String,
    pub kind: ListCellKind,
    pub bold: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl ListCell {
    pub fn new(text: impl Into<String>, kind: ListCellKind) -> Self {
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

pub trait ListModel {
    fn item_count(&self) -> usize;
    fn get_item(&self, index: usize) -> Option<ListCell>;
}
