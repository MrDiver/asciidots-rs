use crate::{
    engine::{
        callstack::{Callstack, Callstackable},
        dot_trait::{EngineDot, IsTrue},
        operator::Operand,
        queues::Pairable,
    },
    parser::{AsciiCell, SpanToken},
    primitives::{Directed, Direction, Orientation, Oriented, Position},
    state_machine::ValueSelector,
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
            | MachineEvent::Input(_)
            | MachineEvent::Display { .. } => match &mut self.state {
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
    fn callstack(&self) -> &Callstack<Position> {
        self.dot().callstack()
    }

    fn callstack_mut(&mut self) -> &mut Callstack<Position> {
        self.dot_mut().callstack_mut()
    }
}

impl Directed for DotMachine {
    fn direction(&self) -> Direction {
        self.dot.direction()
    }

    fn set_direction(&mut self, direction: Direction) {
        self.dot.set_dir(direction);
    }
}

impl Oriented for DotMachine {
    fn orientation(&self) -> Orientation {
        self.dot.orientation()
    }
}

impl IsTrue for DotMachine {
    fn is_true(&self) -> bool {
        self.dot.selected() as u64 != 0
    }
}

impl Pairable for DotMachine {
    fn key(&self) -> u64 {
        self.dot.callstack().current_hash()
    }
}

impl Operand<f64> for DotMachine {
    fn apply_operator(mut self, other: Self, op: impl Fn(f64, f64) -> f64) -> Self {
        let result = op(self.dot.selected(), other.dot.selected());
        self.dot.set_selected(result);
        self
    }
}

impl EngineDot for DotMachine {
    type Value = f64;
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
