use std::sync::Arc;

use crate::primitives::Direction;

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
    Call(u64, Arc<str>),
    CallById(u64, u32),
    Display {
        display_id: u32,
        direction: Direction,
        addr: f64,
        value: f64,
    },
}
