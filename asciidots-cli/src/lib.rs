use asciidots_core::parser::{LexErrors, Lexer};

pub fn check(content: String) -> Result<(), LexErrors> {
    let mut lexer: Lexer = content.into();
    lexer.run();
    lexer.finish().map(|_| ())
}
