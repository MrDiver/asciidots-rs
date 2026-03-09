pub mod dot;
pub mod interpreter;
pub mod parser;
pub mod state;
pub mod token;

use std::collections::HashMap;

use crossterm::style::Stylize;

use crate::{
    dot::{Dot, DotState, Position},
    parser::{Diagnostic, Lexer, Severity},
    token::{Direction, SpanToken, Spanned, Token},
};

pub type TokenField = HashMap<(usize, usize), SpanToken>;

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
        Ok(AsciiField { field: field })
    }

    pub fn get_field(&self) -> &TokenField {
        &self.field
    }

    pub fn at(&self, position: &Position) -> Token {
        let &Position { x, y } = position;
        self.field
            .get(&(x as usize, y as usize))
            .map(|x| *x.inner())
            .unwrap_or_else(|| Token::Empty)
    }
}

pub struct ProgramState {
    field: AsciiField,
    dots: Vec<Dot>,
    pending_dots: Vec<Dot>,
    diagnostics: Vec<Diagnostic>,
}

impl ProgramState {
    pub fn from_string(content: &str) -> Result<Self, Vec<Diagnostic>> {
        let field = AsciiField::from_string(content)?;
        Ok(ProgramState {
            field,
            dots: Vec::new(),
            pending_dots: Vec::with_capacity(10),
            diagnostics: Vec::new(),
        })
    }
    
    pub fn get_field(&self) -> &AsciiField {
        &self.field
    }

    pub fn initalize_dots(&mut self) {
        for ((x, y), token) in self.field.get_field().iter() {
            let position = Position::new(*x as i32, *y as i32);
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
                        let dot = Dot::new(*direction, position, 0.0);
                        self.dots.push(dot);
                        break; // Only one dot per position, so break after finding the first valid direction
                    }
                }
            }
        }
    }

    fn interpret(&mut self) {
        let mut program_ending = false;
        for dot in self.dots.iter_mut() {
            let token = self.field.at(&dot.position);
            if let Err(d) = interpreter::interpret(dot, &token, &mut self.pending_dots) {
                let Position { x, y } = dot.position;
                self.diagnostics
                    .push(Diagnostic::new(x as usize, y as usize, d, Severity::Error));
            }
            match dot.state {
                DotState::EndProgram => {
                    program_ending = true;
                    break;
                }
                DotState::Dead => {}
                _ => {}
            }
        }
        if program_ending {
            self.dots.clear();
        }
    }

    pub fn step(&mut self) -> Result<(), &[Diagnostic]> {
        // 1. Step all Dots
        self.interpret();

        // 2. Compaction of Dots (remove dead)
        let mut i = 0;
        while i < self.dots.len() {
            if self.dots[i].state == DotState::Dead {
                self.dots.swap_remove(i);
            } else {
                i += 1;
            }
        }
        // 3. Append new Dots
        self.dots.extend_from_slice(&self.pending_dots);

        // 4. Clear added dots from pending list
        self.pending_dots.clear();

        if self.diagnostics.len() > 0 {
            Err(&self.diagnostics)
        } else {
            Ok(())
        }
    }

    pub fn num_dots(&self) -> usize {
        self.dots.len()
    }
    
    pub fn get_dots(&self) -> &[Dot] {
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
                .map(|t| t.1.inner())
                .collect::<Vec<_>>()
        )?;
        writeln!(f, "Dots:")?;
        for dot in &self.dots {
            writeln!(
                f,
                "Dot at ({}, {}) facing {:?} with value {:?}",
                dot.position.x, dot.position.y, dot.direction, dot.value
            )?;
        }
        Ok(())
    }
}
