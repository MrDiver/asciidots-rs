use std::collections::VecDeque;

use miette::{Diagnostic, SourceSpan};
use rustc_hash::FxHashMap;
use thiserror::Error;

use crate::{
    dot::Position,
    parser::{LexError, Lexer},
    state_machine::{DotMachine, MachineEvent},
    token::{Direction, Orientation, SpanToken, Token},
};

pub type TokenField = Vec<Vec<SpanToken>>;

#[derive(Debug)]
pub struct AsciiField {
    field: TokenField,
    offsets: Vec<Vec<usize>>,
    width: usize,
    height: usize,
}

impl AsciiField {
    pub fn from_string(content: &str) -> Result<Self, Vec<LexError>> {
        let mut lexer = Lexer::new(content.into());
        lexer.run();
        let offsets = lexer.offsets();
        let field = lexer.finish()?;
        let height = field.len();
        let width = field
            .iter()
            .map(|line| line.len())
            .max()
            .unwrap_or_default();
        Ok(AsciiField {
            field,
            offsets: offsets,
            height,
            width,
        })
    }

    pub fn get_field(&self) -> &TokenField {
        &self.field
    }

    pub fn at(&self, position: &Position) -> Token {
        let &Position { x, y } = position;
        if x < 0 || y < 0 {
            return Token::Empty;
        }
        self.field
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .map(|token| *token.inner())
            .unwrap_or_else(|| Token::Empty)
    }

    pub fn offset(&self, position: &Position) -> usize {
        self.offsets[position.y as usize][position.x as usize]
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

#[derive(Debug)]
pub struct OperatorQueues {
    orientation: Orientation,
    op: char,
    horizontal: VecDeque<DotMachine>,
    vertical: VecDeque<DotMachine>,
}

impl OperatorQueues {
    fn new(orientation: Orientation, op: char) -> Self {
        Self {
            orientation,
            op,
            horizontal: VecDeque::with_capacity(1),
            vertical: VecDeque::with_capacity(1),
        }
    }

    fn push(&mut self, dot: DotMachine) {
        match dot.direction().orientation() {
            Orientation::Horizontal => self.horizontal.push_back(dot),
            Orientation::Vertical => self.vertical.push_back(dot),
        }
    }

    fn get_result(&mut self) -> Option<DotMachine> {
        if self.horizontal.len() > 0 && self.vertical.len() > 0 {
            let (mut primary, secondary) = {
                let doth = self.horizontal.pop_front().unwrap();
                let dotv = self.vertical.pop_front().unwrap();
                if matches!(self.orientation, Orientation::Horizontal) {
                    (doth, dotv)
                } else {
                    (dotv, doth)
                }
            };

            let a = primary.value();
            let b = secondary.value();
            let new_val = match self.op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                _ => a,
            };
            let mut new_dot = primary.dot();
            new_dot.set_value(new_val);
            primary.replace_dot(new_dot);
            return Some(primary);
        }
        None
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("The Program died Unexpectedly")]
pub enum RuntimeError {
    None,
    #[error("The printing modifier encountered is invalid {}", char::from(*invalid))]
    #[diagnostic(help("Try: one of @, #, \"...\" or '...' "))]
    InvalidPrintingModifier {
        invalid: Token,
        #[label("this here")]
        span: SourceSpan,
    },
}

impl RuntimeError {
    pub fn invalid_printing_modifier(invalid: Token, span: SourceSpan) -> Self {
        RuntimeError::InvalidPrintingModifier { invalid, span }
    }
}

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
    diagnostics: Vec<RuntimeError>,
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

    fn initalize(&mut self) {
        for row in self.field.get_field() {
            for token in row {
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
                                break; // Only one dot per position, so break after finding the first valid direction
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
    }

    fn interpret(&mut self) {
        let mut i = 0;
        let mut repeat_iteration = false;
        while i < self.dots.len() {
            let dot = &mut self.dots[i];
            let token = &self.field.at(dot.position());
            let event = match dot.run(token, &mut self.pending_dots) {
                Ok(e) => e,
                Err(d) => {
                    let Position { x, y } = dot.position();
                    self.diagnostics.push(RuntimeError::None); // TODO: implement the error
                    // handling
                    MachineEvent::EndProgram
                }
            };
            match event {
                MachineEvent::EndProgram => {
                    self.results.push(InterpreterResult::EndProgram);
                }
                MachineEvent::HitOperator => {
                    let mut dot = self.dots.swap_remove(i);
                    dot.next(&event, &Token::Empty);
                    let opqueue = self
                        .operator_map
                        .get_mut(&(dot.position().into()))
                        .unwrap_or_else(|| unreachable!());
                    opqueue.push(dot);
                    // Add a result dot back to the running loop
                    if let Some(dot) = opqueue.get_result() {
                        self.dots.push(dot)
                    }

                    repeat_iteration = true;
                }
                MachineEvent::PrintLn(s) => {
                    self.results.push(InterpreterResult::PrintLn(s));
                    let lookahead = self.field.at(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                MachineEvent::Print(s) => {
                    self.results.push(InterpreterResult::Print(s));
                    let lookahead = self.field.at(dot.position());
                    dot.next(&MachineEvent::Nothing, &lookahead)
                }
                _ => {
                    let lookahead = self.field.at(dot.position());
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

    pub fn step(&mut self) -> Result<&[InterpreterResult], &[RuntimeError]> {
        self.results.clear();
        // 1. Step all Dots
        self.interpret();

        // 2. Compaction of Dots (remove dead)
        Self::cleanup_dead_dots(&mut self.dots);
        // 3. Append new Dots
        self.dots.extend_from_slice(&self.pending_dots);

        // 4. Clear added dots from pending list
        self.pending_dots.clear();

        if !self.diagnostics.is_empty() {
            Err(&self.diagnostics)
        } else {
            Ok(&self.results)
        }
    }

    pub fn num_dots(&self) -> usize {
        self.dots.len()
    } // TODO: Investigate if it needs counting the operator dots

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
                .flat_map(|row| row.iter().map(|token| token.inner()))
                .collect::<Vec<_>>()
        )?;
        writeln!(f, "Dots:")?;
        for dot in &self.dots {
            writeln!(f, "{}", dot)?;
        }
        Ok(())
    }
}
