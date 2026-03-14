use crate::dot::{Dot, Position};
use crate::token::{Direction, MirrorKind, Orientation, Token};

#[derive(Debug, Clone)]
pub enum SetKind {
    Id,
    Value,
}

#[derive(Debug, Clone)]
pub enum MachineEvent {
    Nothing,
    Died,
    EndProgram,
    HitOperator,
    PrintLn(String),
    Print(String),
    Input(usize),
}

#[derive(Debug, Clone)]
pub struct TravelState;

#[derive(Debug, Clone)]
pub struct SetState {
    kind: SetKind,
    acc: u64,
}
#[derive(Debug, Clone)]
pub struct DeadState;

#[derive(Debug, Clone)]
pub struct WaitingState;

#[derive(Debug, Clone)]
pub struct PrintingState {
    inside_quote: bool,
    streaming: bool,
    accumulator: String,
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
    fn travel() -> Self {
        MachineState::Travel(TravelState)
    }

    fn set(kind: SetKind) -> Self {
        MachineState::Set(SetState { kind, acc: 0 })
    }

    fn dead() -> Self {
        MachineState::Dead(DeadState)
    }

    fn waiting() -> Self {
        MachineState::Waiting(WaitingState)
    }

    fn printing() -> MachineState {
        MachineState::Printing(PrintingState {
            accumulator: String::default(),
            streaming: false,
            inside_quote: false,
        })
    }
}

pub enum RuntimeErrorKind {
    InvalidPrintingModifier(Token),
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

#[derive(Clone)]
pub struct DotMachine {
    dot: Dot,
    state: MachineState,
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
            MachineState::Travel(s) => s.process(&mut self.dot, &token, pending_dots),
            MachineState::Set(s) => s.process(&mut self.dot, &token, pending_dots),
            MachineState::Waiting(s) => s.process(&mut self.dot, &token, pending_dots),
            MachineState::Printing(s) => s.process(&mut self.dot, &token, pending_dots),
            MachineState::InputWaiting(s) => s.process(&mut self.dot, &token, pending_dots),
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
                MachineState::Travel(s) => s.next(&mut self.dot, &lookahead),
                MachineState::Set(s) => s.next(&mut self.dot, &lookahead),
                MachineState::Waiting(s) => s.next(&mut self.dot, &lookahead),
                MachineState::Printing(s) => s.next(&mut self.dot, &lookahead),
                MachineState::InputWaiting(s) => s.next(&mut self.dot, &lookahead),
                MachineState::Dead(_) => self.state.clone(),
            },
        };
    }
}

impl std::fmt::Display for DotMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dot({:?} {:.2})", self.position(), self.dot.value,)
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
                        // only spawn the TWO perpendicular directions
                        let mut new_dot = dot.clone();
                        new_dot.set_dir(*d).step();
                        pending_dots.push(DotMachine::from(new_dot));
                    }
                }
                // original dot keeps going forward
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
            match *token {
                Token::Quote => {
                    self.inside_quote = false;
                    return Ok(MachineEvent::PrintLn(self.accumulator.clone()));
                }
                _ => {}
            }
            self.accumulator.push_str(format!("{}", token).as_str());
            return Ok(MachineEvent::Nothing);
        } else if self.streaming {
            dot.step();
            match *token {
                Token::QuoteStream => {
                    self.streaming = false;
                    return Ok(MachineEvent::PrintLn(String::new()));
                }
                _ => {}
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
            MachineState::Printing(self.clone()) // TODO: Fix the cloning thing here by
        // making next consume
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
        dot: &mut Dot,
        token: &Token,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, RuntimeErrorKind> {
        todo!()
    }

    fn next(&mut self, dot: &mut Dot, token: &Token) -> MachineState {
        todo!()
    }
}
