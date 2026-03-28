use crate::{
    parser::{AsciiCell, SpanToken},
    primitives::{Direction, Position},
    state_machine::{Callstackable, ValueSelector},
};

use super::dot::Dot;
use super::error::RuntimeDiagnostic;
use super::event::MachineEvent;
use super::states::{MachineState, MachineStateExecutor, StateTransition};

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
        self.dot.position()
    }

    pub fn direction(&self) -> Direction {
        self.dot.direction()
    }

    pub fn selected(&self) -> f64 {
        self.dot.selected()
    }

    pub fn uuid(&self) -> u64 {
        self.dot.uuid()
    }

    pub fn state_label(&self) -> &'static str {
        self.state.short_name()
    }

    pub fn dot(&self) -> &Dot {
        &self.dot
    }

    pub fn dot_mut(&mut self) -> &mut Dot {
        &mut self.dot
    }

    pub fn state_mut(&mut self) -> &mut MachineState {
        &mut self.state
    }

    pub fn replace_dot(&mut self, dot: Dot) {
        self.dot = dot;
    }

    pub fn selector(&self) -> ValueSelector {
        self.dot.selector()
    }

    pub fn process(
        &mut self,
        token: &SpanToken,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        match &mut self.state {
            MachineState::Travel(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Set(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Waiting(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Printing(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Asserting(s) => s.process(&mut self.dot, token, pending_dots),
            MachineState::Dead(_) => Ok(MachineEvent::Nothing),
        }
    }

    pub fn next(&mut self, event: &MachineEvent, lookahead: &SpanToken) {
        let transition = match event {
            MachineEvent::Died
            | MachineEvent::EndProgram
            | MachineEvent::AssertionFailed { .. } => {
                self.state = MachineState::dead();
                StateTransition::Stay
            }
            MachineEvent::HitOperator => {
                self.state = MachineState::waiting();
                StateTransition::Stay
            }
            MachineEvent::Nothing
            | MachineEvent::PrintLn(_)
            | MachineEvent::PrintChar(_)
            | MachineEvent::Call(_, _)
            | MachineEvent::CallById(_, _)
            | MachineEvent::Input(_) => match &mut self.state {
                MachineState::Travel(s) => s.next(&mut self.dot, lookahead),
                MachineState::Set(s) => s.next(&mut self.dot, lookahead),
                MachineState::Waiting(s) => s.next(&mut self.dot, lookahead),
                MachineState::Printing(s) => s.next(&mut self.dot, lookahead),
                MachineState::Asserting(s) => s.next(&mut self.dot, lookahead),
                MachineState::Dead(_) => {
                    self.state = MachineState::dead();
                    StateTransition::Stay
                }
            },
        };

        if let StateTransition::Transition = transition {
            self.state = match lookahead.inner() {
                AsciiCell::Empty => MachineState::dead(),
                AsciiCell::ValueMarker => MachineState::set(ValueSelector::Value),
                AsciiCell::AddrMarker => MachineState::set(ValueSelector::Addr),
                AsciiCell::Write => MachineState::printing(),
                AsciiCell::Assertion => MachineState::asserting(),
                _ => MachineState::travel(),
            }
        }
    }
}

impl Callstackable<Position> for DotMachine {
    fn callstack(&self) -> &super::CallStack<Position> {
        self.dot().callstack()
    }

    fn callstack_mut(&mut self) -> &mut super::CallStack<Position> {
        self.dot_mut().callstack_mut()
    }
}

impl std::fmt::Display for DotMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dot({:?} {:.2})", self.position(), self.dot.value())
    }
}

impl std::fmt::Debug for DotMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dot({:?} {:?} {:?} {:?} {})",
            self.state,
            self.position(),
            self.dot.direction(),
            self.dot.value(),
            self.is_dead()
        )
    }
}
