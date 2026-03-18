use std::io::{self, Write};

use asciidots_core::{
    interpreter::{DotsInterpreter, InterpreterCommand, InterpreterResult},
    parser::{LexError, Lexer},
};
use miette::{Diagnostic, IntoDiagnostic, NamedSource, SourceCode};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
#[error("Lex Error")]
pub struct LexDiagnostics {
    #[related]
    diagnostics: Vec<LexError>,
}

pub fn check(content: &str) -> miette::Result<()> {
    let lexer: Lexer = Lexer::new(content);

    match lexer.run() {
        Ok(_) => Ok(()),
        // TODO: Make better error wrapper
        Err(e) => Err(e),
    }
}

pub fn run(content: &str) -> miette::Result<()> {
    let mut interpreter = DotsInterpreter::from_string(content)
        .or_else(|e| Err(e.with_source_code(NamedSource::new("Source", content.to_string()))))?;

    'outer: while interpreter.num_dots() > 0 {
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
