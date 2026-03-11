use crate::AsciiField;
use crate::dot::{Dot, Position};
use crate::parser::DiagnosticKind;
use crate::token::{Direction, MirrorKind, Orientation, Token};

#[derive(Debug, Copy, Clone)]
pub enum SetKind {
    Id,
    Value,
}

#[derive(Debug, Copy, Clone)]
pub enum MachineEvent {
    Nothing,
    Died,
    EndProgram,
}

#[derive(Debug, Copy, Clone)]
pub struct TravelState;
#[derive(Debug, Copy, Clone)]
pub struct SetState {
    kind: SetKind,
    acc: u64,
}
#[derive(Debug, Copy, Clone)]
pub struct DeadState;

#[derive(Debug, Copy, Clone)]
pub enum MachineState {
    Travel(TravelState),
    Set(SetState),
    Dead(DeadState),
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
}

pub trait MachineStateExecutor {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &Token,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, DiagnosticKind>;
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

    pub fn is_dead(&self) -> bool {
        matches!(self.state, MachineState::Dead(_))
    }

    pub fn position(&self) -> &Position {
        &self.dot.position
    }

    pub fn run(
        &mut self,
        token: &Token,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, DiagnosticKind> {
        match &mut self.state {
            MachineState::Travel(s) => s.process(&mut self.dot, &token, pending_dots),
            MachineState::Set(s) => s.process(&mut self.dot, &token, pending_dots),
            _ => Ok(MachineEvent::Nothing),
        }
    }

    pub fn next(&mut self, event: &MachineEvent, lookahead: &Token) {
        self.state = match event {
            MachineEvent::Nothing => match &mut self.state {
                MachineState::Travel(s) => s.next(&mut self.dot, &lookahead),
                MachineState::Set(s) => s.next(&mut self.dot, &lookahead),
                MachineState::Dead(_) => MachineState::dead(),
            },
            MachineEvent::Died => MachineState::dead(),
            MachineEvent::EndProgram => MachineState::dead(),
        };
    }
}

impl std::fmt::Display for DotMachine {
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

impl std::fmt::Debug for DotMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl MachineStateExecutor for TravelState {
    fn process(
        &mut self,
        dot: &mut Dot,
        token: &Token,
        pending_dots: &mut Vec<DotMachine>,
    ) -> Result<MachineEvent, DiagnosticKind> {
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
            // Pass these without dying
            Token::Digit(_) | Token::Char(_) => {
                dot.step();
            }
            _ => return Ok(MachineEvent::Died),
        };
        Ok(MachineEvent::Nothing)
    }

    fn next(&mut self, _dot: &mut Dot, token: &Token) -> MachineState {
        match *token {
            Token::Empty => MachineState::dead(),
            Token::ValueMarker => MachineState::set(SetKind::Value),
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
    ) -> Result<MachineEvent, DiagnosticKind> {
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
            Token::Digit(_) => MachineState::Set(*self),
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
