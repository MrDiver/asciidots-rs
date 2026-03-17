use std::io::{self, Write};

use asciidots_core::{
    interpreter::{DotsInterpreter, InterpreterCommand, InterpreterResult},
    parser::{LexErrors, Lexer},
};
use miette::{IntoDiagnostic, NamedSource};

pub fn check(content: &str) -> Result<(), LexErrors> {
    let mut lexer: Lexer = content.into();
    lexer.run();

    match lexer.finish() {
        Ok(_) => Ok(()),
        Err(e) => Err(LexErrors {
            _src: NamedSource::new("Some Dots", content.to_string()),
            _errors: e,
        }),
    }
}

pub fn run(content: &str) -> miette::Result<()> {
    let mut interpreter = DotsInterpreter::from_string(content).map_err(|e| LexErrors {
        _src: NamedSource::new("Some Dots", content.to_string()),
        _errors: e,
    })?;

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
