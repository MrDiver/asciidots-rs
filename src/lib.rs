pub mod dot;
pub mod interpreter;
pub mod parser;
pub mod state;
pub mod token;

use std::collections::VecDeque;

use rustc_hash::FxHashMap;

use crate::interpreter::MachineState;
use crate::{
    dot::{Dot, Position},
    interpreter::{DotMachine, MachineEvent},
    parser::{Diagnostic, Lexer, Severity},
    token::{Direction, Orientation, SpanToken, Token},
};

pub type TokenField = Vec<Vec<SpanToken>>;

#[derive(Debug)]
pub struct AsciiField {
    field: TokenField,
}

impl AsciiField {
    pub fn from_string(content: &str) -> Result<Self, Vec<Diagnostic>> {
        let field: Vec<Vec<char>> = content.lines().map(|line| line.chars().collect()).collect();
        let mut lexer = Lexer::new(field.clone());
        lexer.run();
        let field = lexer.finish()?;
        Ok(AsciiField { field })
    }

    pub fn get_field(&self) -> &TokenField {
        &self.field
    }

    pub fn at(&self, position: &Position) -> Token {
        let &Position { x, y } = position;
        self.field
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .map(|token| *token.inner())
            .unwrap_or_else(|| Token::Empty)
    }
}

#[derive(Debug)]
pub struct OperatorQueues {
    orientation: Orientation,
    op: char,
    horizontal: VecDeque<DotMachine>,
    vertical: VecDeque<DotMachine>,
}

impl OperatorQueues {
    fn new(orientation: Orientation, op: char) -> Self {
        Self {
            orientation,
            op,
            horizontal: VecDeque::with_capacity(1),
            vertical: VecDeque::with_capacity(1),
        }
    }

    fn push(&mut self, dot: DotMachine) {
        match dot.direction().orientation() {
            Orientation::Horizontal => self.horizontal.push_back(dot),
            Orientation::Vertical => self.vertical.push_back(dot),
        }
    }

    fn get_result(&mut self) -> Option<DotMachine> {
        if self.horizontal.len() > 0 && self.vertical.len() > 0 {
            let (mut primary, secondary) = {
                let doth = self.horizontal.pop_front().unwrap();
                let dotv = self.vertical.pop_front().unwrap();
                if matches!(self.orientation, Orientation::Horizontal) {
                    (doth, dotv)
                } else {
                    (dotv, doth)
                }
            };

            let a = primary.value();
            let b = secondary.value();
            let new_val = match self.op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                _ => a,
            };
            let mut new_dot = primary.dot();
            new_dot.set_value(new_val);
            primary.replace_dot(new_dot);
            return Some(primary);
        }
        None
    }
}

pub struct ProgramState {
    field: AsciiField,
    dots: Vec<DotMachine>,
    pending_dots: Vec<DotMachine>,
    diagnostics: Vec<Diagnostic>,
    operator_map: FxHashMap<(usize, usize), OperatorQueues>,
    output: String,
}

impl ProgramState {
    pub fn from_string(content: &str) -> Result<Self, Vec<Diagnostic>> {
        let field = AsciiField::from_string(content)?;
        let mut state = ProgramState {
            field,
            dots: Vec::new(),
            pending_dots: Vec::with_capacity(10),
            diagnostics: Vec::new(),
            operator_map: FxHashMap::default(),
            output: String::with_capacity(128),
        };
        state.initalize();
        Ok(state)
    }

    pub fn get_field(&self) -> &AsciiField {
        &self.field
    }

    fn initalize(&mut self) {
        for row in self.field.get_field() {
            for token in row {
                let position = Position::new(token.col() as i32, token.line() as i32);
                match *token.inner() {
                    Token::Dot => {
                        for direction in &[
                            Direction::Up,
                            Direction::Down,
                            Direction::Left,
                            Direction::Right,
                        ] {
                            let offsetpos = position + (*direction).into();
                            if let Token::Pipe(orientation) = self.field.at(&offsetpos)
                                && orientation == direction.orientation()
                            {
                                self.dots.push(DotMachine::new(
                                    *direction,
                                    token.col() as i32,
                                    token.line() as i32,
                                ));
                                break; // Only one dot per position, so break after finding the first valid direction
                            }
                        }
                    }
                    Token::Operator { orientation, repr } => {
                        self.operator_map.insert(
                            (token.col(), token.line()),
                            OperatorQueues::new(orientation, repr),
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    fn interpret(&mut self) {
        let mut i = 0;
        let mut repeat_iteration = false;
        while i < self.dots.len() {
            let dot = &mut self.dots[i];
            let token = &self.field.at(dot.position());
            let event = match dot.run(token, &mut self.pending_dots) {
                Ok(e) => e,
                Err(d) => {
                    let Position { x, y } = dot.position();
                    self.diagnostics.push(Diagnostic::new(
                        *x as usize,
                        *y as usize,
                        d,
                        Severity::Error,
                    ));
                    MachineEvent::EndProgram
                }
            };
            match event {
                MachineEvent::EndProgram => {
                    self.dots.clear();
                    return;
                }
                MachineEvent::HitOperator => {
                    let mut dot = self.dots.swap_remove(i);
                    dot.next(&event, &Token::Empty);
                    let opqueue = self
                        .operator_map
                        .get_mut(&(dot.position().into()))
                        .unwrap_or_else(|| unreachable!());
                    opqueue.push(dot);
                    // Add a result dot back to the running loop
                    if let Some(dot) = opqueue.get_result() {
                        self.dots.push(dot)
                    }

                    repeat_iteration = true;
                }
                MachineEvent::PrintLn(s) => {
                    self.output.push_str(format!("{}\n", s).as_str());
                    let lookahead = self.field.at(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                MachineEvent::Print(s) => {
                    self.output.push_str(format!("{}", s).as_str());
                    let lookahead = self.field.at(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                _ => {
                    let lookahead = self.field.at(dot.position());
                    dot.next(&event, &lookahead)
                }
            }
            if !repeat_iteration {
                i += 1;
            } else {
                repeat_iteration = false;
            }
        }
    }

    pub(crate) fn cleanup_dead_dots(dots: &mut Vec<DotMachine>) {
        let mut i = 0;
        while i < dots.len() {
            if dots[i].is_dead() {
                dots.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn step(&mut self) -> Result<(), &[Diagnostic]> {
        // 1. Step all Dots
        self.interpret();

        // 2. Compaction of Dots (remove dead)
        Self::cleanup_dead_dots(&mut self.dots);
        // 3. Append new Dots
        self.dots.extend_from_slice(&self.pending_dots);

        // 4. Clear added dots from pending list
        self.pending_dots.clear();

        if !self.diagnostics.is_empty() {
            Err(&self.diagnostics)
        } else {
            Ok(())
        }
    }

    pub fn num_dots(&self) -> usize {
        self.dots.len()
    } // TODO: Investigate if it needs counting the operator dots

    pub fn get_dots(&self) -> &[DotMachine] {
        &self.dots
    }

    pub fn queues(&self) -> &FxHashMap<(usize, usize), OperatorQueues> {
        &self.operator_map
    }

    pub fn output(&self) -> &str {
        &self.output
    }
}

impl std::fmt::Debug for ProgramState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Tokens: {:?}",
            self.field
                .get_field()
                .iter()
                .flat_map(|row| row.iter().map(|token| token.inner()))
                .collect::<Vec<_>>()
        )?;
        writeln!(f, "Dots:")?;
        for dot in &self.dots {
            writeln!(f, "{}", dot)?;
        }
        Ok(())
    }
}
