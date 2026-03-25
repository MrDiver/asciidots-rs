use std::io::{self, Write};

use asciidots_core::{
    interpreter::{DotsInterpreter, InterpreterCommand, InterpreterResult},
    parser::{LexError},
};
use miette::{Diagnostic, IntoDiagnostic};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
#[error("Lex Error")]
pub struct LexDiagnostics {
    #[related]
    diagnostics: Vec<LexError>,
}

pub fn check_file(path: &str) -> miette::Result<()> {
    DotsInterpreter::from_file(path)?;
    Ok(())
}

pub fn run_file(path: &str) -> miette::Result<()> {
    let mut interpreter = DotsInterpreter::from_file(path)?;

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
