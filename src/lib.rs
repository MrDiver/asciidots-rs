pub mod dot;
pub mod state;
pub mod state_machine;

use std::rc::Rc;

use crossterm::style::Stylize;

use crate::{
    dot::Dot,
    state::{Direction, DotState, Position},
};

pub struct AsciiField {
    field: Vec<Vec<char>>,
}

impl AsciiField {
    pub fn from_file(path: &str) -> Self {
        let contents = std::fs::read_to_string(path).expect("Failed to read file");
        let field: Vec<Vec<char>> = contents
            .lines()
            .map(|line| line.chars().collect())
            .collect();
        AsciiField { field: field }
    }

    pub fn get_field(&self) -> &Vec<Vec<char>> {
        &self.field
    }

    fn at(&self, position: &Position) -> Option<char> {
        if position.y >= 0 && (position.y as usize) < self.field.len() {
            let row = &self.field[position.y as usize];
            if position.x >= 0 && (position.x as usize) < row.len() {
                return Some(row[position.x as usize]);
            }
        }
        None
    }

    pub fn initalize_dots(&self) -> Vec<Dot> {
        let mut dots = Vec::new();
        for y in 0..self.field.len() {
            for x in 0..self.field[y].len() {
                let cell = self.field[y][x];
                let position = Position::new(x as i32, y as i32);
                if cell == '.' {
                    for direction in &[
                        Direction::Up,
                        Direction::Down,
                        Direction::Left,
                        Direction::Right,
                    ] {
                        let offsetpos = position + direction.offset();
                        if let Some(pipe) = self.at(&offsetpos)
                            && pipe == direction.pipe()
                        {
                            let dot = Dot::new(*direction, position, 0.0);
                            dots.push(dot);
                            break; // Only one dot per position, so break after finding the first valid direction
                        }
                    }
                }
            }
        }
        dots
    }

    pub fn interpret(&self, dots: &mut Vec<Dot>, pending_dots: &mut Vec<Dot>) {
        let mut program_ending = false;
        for dot in dots.iter_mut() {
            if let Some(cell) = self.at(&dot.position) {
                state_machine::interpret(dot, cell, pending_dots);
                match dot.state {
                    state::DotState::EndProgram => {
                        program_ending = true;
                        break;
                    }
                    state::DotState::Dead => {}
                    _ => {}
                }
            }
        }
        if program_ending {
            dots.clear()
        }
    }
}

pub struct ProgramState {
    field: AsciiField,
    dots: Vec<Dot>,
    pending_dots: Vec<Dot>,
}

impl ProgramState {
    pub fn from_file(path: &str) -> Self {
        let field = AsciiField::from_file(path);
        let dots = field.initalize_dots();
        ProgramState {
            field,
            dots,
            pending_dots: Vec::with_capacity(10),
        }
    }

    pub fn step(&mut self) {
        // 1. Step all Dots
        self.field.interpret(&mut self.dots, &mut self.pending_dots);

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
    }

    pub fn num_dots(&self) -> usize {
        self.dots.len()
    }
}

impl std::fmt::Debug for ProgramState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl std::fmt::Display for ProgramState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dot_positions: std::collections::HashSet<(i32, i32)> = self
            .dots
            .iter()
            .map(|d| (d.position.x, d.position.y))
            .collect();
        for (y, row) in self.field.get_field().iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let cell = format!("{}", cell);
                if dot_positions.contains(&(x as i32, y as i32)) {
                    write!(f, "{}", cell.red())?;
                } else {
                    write!(f, "{}", cell)?;
                }
            }
            write!(f, "\r\n")?;
        }
        Ok(())
    }
}
