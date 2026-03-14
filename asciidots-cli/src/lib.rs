use asciidots_core::{
    interpreter::{DotsInterpreter, InterpreterResult},
    parser::{LexError, LexErrors, Lexer},
};
use miette::NamedSource;

pub fn check(content: &str) -> Result<(), LexErrors> {
    let mut lexer: Lexer = content.into();
    lexer.run();

    match lexer.finish() {
        Ok(_) => Ok(()),
        Err(e) => Err(LexErrors {
            src: NamedSource::new("Some Dots", content.to_string()),
            errors: e,
        }),
    }
}

pub fn run(content: &str) -> miette::Result<()> {
    let mut interpreter = DotsInterpreter::from_string(content).map_err(|e| LexErrors {
        src: NamedSource::new("Some Dots", content.to_string()),
        errors: e,
    })?;

    'outer: while interpreter.num_dots() > 0 {
        let commands = interpreter.step().unwrap();
        for command in commands {
            match command {
                InterpreterResult::Print(s) => print!("{}", s),
                InterpreterResult::PrintLn(s) => println!("{}", s),
                InterpreterResult::InputRequest(_) => todo!(),
                InterpreterResult::EndProgram => break 'outer,
            }
        }
    }
    println!("Finished");

    Ok(())
}
