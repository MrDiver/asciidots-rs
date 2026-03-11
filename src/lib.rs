pub mod dot;
pub mod interpreter;
pub mod parser;
pub mod state;
pub mod token;

use std::collections::VecDeque;

use crate::{
    dot::{Dot, Position},
    interpreter::{DotMachine, MachineEvent, MachineState},
    parser::{Diagnostic, Lexer, Severity},
    token::{Direction, SpanToken, Token},
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

struct OperatorQueues<'a> {
    horizontal: VecDeque<&'a DotMachine>,
    vertical: VecDeque<&'a DotMachine>,
}

pub struct ProgramState {
    field: AsciiField,
    dots: Vec<DotMachine>,
    pending_dots: Vec<DotMachine>,
    diagnostics: Vec<Diagnostic>,
}

impl ProgramState {
    pub fn from_string(content: &str) -> Result<Self, Vec<Diagnostic>> {
        let field = AsciiField::from_string(content)?;
        let mut state = ProgramState {
            field,
            dots: Vec::new(),
            pending_dots: Vec::with_capacity(10),
            diagnostics: Vec::new(),
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
                if let Token::Dot = token.inner() {
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
            }
        }
    }

    fn interpret(&mut self) {
        for dot in self.dots.iter_mut() {
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
                _ => {
                    let lookahead = self.field.at(dot.position());
                    dot.next(&event, &lookahead)
                }
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
    }

    pub fn get_dots(&self) -> &[DotMachine] {
        &self.dots
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
