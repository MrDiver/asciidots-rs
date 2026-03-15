use super::dot::Dot;
use super::event::{MachineEvent, RuntimeErrorKind};
use super::machine::DotMachine;
use crate::{
    parser::{MirrorKind, Token},
    primitives::{Direction, Orientation, Position},
};

#[derive(Debug, Clone)]
pub enum SetKind {
    Id,
    Value,
}

#[derive(Debug, Clone)]
pub struct TravelState;

#[derive(Debug, Clone)]
pub struct SetState {
    kind: SetKind,
    pub(super) acc: u64,
}

#[derive(Debug, Clone)]
pub struct DeadState;

#[derive(Debug, Clone)]
pub struct WaitingState;

#[derive(Debug, Clone)]
pub struct PrintingState {
    pub(super) inside_quote: bool,
    pub(super) streaming: bool,
    pub(super) accumulator: String,
}

#[derive(Debug, Clone)]
pub struct InputWaitingState;

#[derive(Debug, Clone)]
pub enum MachineState {
    Travel(TravelState),
    Set(SetState),
    Dead(DeadState),
    Waiting(WaitingState),
    Printing(PrintingState),
    InputWaiting(InputWaitingState),
}

impl MachineState {
    pub(super) fn travel() -> Self {
        MachineState::Travel(TravelState)
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
}

pub trait MachineStateExecutor {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &Token,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeErrorKind>;
    fn next(&mut self, dot: &mut Dot, token: &Token) -> MachineState;
}

impl MachineStateExecutor for TravelState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &Token,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeErrorKind> {
        match *token {
            Token::Dot => {
                dot.step();
            }
            Token::Pipe(orientation) if orientation == dot.orientation() => {
                dot.step();
            }
            Token::CrossPipe => {
                dot.step();
            }
            Token::ForceDirection(dir) => {
                if dot.direction.orthogonal(&dir) {
                    dot.set_dir(dir);
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
                for d in &[
                    Direction::Up,
                    Direction::Down,
                    Direction::Left,
                    Direction::Right,
                ] {
                    if d != &dot.direction && d != &dot.direction.reverse() {
                        let mut new_dot = dot.clone();
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

    fn next(&mut self, _dot: &mut Dot, token: &Token) -> MachineState {
        match *token {
            Token::Empty => MachineState::dead(),
            Token::ValueMarker => MachineState::set(SetKind::Value),
            Token::IdMarker => MachineState::set(SetKind::Id),
            Token::Write => MachineState::printing(),
            _ => MachineState::travel(),
        }
    }
}

impl MachineStateExecutor for SetState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &Token,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeErrorKind> {
        match *token {
            Token::Digit(d) => {
                self.acc = self.acc * 10 + d as u64;
                dot.step();
            }
            _ => {
                dot.step();
            }
        }
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, dot: &mut Dot, token: &Token) -> MachineState {
        match *token {
            Token::Digit(_) => MachineState::Set(self.clone()),
            _ => {
                match self.kind {
                    SetKind::Value => dot.set_value(self.acc as f64),
                    SetKind::Id => dot.set_id(self.acc),
                };
                MachineState::travel()
            }
        }
    }
}

impl MachineStateExecutor for WaitingState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &Token,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeErrorKind> {
        match *token {
            Token::OperatorBegin(_) | Token::OperatorEnd(_) | Token::Operator { .. } => {
                dot.step();
            }
            _ => {
                unreachable!("This should have never happened")
            }
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, _dot: &mut Dot, token: &Token) -> MachineState {
        match token {
            Token::OperatorBegin(_) | Token::OperatorEnd(_) => MachineState::waiting(),
            _ => MachineState::travel(),
        }
    }
}

impl MachineStateExecutor for PrintingState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &Token,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeErrorKind> {
        if self.inside_quote {
            dot.step();
            if *token == Token::Quote {
                self.inside_quote = false;
                return Ok(MachineEvent::PrintLn(self.accumulator.clone()));
            }
            self.accumulator.push_str(format!("{}", token).as_str());
            return Ok(MachineEvent::Nothing);
        } else if self.streaming {
            dot.step();
            if *token == Token::QuoteStream {
                self.streaming = false;
                return Ok(MachineEvent::PrintLn(String::new()));
            }
            let char = format!("{}", token);
            return Ok(MachineEvent::Print(char));
        }
        match *token {
            Token::Write => {
                dot.step();
            }
            Token::ValueMarker => {
                dot.step();
                return Ok(MachineEvent::PrintLn(format!("{}", dot.value)));
            }
            Token::IdMarker => {
                dot.step();
                return Ok(MachineEvent::PrintLn(format!("{}", dot.id)));
            }
            Token::QuoteStream => {
                dot.step();
                self.streaming = true;
            }
            Token::Quote => {
                dot.step();
                self.inside_quote = true;
            }
            c => return Err(RuntimeErrorKind::InvalidPrintingModifier(c)),
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, _dot: &mut Dot, token: &Token) -> MachineState {
        if self.inside_quote || self.streaming {
            MachineState::Printing(self.clone())
        } else {
            match *token {
                Token::ValueMarker | Token::Quote | Token::QuoteStream | Token::IdMarker => {
                    MachineState::Printing(self.clone())
                }
                _ => MachineState::travel(),
            }
        }
    }
}

impl MachineStateExecutor for InputWaitingState {
    fn process(
        &mut self,
        _dot: &mut Dot,
        _token: &Token,
        _pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeErrorKind> {
        todo!()
    }

    fn next(&mut self, _dot: &mut Dot, _token: &Token) -> MachineState {
        todo!()
    }
}
