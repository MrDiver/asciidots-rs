use std::io::{self, Write};

use asciidots_core::{
    display::DisplayBackend,
    interpreter::{DotsInterpreter, InterpreterCommand, InterpreterResult},
    linker,
    native::{NativeFunction, nativelib::default_native_library_for},
    parser::LexError,
};
use asciidots_display_minifb::MinifbDisplayBackend;
use asciidots_graph::ir::graphjson::graph_to_json;
use asciidots_graph::ir::graphsvg::graph_to_svg;
use asciidots_graph::ir::graphviz::to_graphviz;
use asciidots_graph::{
    compiler::tracer::Tracer, ir::dot::GraphDot, runtime::executor::GraphExecutor,
};
use miette::{Diagnostic, IntoDiagnostic};
use rustc_hash::FxHashMap;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
#[error("Lex Error")]
pub struct LexDiagnostics {
    #[related]
    diagnostics: Vec<LexError>,
}

pub enum WriteType {
    Graphviz,
    SVG,
    Json,
}

pub fn graph_file(path: &str, t: WriteType) -> miette::Result<String> {
    let native_registry: FxHashMap<String, Box<dyn NativeFunction<GraphDot>>> =
        default_native_library_for::<GraphDot>()
            .into_iter()
            .map(|f| (f.name().to_string(), f))
            .collect();
    let available = native_registry.keys().cloned().collect();
    let linked = linker::link_file(path.as_ref(), &available)?;
    let mut tracer = Tracer::new(linked.module);
    tracer.trace();
    let graph = tracer.into_graph();
    let res = match t {
        WriteType::Graphviz => to_graphviz(&graph),
        WriteType::SVG => graph_to_svg(&graph),
        WriteType::Json => graph_to_json(&graph),
    };
    Ok(res)
}

pub fn graph_run_file(path: &str, use_minifb: bool) -> miette::Result<()> {
    let native_registry: FxHashMap<String, Box<dyn NativeFunction<GraphDot>>> =
        default_native_library_for::<GraphDot>()
            .into_iter()
            .map(|f| (f.name().to_string(), f))
            .collect();
    let available = native_registry.keys().cloned().collect();
    let linked = linker::link_file(path.as_ref(), &available)?;
    let mut tracer = Tracer::new(linked.module);
    tracer.trace();
    let max_pending = tracer.get_max_pending();
    let graph = tracer.into_graph();

    let mut executor = GraphExecutor::new(graph, native_registry, Some(max_pending));
    let mut display_backends: FxHashMap<u32, Box<dyn DisplayBackend>> = FxHashMap::default();

    'outer: while !executor.is_finished() {
        let results = executor.step();
        let mut commands = Vec::new();
        for result in results {
            match result {
                InterpreterResult::PrintChar(c) => print!("{}", c),
                InterpreterResult::PrintLn(s) => println!("{}", s),
                InterpreterResult::InputRequest(id) => {
                    let mut stdout = io::stdout();
                    print!("Input Dot({}): ", id);
                    stdout.flush().into_diagnostic()?;
                    let mut buffer = String::new();
                    io::stdin().read_line(&mut buffer).into_diagnostic()?;
                    let value: f64 = buffer.trim().parse().into_diagnostic()?;
                    commands.push(InterpreterCommand::InputAnswer(*id, value));
                }
                InterpreterResult::Display {
                    display_id,
                    command,
                } => {
                    if use_minifb {
                        let backend = display_backends
                            .entry(*display_id)
                            .or_insert_with(|| Box::new(MinifbDisplayBackend::new()));
                        backend.handle(command.clone());
                    }
                }
                InterpreterResult::EndProgram => break 'outer,
            }
        }
        for command in commands {
            executor.handle_command(command);
        }
    }
    println!("Finished");
    Ok(())
}

pub fn check_file(path: &str) -> miette::Result<()> {
    DotsInterpreter::from_file(path)?;
    Ok(())
}

pub fn run_file(path: &str, use_minifb: bool) -> miette::Result<()> {
    let mut interpreter = DotsInterpreter::from_file(path)?;
    let mut display_backends: FxHashMap<u32, Box<dyn DisplayBackend>> = FxHashMap::default();

    'outer: while !interpreter.is_finished() {
        let commands = interpreter.step().unwrap();
        let mut interpreter_commands = Vec::new();
        for command in commands {
            match command {
                InterpreterResult::PrintChar(s) => print!("{}", s),
                InterpreterResult::PrintLn(s) => println!("{}", s),
                InterpreterResult::InputRequest(uuid) => {
                    let mut stdout = io::stdout();
                    print!("Input Dot({}): ", uuid);
                    stdout.flush().into_diagnostic()?;

                    let mut buffer = String::new();
                    let stdin = io::stdin();
                    stdin.read_line(&mut buffer).into_diagnostic()?;
                    let value: f64 = buffer.trim().parse().into_diagnostic()?;
                    interpreter_commands.push(InterpreterCommand::InputAnswer(*uuid, value));
                }
                InterpreterResult::Display {
                    display_id,
                    command,
                } => {
                    if use_minifb {
                        let backend = display_backends
                            .entry(*display_id)
                            .or_insert_with(|| Box::new(MinifbDisplayBackend::new()));
                        backend.handle(command.clone());
                    }
                }
                InterpreterResult::EndProgram => break 'outer,
            }
        }
        for command in interpreter_commands {
            interpreter.handle_command(command);
        }
    }
    println!("Finished");

    Ok(())
}

#[cfg(feature = "runtime-metrics")]
pub fn inspect_file(path: &str, every: usize, max_steps: Option<usize>) -> miette::Result<()> {
    let mut interpreter = DotsInterpreter::from_file(path)?;
    let every = every.max(1);

    println!(
        "Inspecting '{}' (every {} steps{})",
        path,
        every,
        max_steps
            .map(|steps| format!(", max {steps} steps"))
            .unwrap_or_default()
    );

    'outer: while !interpreter.is_finished() {
        let commands = interpreter.step().unwrap();
        let mut interpreter_commands = Vec::new();
        for command in commands {
            match command {
                InterpreterResult::PrintChar(s) => print!("{}", s),
                InterpreterResult::PrintLn(s) => println!("{}", s),
                InterpreterResult::InputRequest(uuid) => {
                    let mut stdout = io::stdout();
                    print!("Input Dot({}): ", uuid);
                    stdout.flush().into_diagnostic()?;

                    let mut buffer = String::new();
                    let stdin = io::stdin();
                    stdin.read_line(&mut buffer).into_diagnostic()?;
                    let value: f64 = buffer.trim().parse().into_diagnostic()?;
                    interpreter_commands.push(InterpreterCommand::InputAnswer(*uuid, value));
                }
                InterpreterResult::Display { .. } => {} // display not shown in inspect mode
                InterpreterResult::EndProgram => break 'outer,
            }
        }
        for command in interpreter_commands {
            interpreter.handle_command(command);
        }

        let metrics = interpreter.runtime_metrics();
        if metrics.step_count % every as u64 == 0 {
            println!(
                "[step={}] dots={} pending={} native_ret={} op_waiting={} peaks(dots={}, pending={}, native_ret={}, op_waiting={})",
                metrics.step_count,
                metrics.dots_len,
                metrics.pending_len,
                metrics.native_returned_len,
                metrics.operator_waiting_len,
                metrics.peak_dots_len,
                metrics.peak_pending_len,
                metrics.peak_native_returned_len,
                metrics.peak_operator_waiting_len,
            );
        }

        if let Some(max_steps) = max_steps
            && metrics.step_count >= max_steps as u64
        {
            println!("Reached max steps ({max_steps}), stopping inspect run");
            break;
        }
    }

    let metrics = interpreter.runtime_metrics();
    println!(
        "Inspect done: steps={} dots={} pending={} native_ret={} op_waiting={} caps(dots={}, pending={}, native_ret={}) peaks(dots={}, pending={}, native_ret={}, op_waiting={})",
        metrics.step_count,
        metrics.dots_len,
        metrics.pending_len,
        metrics.native_returned_len,
        metrics.operator_waiting_len,
        metrics.dots_capacity,
        metrics.pending_capacity,
        metrics.native_returned_capacity,
        metrics.peak_dots_len,
        metrics.peak_pending_len,
        metrics.peak_native_returned_len,
        metrics.peak_operator_waiting_len,
    );

    Ok(())
}

#[cfg(not(feature = "runtime-metrics"))]
pub fn inspect_file(_path: &str, _every: usize, _max_steps: Option<usize>) -> miette::Result<()> {
    Err(miette::miette!(
        "inspect requires runtime metrics support. Rebuild asciidots-cli with --features runtime-metrics"
    ))
}
