use log::trace;

use super::dot::Dot;
use super::error::RuntimeDiagnostic;
use super::event::MachineEvent;
use super::machine::DotMachine;
use crate::{
    parser::{AsciiCell, MirrorKind, SpanToken},
    primitives::{Direction, Orientation, Position},
    state_machine::dot::ValueSelector,
};

#[derive(Debug, Clone, Copy)]
pub struct TravelState {
    is_warping: bool,
}

#[derive(Debug, Clone)]
pub struct SetState {
    awaiting_input: bool,
    input: Option<f64>,
    target: ValueSelector,
    digit_accumulator: Option<u64>,
    in_quote: bool,
    accumulator: String,
    quoted_assignment_done: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DeadState;

#[derive(Debug, Clone, Copy)]
pub struct WaitingState;

#[derive(Debug, Clone)]
pub struct PrintingState {
    in_quote: bool,
    streaming: bool,
    accumulator: String,
    got_modifier: bool,
}

#[derive(Debug, Clone)]
pub struct AssertingState {
    in_quote: bool,
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
        MachineState::Travel(TravelState { is_warping: false })
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

    pub(super) fn set(target: ValueSelector) -> Self {
        MachineState::Set(SetState {
            awaiting_input: false,
            input: None,
            target,
            digit_accumulator: None,
            in_quote: false,
            accumulator: String::default(),
            quoted_assignment_done: false,
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
            in_quote: false,
            got_modifier: false,
        })
    }

    pub fn asserting() -> MachineState {
        MachineState::Asserting(AssertingState {
            accumulator: String::default(),
            in_quote: false,
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
        trace!("[state{}] Process Travel", dot.uuid());
        match token.inner() {
            AsciiCell::Dot => {
                dot.step();
            }
            AsciiCell::Pipe(orientation) if *orientation == dot.orientation() => {
                dot.step();
            }
            AsciiCell::CrossPipe => {
                dot.step();
            }
            AsciiCell::ForceDirection(dir) => {
                if dot.direction().orthogonal(dir) {
                    dot.set_dir(*dir);
                }
                dot.step();
            }
            AsciiCell::Mirror(kind) => {
                let Position { x, y } = (dot.direction()).into();
                let tmp = match kind {
                    MirrorKind::Forward => Position { x: -y, y: -x },
                    MirrorKind::Backward => Position { x: y, y: x },
                }
                .into();
                if let Some(dir) = tmp {
                    dot.set_dir(dir).step();
                }
            }
            AsciiCell::Reflector(kind) if dot.orientation() == Orientation::Horizontal => {
                match kind {
                    MirrorKind::Forward => dot.set_dir(Direction::Right),
                    MirrorKind::Backward => dot.set_dir(Direction::Left),
                };
                dot.step();
            }
            AsciiCell::Duplicate => {
                pending_dots.reserve(2);
                let dot_snapshot = dot.clone();
                for d in [
                    Direction::Up,
                    Direction::Down,
                    Direction::Left,
                    Direction::Right,
                ] {
                    if d != dot_snapshot.direction() && d != dot_snapshot.direction().reverse() {
                        let mut new_dot = dot_snapshot.clone();
                        new_dot.set_dir(d).step();
                        pending_dots.push(DotMachine::from(new_dot));
                    }
                }
                dot.step();
            }
            AsciiCell::Digit(_) | AsciiCell::Char(_) => {
                dot.step();
            }
            AsciiCell::OperatorBegin(_) | AsciiCell::OperatorEnd(_)
                if matches!(dot.orientation(), Orientation::Horizontal) =>
            {
                dot.step();
                return Ok(MachineEvent::HitOperator);
            }
            AsciiCell::Operator { .. } => {
                return Ok(MachineEvent::HitOperator);
            }
            // AsciiCell::Conditional(_) if dot.direction() == Direction::Down => {
            //     return Ok(MachineEvent::Died);
            // }
            AsciiCell::Conditional(_) => {
                return Ok(MachineEvent::HitOperator);
            }
            AsciiCell::Filter(true) => {
                if dot.selected() == 1.0 {
                    return Ok(MachineEvent::Died);
                }
                dot.step();
            }
            AsciiCell::Filter(false) => {
                if dot.selected() == 0.0 {
                    return Ok(MachineEvent::Died);
                }
                dot.step();
            }
            AsciiCell::Warp { other, .. } => {
                if self.is_warping {
                    dot.step();
                    self.is_warping = false;
                } else {
                    dot.set_position(*other);
                    self.is_warping = true;
                }
            }
            AsciiCell::Call(name) => {
                return Ok(MachineEvent::Call(dot.uuid(), name.clone()));
            }
            AsciiCell::CallById(id) => {
                return Ok(MachineEvent::CallById(dot.uuid(), *id));
            }
            AsciiCell::Display(display_id) => {
                return Ok(MachineEvent::Display {
                    display_id: *display_id,
                    direction: dot.direction(),
                    addr: dot.addr(),
                    value: dot.value(),
                });
            }
            AsciiCell::StopProgram => {
                return Ok(MachineEvent::EndProgram);
            }
            AsciiCell::DefRet(_) => {
                dot.ret()
                    .expect("For now don't handle returns when not inside a function");
                dot.step();
                return Ok(MachineEvent::Nothing);
            }
            AsciiCell::Comment(_) | AsciiCell::Negate => {
                dot.step();
            }
            _ => return Ok(MachineEvent::Died),
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Travel", dot.uuid());
        match token.inner() {
            AsciiCell::Pipe(_)
            | AsciiCell::CrossPipe
            | AsciiCell::Mirror(_)
            | AsciiCell::ForceDirection(_)
            | AsciiCell::Reflector(_)
            | AsciiCell::Duplicate
            | AsciiCell::Dot
            | AsciiCell::Char(_)
            | AsciiCell::Digit(_)
            | AsciiCell::OperatorBegin(_)
            | AsciiCell::OperatorEnd(_)
            | AsciiCell::Operator { .. }
            | AsciiCell::Filter(_)
            | AsciiCell::Conditional(_)
            | AsciiCell::Warp { .. } => {
                dot.reset_selector();
                StateTransition::Stay
            }

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
        trace!("[state{}] Process Set", dot.uuid());
        if self.awaiting_input {
            return Ok(MachineEvent::Nothing);
        }
        if self.in_quote {
            dot.step();
            if matches!(token.inner(), AsciiCell::Quote | AsciiCell::QuoteStream) {
                self.in_quote = false;
                let len = self.accumulator.len();
                let accumulator = std::mem::take(&mut self.accumulator);
                let parsed: f64 = accumulator.parse().map_err(|_| {
                    RuntimeDiagnostic::failed_to_parse_assert(
                        accumulator,
                        (token.span().offset().saturating_sub(len), len).into(),
                    )
                })?;
                match self.target {
                    ValueSelector::Value => {
                        dot.set_value(parsed);
                    }
                    ValueSelector::Addr => {
                        dot.set_addr(parsed);
                    }
                }
                self.quoted_assignment_done = true;
            } else {
                self.accumulator.push(char::from(token.inner()));
            }
            return Ok(MachineEvent::Nothing);
        }
        match token.inner() {
            AsciiCell::Digit(d) => {
                if self.digit_accumulator.is_none() {
                    self.digit_accumulator = Some(0)
                }
                self.digit_accumulator = self.digit_accumulator.map(|n| n * 10 + *d as u64);
                dot.step();
            }
            AsciiCell::Quote | AsciiCell::QuoteStream => {
                self.in_quote = true;
                self.accumulator.clear();
                dot.step();
            }
            AsciiCell::Read => {
                dot.step();
                return {
                    self.awaiting_input = true;
                    Ok(MachineEvent::Input(dot.uuid()))
                };
            }
            _ => {
                dot.step();
            }
        }
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Set", dot.uuid());
        if self.quoted_assignment_done {
            self.quoted_assignment_done = false;
            return StateTransition::Transition;
        }
        if self.in_quote {
            return StateTransition::Stay;
        }
        if self.awaiting_input && self.input.is_some() {
            match self.target {
                ValueSelector::Value => dot.set_value(self.input.unwrap_or_else(|| unreachable!())),
                ValueSelector::Addr => dot.set_addr(self.input.unwrap_or_else(|| unreachable!())),
            };
            dot.step();
            return StateTransition::Transition;
        } else if self.awaiting_input {
            return StateTransition::Stay;
        }

        match token.inner() {
            AsciiCell::Read | AsciiCell::Digit(_) | AsciiCell::Quote | AsciiCell::QuoteStream => {
                StateTransition::Stay
            }
            _ => {
                match (self.digit_accumulator, self.target) {
                    (Some(n), ValueSelector::Value) => {
                        dot.set_value(n as f64);
                    }
                    (Some(n), ValueSelector::Addr) => {
                        dot.set_addr(n as f64);
                    }
                    (None, target) => {
                        dot.select(target);
                    }
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
        trace!("[state{}] Process Waiting", dot.uuid());
        match token.inner() {
            AsciiCell::OperatorBegin(_)
            | AsciiCell::OperatorEnd(_)
            | AsciiCell::Operator { .. }
            | AsciiCell::Conditional(_) => {
                dot.step();
            }
            _ => {
                // unreachable!("This should have never happened")
                dot.step();
            }
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Waiting", dot.uuid());
        match token.inner() {
            AsciiCell::OperatorBegin(_) | AsciiCell::OperatorEnd(_) => StateTransition::Stay,
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
        trace!("[state{}] Process Printing", dot.uuid());
        if self.in_quote {
            dot.step();
            if matches!(*token.inner(), AsciiCell::Quote) {
                self.in_quote = false;
                return Ok(MachineEvent::PrintLn(std::mem::take(&mut self.accumulator)));
            }
            self.accumulator.push(char::from(token.inner()));
            return Ok(MachineEvent::Nothing);
        } else if self.streaming {
            dot.step();
            if matches!(*token.inner(), AsciiCell::QuoteStream) {
                self.streaming = false;
                return Ok(MachineEvent::PrintLn(String::new()));
            }
            return Ok(MachineEvent::PrintChar(char::from(token.inner())));
        }
        match token.inner() {
            AsciiCell::Write => {
                dot.step();
            }
            AsciiCell::ValueMarker => {
                dot.step();
                return Ok(MachineEvent::PrintLn(dot.value().to_string()));
            }
            AsciiCell::AddrMarker => {
                dot.step();
                return Ok(MachineEvent::PrintLn(dot.addr().to_string()));
            }
            AsciiCell::QuoteStream => {
                dot.step();
                self.streaming = true;
            }
            AsciiCell::Quote => {
                dot.step();
                self.in_quote = true;
            }
            _ => {
                dot.step();
                // return Err(RuntimeDiagnostic::invalid_printing_modifier(
                //     c.clone(),
                //     token.span(),
                // ));
            }
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Printing", dot.uuid());
        if self.in_quote || self.streaming {
            StateTransition::Stay
        } else {
            match token.inner() {
                AsciiCell::ValueMarker
                | AsciiCell::AddrMarker
                | AsciiCell::Quote
                | AsciiCell::QuoteStream
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
        trace!("[state{}] Process Asserting", dot.uuid());
        dot.step();
        if self.in_quote {
            if matches!(token.inner(), AsciiCell::Quote | AsciiCell::QuoteStream) {
                self.in_quote = false;
                let expected: f64 = self.accumulator.parse().map_err(|_| {
                    RuntimeDiagnostic::failed_to_parse_assert(
                        std::mem::take(&mut self.accumulator),
                        {
                            let len = self.accumulator.len();
                            (token.span().offset() - len, len).into()
                        },
                    )
                })?;
                if (dot.selected() - expected).abs() < 1e-3 {
                    Ok(MachineEvent::Nothing)
                } else {
                    Ok(MachineEvent::AssertionFailed {
                        expected,
                        found: dot.selected(),
                    })
                }
            } else {
                self.accumulator.push(char::from(token.inner()));
                Ok(MachineEvent::Nothing)
            }
        } else {
            match token.inner() {
                AsciiCell::Quote | AsciiCell::QuoteStream => {
                    self.in_quote = true;
                }
                _ => {}
            }
            Ok(MachineEvent::Nothing)
        }
    }

    fn next(&mut self, dot: &mut Dot, token: &SpanToken) -> StateTransition {
        trace!("[state{}] Next Asserting", dot.uuid());
        if self.in_quote {
            StateTransition::Stay
        } else {
            match token.inner() {
                AsciiCell::Quote | AsciiCell::QuoteStream => StateTransition::Stay,
                _ => StateTransition::Transition,
            }
        }
    }
}
