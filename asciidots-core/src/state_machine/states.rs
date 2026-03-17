use log::trace;

use super::dot::Dot;
use super::error::RuntimeDiagnostic;
use super::event::MachineEvent;
use super::machine::DotMachine;
use crate::{
    parser::{MirrorKind, SpanToken, Token},
    primitives::{Direction, Orientation, Position},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SetKind {
    Addr,
    Value,
}

#[derive(Debug, Clone, Copy)]
pub struct TravelState;

#[derive(Debug, Clone, Copy)]
pub struct SetState {
    waiting_input: bool,
    input: Option<f64>,
    kind: SetKind,
    pub(super) acc: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DeadState;

#[derive(Debug, Clone, Copy)]
pub struct WaitingState;

#[derive(Debug, Clone)]
pub struct PrintingState {
    inside_quote: bool,
    streaming: bool,
    accumulator: String,
    got_modifier: bool,
}

#[derive(Debug, Clone)]
pub struct AssertingState {
    inside_quote: bool,
    accumulator: String,
}

#[derive(Debug, Clone)]
pub enum MachineState {
    Travel(TravelState),
    Set(SetState),
    Dead(DeadState),
    Waiting(WaitingState),
    Printing(PrintingState),
    Asserting(AssertingState),
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
        }
    }

    pub(super) fn set(kind: SetKind) -> Self {
        MachineState::Set(SetState {
            waiting_input: false,
            input: None,
            kind,
            acc: 0,
        })
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
            got_modifier: false,
        })
    }

    pub fn asserting() -> MachineState {
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
    Transition,
}

impl MachineStateExecutor for TravelState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        trace!("[state{}] Process Travel", dot.uuid);
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
            Token::Conditional(_, _) if dot.direction() == Direction::Down => {
                return Ok(MachineEvent::Died);
            }
            Token::Conditional(_, _) => {
                return Ok(MachineEvent::HitOperator);
            }
            Token::Filter(true) => {
                if dot.value == 1.0 {
                    return Ok(MachineEvent::Died);
                }
                dot.step();
            }
            Token::Filter(false) => {
                if dot.value == 0.0 {
                    return Ok(MachineEvent::Died);
                }
                dot.step();
            }
            _ => return Ok(MachineEvent::Died),
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Travel", dot.uuid);
        match token.inner() {
            Token::Pipe(_)
            | Token::CrossPipe
            | Token::Mirror(_)
            | Token::ForceDirection(_)
            | Token::Reflector(_)
            | Token::Duplicate
            | Token::Dot
            | Token::Char(_)
            | Token::Digit(_)
            | Token::OperatorBegin(_)
            | Token::OperatorEnd(_)
            | Token::Operator { .. }
            | Token::Filter(_)
            | Token::Conditional(_, _) => StateTransition::Stay,
            _ => StateTransition::Transition,
        }
    }
}

impl SetState {
    pub fn receive_input(&mut self, value: f64) {
        self.input = Some(value)
    }
}

impl MachineStateExecutor for SetState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        trace!("[state{}] Process Set", dot.uuid);
        if self.waiting_input {
            return Ok(MachineEvent::Nothing);
        }
        match token.inner() {
            Token::Digit(d) => {
                self.acc = self.acc * 10 + *d as u64;
                dot.step();
            }
            Token::Read => {
                dot.step();
                return {
                    self.waiting_input = true;
                    Ok(MachineEvent::Input(dot.uuid))
                };
            }
            _ => {
                dot.step();
            }
        }
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Set", dot.uuid);
        if self.waiting_input && self.input.is_some() {
            match self.kind {
                SetKind::Value => dot.set_value(self.input.unwrap_or_else(|| unreachable!())),
                SetKind::Addr => dot.set_addr(self.input.unwrap_or_else(|| unreachable!()) as u64),
            };
            dot.set_value(self.input.unwrap_or_else(|| unreachable!()));
            dot.step();
            return StateTransition::Transition;
        } else if self.waiting_input {
            return StateTransition::Stay;
        }
        match token.inner() {
            Token::Read | Token::Digit(_) => StateTransition::Stay,
            _ => {
                match self.kind {
                    SetKind::Value => dot.set_value(self.acc as f64),
                    SetKind::Addr => dot.set_addr(self.acc),
                };
                StateTransition::Transition
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
        trace!("[state{}] Process Waiting", dot.uuid);
        match token.inner() {
            Token::OperatorBegin(_)
            | Token::OperatorEnd(_)
            | Token::Operator { .. }
            | Token::Conditional(_, _) => {
                dot.step();
            }
            _ => {
                unreachable!("This should have never happened")
            }
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Waiting", dot.uuid);
        match token.inner() {
            Token::OperatorBegin(_) | Token::OperatorEnd(_) => StateTransition::Stay,
            _ => StateTransition::Transition,
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
        trace!("[state{}] Process Printing", dot.uuid);
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
            Token::AddrMarker => {
                dot.step();
                return Ok(MachineEvent::PrintLn(dot.addr.to_string()));
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

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Printing", dot.uuid);
        if self.inside_quote || self.streaming {
            StateTransition::Stay
        } else {
            match token.inner() {
                Token::ValueMarker | Token::Quote | Token::QuoteStream | Token::AddrMarker
                    if !self.got_modifier =>
                {
                    self.got_modifier = true;
                    StateTransition::Stay
                }
                _ => StateTransition::Transition,
            }
        }
    }
}

impl MachineStateExecutor for AssertingState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &SpanToken,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeDiagnostic> {
        trace!("[state{}] Process Asserting", dot.uuid);
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

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Asserting", dot.uuid);
        if self.inside_quote {
            StateTransition::Stay
        } else {
            match token.inner() {
                Token::Quote | Token::QuoteStream => StateTransition::Stay,
                _ => StateTransition::Transition,
            }
        }
    }
}
