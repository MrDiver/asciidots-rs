use super::dot::Dot;
use super::error::RuntimeDiagnostic;
use super::event::MachineEvent;
use super::machine::DotMachine;
use crate::{
    parser::{MirrorKind, SpanToken, Token},
    primitives::{Direction, Orientation, Position},
};

#[derive(Debug, Clone, Copy)]
pub enum SetKind {
    Id,
    Value,
}

#[derive(Debug, Clone, Copy)]
pub struct TravelState;

#[derive(Debug, Clone, Copy)]
pub struct SetState {
    kind: SetKind,
    pub(super) acc: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DeadState;

#[derive(Debug, Clone, Copy)]
pub struct WaitingState;

#[derive(Debug, Clone)]
pub struct PrintingState {
    pub(super) inside_quote: bool,
    pub(super) streaming: bool,
    pub(super) accumulator: String,
}

#[derive(Debug, Clone)]
pub struct AssertingState {
    inside_quote: bool,
    accumulator: String,
}

#[derive(Debug, Clone, Copy)]
pub struct InputWaitingState;

#[derive(Debug, Clone)]
pub enum MachineState {
    Travel(TravelState),
    Set(SetState),
    Dead(DeadState),
    Waiting(WaitingState),
    Printing(PrintingState),
    Asserting(AssertingState),
    InputWaiting(InputWaitingState),
}

impl MachineState {
    pub(super) fn travel() -> Self {
        MachineState::Travel(TravelState)
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            MachineState::Travel(_) => "trav",
            MachineState::Set(_) => "set",
            MachineState::Dead(_) => "dead",
            MachineState::Waiting(_) => "wait",
            MachineState::Printing(_) => "print",
            MachineState::Asserting(_) => "assert",
            MachineState::InputWaiting(_) => "input",
        }
    }

    pub(super) fn set(kind: SetKind) -> Self {
        MachineState::Set(SetState { kind, acc: 0 })
    }

    pub(super) fn dead() -> Self {
        MachineState::Dead(DeadState)
    }

    pub(super) fn waiting() -> Self {
        MachineState::Waiting(WaitingState)
    }

    pub(super) fn printing() -> MachineState {
        MachineState::Printing(PrintingState {
            accumulator: String::default(),
            streaming: false,
            inside_quote: false,
        })
    }

    pub(super) fn asserting() -> MachineState {
        MachineState::Asserting(AssertingState {
            accumulator: String::default(),
            inside_quote: false,
        })
    }
}

pub trait MachineStateExecutor {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic>;
    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition;
}

pub enum StateTransition {
    Stay,
    Transition(MachineState),
}

impl MachineStateExecutor for TravelState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        match token.inner() {
            Token::Dot => {
                dot.step();
            }
            Token::Pipe(orientation) if *orientation == dot.orientation() => {
                dot.step();
            }
            Token::CrossPipe => {
                dot.step();
            }
            Token::ForceDirection(dir) => {
                if dot.direction.orthogonal(dir) {
                    dot.set_dir(*dir);
                }
                dot.step();
            }
            Token::Mirror(kind) => {
                let Position { x, y } = dot.direction.into();
                let tmp = match kind {
                    MirrorKind::Forward => Position { x: -y, y: -x },
                    MirrorKind::Backward => Position { x: y, y: x },
                }
                .into();
                if let Some(dir) = tmp {
                    dot.set_dir(dir).step();
                }
            }
            Token::Reflector(kind) if dot.orientation() == Orientation::Horizontal => {
                match kind {
                    MirrorKind::Forward => dot.set_dir(Direction::Right),
                    MirrorKind::Backward => dot.set_dir(Direction::Left),
                };
                dot.step();
            }
            Token::Duplicate => {
                pending_dots.reserve(2);
                let dot_snapshot = *dot;
                for d in &[
                    Direction::Up,
                    Direction::Down,
                    Direction::Left,
                    Direction::Right,
                ] {
                    if d != &dot_snapshot.direction && d != &dot_snapshot.direction.reverse() {
                        let mut new_dot = dot_snapshot;
                        new_dot.set_dir(*d).step();
                        pending_dots.push(DotMachine::from(new_dot));
                    }
                }
                dot.step();
            }
            Token::Digit(_) | Token::Char(_) => {
                dot.step();
            }
            Token::OperatorBegin(_) | Token::OperatorEnd(_)
                if matches!(dot.orientation(), Orientation::Horizontal) =>
            {
                dot.step();
                return Ok(MachineEvent::HitOperator);
            }
            Token::Operator { .. } => {
                return Ok(MachineEvent::HitOperator);
            }
            _ => return Ok(MachineEvent::Died),
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, _dot: &mut Dot, token: &SpanToken) -> StateTransition {
        match token.inner() {
            Token::Empty => StateTransition::Transition(MachineState::dead()),
            Token::ValueMarker => StateTransition::Transition(MachineState::set(SetKind::Value)),
            Token::IdMarker => StateTransition::Transition(MachineState::set(SetKind::Id)),
            Token::Write => StateTransition::Transition(MachineState::printing()),
            Token::Assertion => StateTransition::Transition(MachineState::asserting()),
            _ => StateTransition::Stay,
        }
    }
}

impl MachineStateExecutor for SetState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        match token.inner() {
            Token::Digit(d) => {
                self.acc = self.acc * 10 + *d as u64;
                dot.step();
            }
            _ => {
                dot.step();
            }
        }
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        match token.inner() {
            Token::Digit(_) => StateTransition::Stay,
            _ => {
                match self.kind {
                    SetKind::Value => dot.set_value(self.acc as f64),
                    SetKind::Id => dot.set_id(self.acc),
                };
                StateTransition::Transition(MachineState::travel())
            }
        }
    }
}

impl MachineStateExecutor for WaitingState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        match token.inner() {
            Token::OperatorBegin(_) | Token::OperatorEnd(_) | Token::Operator { .. } => {
                dot.step();
            }
            _ => {
                unreachable!("This should have never happened")
            }
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, _dot: &mut Dot, token: &SpanToken) -> StateTransition {
        match token.inner() {
            Token::OperatorBegin(_) | Token::OperatorEnd(_) => StateTransition::Stay,
            _ => StateTransition::Transition(MachineState::travel()),
        }
    }
}

impl MachineStateExecutor for PrintingState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        if self.inside_quote {
            dot.step();
            if *token.inner() == Token::Quote {
                self.inside_quote = false;
                return Ok(MachineEvent::PrintLn(std::mem::take(&mut self.accumulator)));
            }
            self.accumulator.push(char::from(*token.inner()));
            return Ok(MachineEvent::Nothing);
        } else if self.streaming {
            dot.step();
            if *token.inner() == Token::QuoteStream {
                self.streaming = false;
                return Ok(MachineEvent::PrintLn(String::new()));
            }
            return Ok(MachineEvent::PrintChar(char::from(*token.inner())));
        }
        match token.inner() {
            Token::Write => {
                dot.step();
            }
            Token::ValueMarker => {
                dot.step();
                return Ok(MachineEvent::PrintLn(dot.value.to_string()));
            }
            Token::IdMarker => {
                dot.step();
                return Ok(MachineEvent::PrintLn(dot.id.to_string()));
            }
            Token::QuoteStream => {
                dot.step();
                self.streaming = true;
            }
            Token::Quote => {
                dot.step();
                self.inside_quote = true;
            }
            c => {
                return Err(RuntimeDiagnostic::invalid_printing_modifier(
                    *c,
                    (token.offset(), 1).into(),
                ));
            }
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, _dot: &mut Dot, token: &SpanToken) -> StateTransition {
        if self.inside_quote || self.streaming {
            StateTransition::Stay
        } else {
            match token.inner() {
                Token::ValueMarker | Token::Quote | Token::QuoteStream | Token::IdMarker => {
                    StateTransition::Stay
                }
                _ => StateTransition::Transition(MachineState::travel()),
            }
        }
    }
}

impl MachineStateExecutor for InputWaitingState {
    fn process(
        &mut self,
        _dot: &mut Dot,
        _token: &SpanToken,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        todo!()
    }

    fn next(&mut self, _dot: &mut Dot, _token: &SpanToken) -> StateTransition {
        todo!()
    }
}

impl MachineStateExecutor for AssertingState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        dot.step();
        if self.inside_quote {
            if matches!(token.inner(), Token::Quote | Token::QuoteStream) {
                self.inside_quote = false;
                let expected: f64 = self.accumulator.parse().map_err(|_| {
                    RuntimeDiagnostic::failed_to_parse_assert(
                        std::mem::take(&mut self.accumulator),
                        {
                            let len = self.accumulator.len();
                            (token.offset() - len, len).into()
                        },
                    )
                })?;
                if dot.value == expected {
                    Ok(MachineEvent::Nothing)
                } else {
                    Ok(MachineEvent::AssertionFailed {
                        expected,
                        found: dot.value,
                    })
                }
            } else {
                self.accumulator.push(char::from(*token.inner()));
                Ok(MachineEvent::Nothing)
            }
        } else {
            match token.inner() {
                Token::Quote | Token::QuoteStream => {
                    self.inside_quote = true;
                }
                _ => {}
            }
            Ok(MachineEvent::Nothing)
        }
    }

    fn next(&mut self, _dot: &mut Dot, token: &SpanToken) -> StateTransition {
        if self.inside_quote {
            StateTransition::Stay
        } else {
            match token.inner() {
                Token::Quote | Token::QuoteStream => StateTransition::Stay,
                _ => StateTransition::Transition(MachineState::travel()),
            }
        }
    }
}

