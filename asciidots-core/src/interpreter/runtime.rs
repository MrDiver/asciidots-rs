use std::collections::BTreeSet;
use std::path::Path;

use log::trace;
use rustc_hash::FxHashMap;

use crate::{
    linker,
    native::{NativeFunction, nativelib},
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

enum FunctionDefinition {
    Ascii(Position),
    Native(String),
}

const EMPTY_DOT_STEP_LIMIT: usize = 1000;

#[cfg(feature = "runtime-metrics")]
#[derive(Debug, Clone, Default)]
pub struct RuntimeMetrics {
    pub step_count: u64,
    pub dots_len: usize,
    pub dots_capacity: usize,
    pub pending_len: usize,
    pub pending_capacity: usize,
    pub native_returned_len: usize,
    pub native_returned_capacity: usize,
    pub operator_waiting_len: usize,
    pub results_len: usize,
    pub diagnostics_len: usize,
    pub peak_dots_len: usize,
    pub peak_pending_len: usize,
    pub peak_native_returned_len: usize,
    pub peak_operator_waiting_len: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ExecutionBackend {
    #[default]
    SingleThread,
}

pub struct DotsInterpreterBuilder {
    native_registry: FxHashMap<String, Box<dyn NativeFunction>>,
    execution_backend: ExecutionBackend,
}

impl DotsInterpreterBuilder {
    pub fn new() -> Self {
        let mut builder = Self {
            native_registry: FxHashMap::default(),
            execution_backend: ExecutionBackend::SingleThread,
        };

        for native_function in nativelib::default_native_library() {
            builder.register_native_function(native_function);
        }

        builder
    }

    pub fn register_native_function(&mut self, function: Box<dyn NativeFunction>) -> &mut Self {
        let name = function.name().to_string();
        self.native_registry.insert(name, function);
        self
    }

    pub fn execution_backend(&mut self, backend: ExecutionBackend) -> &mut Self {
        self.execution_backend = backend;
        self
    }

    pub fn build_from_file(self, path: impl AsRef<Path>) -> miette::Result<DotsInterpreter> {
        let available_native_functions: BTreeSet<String> =
            self.native_registry.keys().cloned().collect();
        let linked = linker::link_file(path.as_ref(), &available_native_functions)?;
        Ok(DotsInterpreter::from_linked(
            linked.module,
            linked.native_imports,
            self.native_registry,
            self.execution_backend,
        ))
    }
}

impl Default for DotsInterpreterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DotsInterpreter {
    field: AsciiField,
    functions: FxHashMap<String, FunctionDefinition>,
    native_registry: FxHashMap<String, Box<dyn NativeFunction>>,
    execution_backend: ExecutionBackend,
    native_returned_dots: Vec<DotMachine>,
    dots: Vec<DotMachine>,
    pending_dots: Vec<DotMachine>,
    diagnostics: Vec<RuntimeDiagnostic>,
    operator_map: FxHashMap<(usize, usize), OperatorQueues>,
    results: Vec<InterpreterResult>,
    finished: bool,
    empty_dot_steps: usize,
    #[cfg(feature = "runtime-metrics")]
    runtime_metrics: RuntimeMetrics,
}

impl DotsInterpreter {
    pub fn from_string(source: &str) -> miette::Result<Self> {
        let lex = Lexer::new(source);
        let source = lex.run()?;
        let parser = Parser::new(source);
        let module = parser.run();
        Ok(Self::new(module?))
    }

    pub fn from_file(path: impl AsRef<Path>) -> miette::Result<Self> {
        DotsInterpreterBuilder::new().build_from_file(path)
    }

    pub fn new(module: DotsModule) -> Self {
        Self::from_linked(
            module,
            FxHashMap::default(),
            FxHashMap::default(),
            ExecutionBackend::SingleThread,
        )
    }

    fn from_linked(
        module: DotsModule,
        native_imports: impl IntoIterator<Item = (String, String)>,
        native_registry: FxHashMap<String, Box<dyn NativeFunction>>,
        execution_backend: ExecutionBackend,
    ) -> Self {
        let (field, functions) = module.unpack();
        let mut function_map: FxHashMap<String, FunctionDefinition> = functions
            .iter()
            .map(|f| {
                let (name, pos) = f.clone().unpack();
                (name, FunctionDefinition::Ascii(pos))
            })
            .collect();

        for (exposed_name, source_name) in native_imports {
            function_map.insert(exposed_name, FunctionDefinition::Native(source_name));
        }

        let mut state = DotsInterpreter {
            field,
            functions: function_map,
            native_registry,
            execution_backend,
            native_returned_dots: Vec::with_capacity(16),
            dots: Vec::new(),
            pending_dots: Vec::with_capacity(10),
            diagnostics: Vec::new(),
            operator_map: FxHashMap::default(),
            results: Vec::with_capacity(10),
            finished: false,
            empty_dot_steps: 0,
            #[cfg(feature = "runtime-metrics")]
            runtime_metrics: RuntimeMetrics::default(),
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

    fn collect_native_returns(&mut self) {
        self.native_returned_dots.clear();
        {
            let native_registry = &mut self.native_registry;
            let native_returned_dots = &mut self.native_returned_dots;
            for native_function in native_registry.values_mut() {
                native_function.tick_into(native_returned_dots);
            }
        }
    }

    fn merge_native_returns_into_dots(&mut self) {
        let mut native_returned_dots = std::mem::take(&mut self.native_returned_dots);
        for mut dot in native_returned_dots.drain(..) {
            if dot.dot_mut().ret().is_err() {
                continue;
            }
            dot.dot_mut().step();
            let lookahead = self.field.at_spanned(dot.position());
            dot.next(&MachineEvent::Nothing, lookahead);
            self.dots.push(dot);
        }
        self.native_returned_dots = native_returned_dots;
    }

    fn process_active_dots_single_thread(&mut self) {
        let default_token = SpanToken::new(AsciiCell::Empty, ' ', (0, 1).into());
        let mut i = 0;
        let mut repeat_iteration = false;
        while i < self.dots.len() {
            let dot = &mut self.dots[i];
            let token = self.field.at_spanned(dot.position());
            let event = match dot.process(token, &mut self.pending_dots) {
                Ok(e) => e,
                Err(d) => {
                    self.diagnostics.push(d);
                    MachineEvent::EndProgram
                }
            };
            match event {
                MachineEvent::EndProgram => {
                    self.results.push(InterpreterResult::EndProgram);
                    self.finished = true;
                    self.dots.clear();
                    self.pending_dots.clear();
                    break;
                }
                MachineEvent::HitOperator => {
                    let mut dot = self.dots.swap_remove(i);
                    dot.next(&event, &default_token);
                    let opqueue = self
                        .operator_map
                        .get_mut(&(dot.position().into()))
                        .unwrap_or_else(|| unreachable!());
                    if let Some(dot) = opqueue.push_and_get_result(dot) {
                        self.dots.push(dot)
                    }
                    repeat_iteration = true;
                }
                MachineEvent::Input(uuid) => {
                    self.results.push(InterpreterResult::InputRequest(uuid));
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, lookahead)
                }
                MachineEvent::PrintLn(s) => {
                    self.results.push(InterpreterResult::PrintLn(s));
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, lookahead)
                }
                MachineEvent::PrintChar(c) => {
                    self.results.push(InterpreterResult::PrintChar(c));
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&MachineEvent::Nothing, lookahead)
                }
                MachineEvent::AssertionFailed { expected, found } => {
                    self.diagnostics.push(RuntimeDiagnostic::assertion_failed(
                        expected,
                        found,
                        self.field.at_spanned(dot.position()).span(),
                    ));
                    self.finished = true;
                    self.dots.clear();
                    self.pending_dots.clear();
                    break;
                }
                MachineEvent::Call(_, name) => {
                    let fun = self.functions.get(&name);
                    match fun {
                        Some(FunctionDefinition::Ascii(fun)) => {
                            let pos: Position = (fun.x, fun.y).into();
                            let current = self.field.at_spanned(&pos);
                            dot.dot_mut().call(pos).step();
                            dot.next(&MachineEvent::Nothing, current)
                        }
                        Some(FunctionDefinition::Native(native_name)) => {
                            let mut dot = self.dots.swap_remove(i);
                            let call_site = *dot.position();
                            dot.dot_mut().call(call_site);
                            if let Some(native_fun) = self.native_registry.get_mut(native_name) {
                                native_fun.dot_enter(dot);
                                repeat_iteration = true;
                            } else {
                                self.diagnostics
                                    .push(RuntimeDiagnostic::MissingFunctionDefinition {
                                        name,
                                        span: self.field.at_spanned(dot.position()).span(),
                                    });
                            }
                        }
                        None => {
                            let current = self.field.at_spanned(dot.position());
                            self.diagnostics
                                .push(RuntimeDiagnostic::MissingFunctionDefinition {
                                    name,
                                    span: current.span(),
                                });
                        }
                    }
                }
                _ => {
                    let lookahead = self.field.at_spanned(dot.position());
                    dot.next(&event, lookahead)
                }
            }
            if !repeat_iteration {
                i += 1;
            } else {
                repeat_iteration = false;
            }
        }
    }

    fn interpret(&mut self) {
        if self.finished {
            return;
        }

        self.collect_native_returns();
        self.merge_native_returns_into_dots();
        self.process_active_dots();
    }

    fn process_active_dots(&mut self) {
        match self.execution_backend {
            ExecutionBackend::SingleThread => self.process_active_dots_single_thread(),
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
                if let Some(machine) = self.dots.iter_mut().find(|dot| dot.uuid() == uuid)
                    && let MachineState::Set(s) = machine.state_mut()
                {
                    s.receive_input(value);
                }
            }
        }
    }

    pub fn step(&mut self) -> Result<&[InterpreterResult], &[RuntimeDiagnostic]> {
        trace!("--- Step ---");
        self.results.clear();

        if self.finished {
            return Ok(&self.results);
        }

        self.interpret();
        self.finalize_step();

        #[cfg(feature = "runtime-metrics")]
        self.update_runtime_metrics();

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

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn get_dots(&self) -> &[DotMachine] {
        &self.dots
    }

    pub fn queues(&self) -> &FxHashMap<(usize, usize), OperatorQueues> {
        &self.operator_map
    }

    #[cfg(feature = "runtime-metrics")]
    pub fn runtime_metrics(&self) -> &RuntimeMetrics {
        &self.runtime_metrics
    }

    fn handle_pending(&mut self) {
        for dot in &mut self.pending_dots {
            let pos = dot.position();
            let tok = self.field.at_spanned(pos);
            dot.next(&MachineEvent::Nothing, tok);
        }
        self.dots.append(&mut self.pending_dots);
    }

    fn finalize_step(&mut self) {
        self.cleanup_dead_dots();
        self.handle_pending();

        if self.finished {
            return;
        }

        if self.dots.is_empty() {
            self.empty_dot_steps += 1;
            if self.empty_dot_steps >= EMPTY_DOT_STEP_LIMIT {
                self.finished = true;
                self.results.push(InterpreterResult::PrintLn(
                    "Warning: nothing changed for 1000 steps, stopping program".to_string(),
                ));
            }
        } else {
            self.empty_dot_steps = 0;
        }
    }

    #[cfg(feature = "runtime-metrics")]
    fn update_runtime_metrics(&mut self) {
        self.runtime_metrics.step_count += 1;
        self.runtime_metrics.dots_len = self.dots.len();
        self.runtime_metrics.dots_capacity = self.dots.capacity();
        self.runtime_metrics.pending_len = self.pending_dots.len();
        self.runtime_metrics.pending_capacity = self.pending_dots.capacity();
        self.runtime_metrics.native_returned_len = self.native_returned_dots.len();
        self.runtime_metrics.native_returned_capacity = self.native_returned_dots.capacity();
        self.runtime_metrics.operator_waiting_len = self
            .operator_map
            .values()
            .map(OperatorQueues::waiting_len)
            .sum();
        self.runtime_metrics.results_len = self.results.len();
        self.runtime_metrics.diagnostics_len = self.diagnostics.len();

        self.runtime_metrics.peak_dots_len = self
            .runtime_metrics
            .peak_dots_len
            .max(self.runtime_metrics.dots_len);
        self.runtime_metrics.peak_pending_len = self
            .runtime_metrics
            .peak_pending_len
            .max(self.runtime_metrics.pending_len);
        self.runtime_metrics.peak_native_returned_len = self
            .runtime_metrics
            .peak_native_returned_len
            .max(self.runtime_metrics.native_returned_len);
        self.runtime_metrics.peak_operator_waiting_len = self
            .runtime_metrics
            .peak_operator_waiting_len
            .max(self.runtime_metrics.operator_waiting_len);
    }
}
