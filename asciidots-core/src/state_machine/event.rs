#[derive(Debug, Clone)]
pub enum MachineEvent {
    Nothing,
    Died,
    EndProgram,
    HitOperator,
    AssertionFailed { expected: f64, found: f64 },
    PrintLn(String),
    PrintChar(char),
    Input(u64),
    Call(u64, String),
}
