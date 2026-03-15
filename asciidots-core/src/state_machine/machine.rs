use crate::{
    parser::Token,
    primitives::{Direction, Position},
};

use super::dot::Dot;
use super::event::{MachineEvent, RuntimeErrorKind};
use super::states::{MachineState, MachineStateExecutor};

#[derive(Clone)]
pub struct DotMachine {
    pub(super) dot: Dot,
    pub(super) state: MachineState,
}

impl DotMachine {
    pub fn new(dir: Direction, x: i32, y: i32) -> Self {
        Self {
            dot: Dot::new(dir, Position::new(x, y), 0.0),
            state: MachineState::travel(),
        }
    }

    pub fn from(dot: Dot) -> Self {
        Self {
            dot,
            state: MachineState::travel(),
        }
    }

    pub fn with_state(dot: Dot, state: MachineState) -> Self {
        Self { dot, state }
    }

    pub fn is_dead(&self) -> bool {
        matches!(self.state, MachineState::Dead(_))
    }

    pub fn position(&self) -> &Position {
        &self.dot.position
    }

    pub fn direction(&self) -> &Direction {
        &self.dot.direction
    }

    pub fn value(&self) -> f64 {
        self.dot.value
    }

    pub fn dot(&self) -> Dot {
        self.dot
    }

    pub fn replace_dot(&mut self, dot: Dot) {
        self.dot = dot;
    }

    pub fn run(
        &mut self,
        token: &Token,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeErrorKind> {
        match &mut self.state {
            MachineState::Travel(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Set(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Waiting(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Printing(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::InputWaiting(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Dead(_) => Ok(MachineEvent::Nothing),
        }
    }

    pub fn next(&mut self, event: &MachineEvent, lookahead: &Token) {
        self.state = match event {
            MachineEvent::Died => MachineState::dead(),
            MachineEvent::EndProgram => MachineState::dead(),
            MachineEvent::HitOperator => MachineState::waiting(),
            MachineEvent::Nothing
            | MachineEvent::PrintLn(_)
            | MachineEvent::Print(_)
            | MachineEvent::Input(_) => match &mut self.state {
                MachineState::Travel(s) => s.next(&mut self.dot, lookahead),
                MachineState::Set(s) => s.next(&mut self.dot, lookahead),
                MachineState::Waiting(s) => s.next(&mut self.dot, lookahead),
                MachineState::Printing(s) => s.next(&mut self.dot, lookahead),
                MachineState::InputWaiting(s) => s.next(&mut self.dot, lookahead),
                MachineState::Dead(_) => self.state.clone(),
            },
        };
    }
}

impl std::fmt::Display for DotMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dot({:?} {:.2})", self.position(), self.dot.value)
    }
}

impl std::fmt::Debug for DotMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dot({:?} {:?} {:?} {:?} {})",
            self.state,
            self.position(),
            self.dot.direction,
            self.dot.value,
            self.is_dead()
        )
    }
}
