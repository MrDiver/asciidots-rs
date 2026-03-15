use crate::parser::Token;

#[derive(Debug, Clone)]
pub enum RuntimeErrorKind {
    InvalidPrintingModifier(Token),
}

#[derive(Debug, Clone)]
pub enum MachineEvent {
    Nothing,
    Died,
    EndProgram,
    HitOperator,
    PrintLn(String),
    Print(String),
    Input(usize),
}
