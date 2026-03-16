#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GridCellKind {
    Default,
    Quote,
    OperatorDelimiter,
    Operator,
    ActiveOperator,
    Command,
    ActiveDot,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct GridCell {
    pub c: char,
    pub kind: GridCellKind,
}

impl GridCell {
    pub const fn new(c: char, kind: GridCellKind) -> Self {
        Self { c, kind }
    }
}

pub trait GridModel {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn get_cell(&self, x: usize, y: usize) -> Option<GridCell>;
}
