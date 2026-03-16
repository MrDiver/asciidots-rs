use miette::SourceSpan;
use rustc_hash::FxHashMap;

use crate::{
    parser::{LexError, SpanToken, Token},
    primitives::{Direction, Position},
    state_machine::{DotMachine, MachineEvent, RuntimeDiagnostic},
};

use super::{field::AsciiField, operators::OperatorQueues};

#[derive(Debug, Clone)]
pub enum InterpreterResult {
    Print(String),
    PrintLn(String),
    InputRequest(usize),
    EndProgram,
}

#[derive(Debug, Clone)]
pub enum InterpreterCommand {
    InputAnswer(usize, f64),
}

pub struct DotsInterpreter {
    field: AsciiField,
    dots: Vec<DotMachine>,
    pending_dots: Vec<DotMachine>,
    diagnostics: Vec<RuntimeDiagnostic>,
    operator_map: FxHashMap<(usize, usize), OperatorQueues>,
    results: Vec<InterpreterResult>,
}

impl DotsInterpreter {
    pub fn from_string(content: &str) -> Result<Self, Vec<LexError>> {
        let field = AsciiField::from_string(content)?;
        let mut state = DotsInterpreter {
            field,
            dots: Vec::new(),
            pending_dots: Vec::with_capacity(10),
            diagnostics: Vec::new(),
            operator_map: FxHashMap::default(),
            results: Vec::with_capacity(10),
        };
        state.initalize();
        Ok(state)
    }

    pub fn get_field(&self) -> &AsciiField {
        &self.field
    }

    fn push_print(results: &mut Vec<InterpreterResult>, value: String) {
        if let Some(InterpreterResult::Print(existing)) = results.last_mut() {
            existing.push_str(&value);
        } else {
            results.push(InterpreterResult::Print(value));
        }
    }

    fn push_print_char(results: &mut Vec<InterpreterResult>, value: char) {
        if let Some(InterpreterResult::Print(existing)) = results.last_mut() {
            existing.push(value);
        } else {
            let mut output = String::with_capacity(1);
            output.push(value);
            results.push(InterpreterResult::Print(output));
        }
    }

    fn initalize(&mut self) {
        for token in self.field.get_field() {
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
                            break;
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

    fn interpret(&mut self) {
        let mut i = 0;
        let mut repeat_iteration = false;
        while i < self.dots.len() {
            let dot = &mut self.dots[i];
            let token = self.field.at_spanned(dot.position());
            let event = match dot.run(&token, &mut self.pending_dots) {
                Ok(e) => e,
                Err(d) => {
                    self.diagnostics.push(d);
                    MachineEvent::EndProgram
                }
            };
            match event {
                MachineEvent::EndProgram => {
                    self.results.push(InterpreterResult::EndProgram);
                }
                MachineEvent::HitOperator => {
                    let mut dot = self.dots.swap_remove(i);
                    let pos = dot.position();
                    dot.next(
                        &event,
                        &SpanToken::new(Token::Empty, pos.x as usize, pos.y as usize, ' ', 0),
                    );
                    let opqueue = self
                        .operator_map
                        .get_mut(&(dot.position().into()))
                        .unwrap_or_else(|| unreachable!());
                    opqueue.push(dot);
                    if let Some(dot) = opqueue.get_result() {
                        self.dots.push(dot)
                    }
                    repeat_iteration = true;
                }
                MachineEvent::PrintLn(s) => {
                    self.results.push(InterpreterResult::PrintLn(s));
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                MachineEvent::PrintChar(c) => {
                    Self::push_print_char(&mut self.results, c);
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                MachineEvent::Print(s) => {
                    Self::push_print(&mut self.results, s);
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                MachineEvent::AssertionFailed { expected, found } => {
                    self.diagnostics.push(RuntimeDiagnostic::assertion_failed(
                        expected,
                        found,
                        (self.field.at_spanned(dot.position()).offset(), 1).into(),
                    ))
                }
                _ => {
                    let lookahead = self.field.at_spanned(dot.position());
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

    pub fn step(&mut self) -> Result<&[InterpreterResult], &[RuntimeDiagnostic]> {
        self.results.clear();
        self.interpret();
        Self::cleanup_dead_dots(&mut self.dots);
        self.dots.append(&mut self.pending_dots);

        if !self.diagnostics.is_empty() {
            Err(&self.diagnostics)
        } else {
            Ok(&self.results)
        }
    }

    pub fn num_dots(&self) -> usize {
        self.dots.len()
    }

    pub fn get_dots(&self) -> &[DotMachine] {
        &self.dots
    }

    pub fn queues(&self) -> &FxHashMap<(usize, usize), OperatorQueues> {
        &self.operator_map
    }
}

impl std::fmt::Debug for DotsInterpreter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Tokens: {:?}",
            self.field
                .get_field()
                .iter()
                .map(|token| token.inner())
                .collect::<Vec<_>>()
        )?;
        writeln!(f, "Dots:")?;
        for dot in &self.dots {
            writeln!(f, "{}", dot)?;
        }
        Ok(())
    }
}
