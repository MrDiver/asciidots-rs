use std::path::Path;

use log::trace;
use rustc_hash::FxHashMap;

use crate::{
    linker,
    parser::{AsciiCell, DotsModule, Lexer, Parser, SpanToken},
    primitives::{Direction, Orientation, Position},
    state_machine::{DotMachine, MachineEvent, MachineState, RuntimeDiagnostic},
};

use super::{field::AsciiField, operators::OperatorQueues};

#[derive(Debug, Clone)]
pub enum InterpreterResult {
    PrintChar(char),
    PrintLn(String),
    InputRequest(u64),
    EndProgram,
}

#[derive(Debug, Clone)]
pub enum InterpreterCommand {
    InputAnswer(u64, f64),
}
type FunDefMap = FxHashMap<String, Position>;

#[derive(Debug)]
pub struct DotsInterpreter {
    field: AsciiField,
    functions: FunDefMap,
    dots: Vec<DotMachine>,
    pending_dots: Vec<DotMachine>,
    diagnostics: Vec<RuntimeDiagnostic>,
    operator_map: FxHashMap<(usize, usize), OperatorQueues>,
    results: Vec<InterpreterResult>,
}

impl DotsInterpreter {

    pub fn from_file(path: impl AsRef<Path>) -> miette::Result<Self> {
        let module = linker::link_file(path.as_ref())?;
        Ok(Self::new(module))
    }

    pub fn new(module: DotsModule) -> Self {
        let (field, functions) = module.unpack();
        let mut state = DotsInterpreter {
            field,
            functions: functions.iter().map(|f| f.clone().unpack()).collect(),
            dots: Vec::new(),
            pending_dots: Vec::with_capacity(10),
            diagnostics: Vec::new(),
            operator_map: FxHashMap::default(),
            results: Vec::with_capacity(10),
        };
        state.initialize();
        state
    }

    pub fn get_field(&self) -> &AsciiField {
        &self.field
    }

    fn initialize(&mut self) {
        for (x, y, token) in self.field.into_iter() {
            let position: Position = (x, y).into();
            match *token.inner() {
                AsciiCell::Dot => {
                    for direction in &[
                        Direction::Up,
                        Direction::Right,
                        Direction::Down,
                        Direction::Left,
                    ] {
                        let offsetpos = position + (*direction).into();
                        match self.field.at(&offsetpos) {
                            AsciiCell::Pipe(orientation)
                                if *orientation == direction.orientation() =>
                            {
                                self.dots
                                    .push(DotMachine::new(*direction, position.x, position.y));
                                break;
                            }
                            AsciiCell::CrossPipe | AsciiCell::Mirror(_) => {
                                self.dots
                                    .push(DotMachine::new(*direction, position.x, position.y));
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                AsciiCell::Operator { orientation, repr } => {
                    self.operator_map
                        .insert((x, y), OperatorQueues::new(orientation, repr, false));
                }
                AsciiCell::Conditional(negated) => {
                    self.operator_map.insert(
                        (x, y),
                        OperatorQueues::new(Orientation::Horizontal, '~', negated),
                    );
                }
                _ => {}
            }
        }
    }

    fn interpret(&mut self) {
        let default_token = SpanToken::new(AsciiCell::Empty, ' ', (0, 1).into());
        let mut i = 0;
        let mut repeat_iteration = false;
        while i < self.dots.len() {
            let dot = &mut self.dots[i];
            let token = self.field.at_spanned(dot.position());
            let event = match dot.process(&token, &mut self.pending_dots) {
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
                    dot.next(&event, &default_token);
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
                MachineEvent::Input(uuid) => {
                    self.results.push(InterpreterResult::InputRequest(uuid));
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                MachineEvent::PrintLn(s) => {
                    self.results.push(InterpreterResult::PrintLn(s));
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                MachineEvent::PrintChar(c) => {
                    self.results.push(InterpreterResult::PrintChar(c));
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                MachineEvent::AssertionFailed { expected, found } => {
                    self.diagnostics.push(RuntimeDiagnostic::assertion_failed(
                        expected,
                        found,
                        self.field.at_spanned(dot.position()).span(),
                    ))
                }
                MachineEvent::Call(_, name) => {
                    // TODO: Implement Native function calls via Lua extensions
                    let fun = self.functions.get(&name);
                    if let Some(fun) = fun {
                        let pos: Position = (fun.x, fun.y).into();
                        let current = self.field.at_spanned(&pos);
                        dot.dot_mut().call(pos).step();
                        dot.next(&MachineEvent::Nothing, current)
                    } else {
                        let current = self.field.at_spanned(dot.position());
                        self.diagnostics
                            .push(RuntimeDiagnostic::MissingFunctionDefinition {
                                name,
                                span: current.span(),
                            });
                    }
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

    pub fn cleanup_dead_dots(&mut self) {
        let mut i = 0;
        while i < self.dots.len() {
            if self.dots[i].is_dead() {
                self.dots.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn handle_command(&mut self, command: InterpreterCommand) {
        trace!("--- Handle Commands ---");
        match command {
            InterpreterCommand::InputAnswer(uuid, value) => {
                if let Some(machine) = self.dots.iter_mut().find(|dot| dot.uuid() == uuid) {
                    if let MachineState::Set(s) = machine.state_mut() {
                        s.receive_input(value);
                    }
                }
            }
        }
    }

    pub fn step(&mut self) -> Result<&[InterpreterResult], &[RuntimeDiagnostic]> {
        trace!("--- Step ---");
        self.results.clear();
        self.interpret();
        self.cleanup_dead_dots();
        self.handle_pending();

        if !self.diagnostics.is_empty() {
            Err(&self.diagnostics)
        } else {
            trace!("diagnostics: {:?}", &self.results);
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

    fn handle_pending(&mut self) {
        for dot in &mut self.pending_dots {
            let pos = dot.position();
            let tok = self.field.at_spanned(pos);
            dot.next(&MachineEvent::Nothing, tok);
        }
        self.dots.append(&mut self.pending_dots);
    }
}
