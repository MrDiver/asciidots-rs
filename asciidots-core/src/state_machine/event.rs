#[derive(Debug, Clone)]
pub enum MachineEvent {
    Nothing,
    Died,
    EndProgram,
    HitOperator,
    AssertionFailed { expected: f64, found: f64 },
    PrintLn(String),
    PrintChar(char),
    Print(String),
    Input(usize),
}
