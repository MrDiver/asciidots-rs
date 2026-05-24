use nom::AsChar;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

use crate::{
    interpreter::AsciiField,
    parser::{
        AsciiCell, MirrorKind, ParsingError, ParsingErrorCollection, SpanToken,
        ascii_cell::AsciiFunc,
        lexer::{DotsSource, PreambleStatement},
    },
    primitives::{Direction, Orientation, Position},
};

pub struct DotsModule {
    field: AsciiField,
    functions: Vec<AsciiFunc>,
    native_names: Vec<String>,
}

impl DotsModule {
    pub fn new(field: AsciiField, functions: Vec<AsciiFunc>, native_names: Vec<String>) -> Self {
        Self {
            field,
            functions,
            native_names,
        }
    }

    pub fn unpack(self) -> (AsciiField, Vec<AsciiFunc>, Vec<String>) {
        (self.field, self.functions, self.native_names)
    }
}

pub struct Parser {
    module: DotsSource,
}

impl Parser {
    pub fn new(module: DotsSource) -> Self {
        Self { module }
    }

    pub fn run(self) -> miette::Result<DotsModule> {
        let grid = self.module.grid();
        let preamble = self.module.preamble();
        let aliases: FxHashMap<String, String> = preamble
            .iter()
            .filter_map(|statement| match statement {
                PreambleStatement::Import {
                    name,
                    alias: Some(alias),
                    ..
                } => Some((alias.clone(), name.clone())),
                _ => None,
            })
            .collect();
        let warps: FxHashSet<char> = preamble
            .iter()
            .filter_map(|statement| match statement {
                PreambleStatement::Warps { warps, .. } => Some(warps),
                _ => None,
            })
            .flatten()
            .map(char::clone)
            .collect();
        let height = grid.len();
        let width = grid.iter().map(|line| line.len()).max().unwrap_or_default();
        let mut functiondefs = Vec::new();
        let mut field = AsciiField::new(width, height);

        let get_char = |x: usize, y: usize| {
            grid.get(y)
                .and_then(|line| line.get(x).map(|token| token.repr()))
        };
        let get_name = |x: usize, y: usize| {
            let mut i = x + 1;
            let mut c_opt = get_char(i, y);
            let mut acc = String::with_capacity(5);
            while let Some(c) = c_opt
                && !c.is_whitespace()
                && !c.is_ascii_punctuation()
                && !c.is_dec_digit()
            {
                acc.push(c);
                i += 1;
                c_opt = get_char(i, y);
            }
            if acc.is_empty() { None } else { Some(acc) }
        };

        let mut inside_comment = false;
        let mut inside_operator = false;
        let mut display_counter: u32 = 0;
        let mut operator_orientation = Orientation::Horizontal;
        let mut op_length = 0;
        let mut diagnostics = Vec::new();
        for (y, line) in grid.iter().enumerate() {
            for (x, token) in line.iter().enumerate() {
                let cell = match token.repr() {
                    c if inside_comment => {
                        if c == '`' {
                            inside_comment = false;
                            AsciiCell::Comment('`')
                        } else {
                            AsciiCell::Comment(c)
                        }
                    }
                    '`' => {
                        inside_comment = true;
                        AsciiCell::Comment('`')
                    }
                    '{' => {
                        inside_operator = true;
                        operator_orientation = Orientation::Horizontal;
                        AsciiCell::OperatorBegin(Orientation::Horizontal)
                    }
                    '[' => {
                        inside_operator = true;
                        operator_orientation = Orientation::Vertical;
                        AsciiCell::OperatorBegin(Orientation::Vertical)
                    }
                    '}' => {
                        if !inside_operator {
                            diagnostics.push(ParsingError::MissingOpeningBrace {
                                _expected: '{',
                                _span: (token.span().offset() - 2, 1).into(),
                            });
                        }
                        if matches!(operator_orientation, Orientation::Vertical) {
                            diagnostics.push(ParsingError::MismatchingBrace {
                                expected: ']',
                                _span: token.span(),
                            });
                        }
                        inside_operator = false;
                        AsciiCell::OperatorEnd(Orientation::Horizontal)
                    }
                    ']' => {
                        if !inside_operator {
                            // diagnostics.push(ParsingError::MissingOpeningBrace {
                            //     _expected: '[',
                            //     _span: (token.span().offset() - 2, 1).into(),
                            // });
                        }
                        if matches!(operator_orientation, Orientation::Horizontal) {
                            // diagnostics.push(ParsingError::MismatchingBrace {
                            //     expected: '}',
                            //     _span: token.span(),
                            // });
                        }
                        inside_operator = false;
                        AsciiCell::OperatorEnd(Orientation::Vertical)
                    }
                    c if inside_operator => {
                        op_length += 1;
                        if op_length > 2 {
                            // diagnostics.push(ParsingError::MissingOpeningBrace {
                            //     _expected: brace,
                            //     _span: token.span(),
                            // })
                        }
                        AsciiCell::Operator {
                            orientation: operator_orientation,
                            repr: c,
                        }
                    }
                    ' ' => AsciiCell::Empty,
                    '.' => AsciiCell::Dot,
                    '-' => AsciiCell::Pipe(Orientation::Horizontal),
                    '|' => AsciiCell::Pipe(Orientation::Vertical),
                    '+' => AsciiCell::CrossPipe,
                    '/' => AsciiCell::Mirror(MirrorKind::Forward),
                    '\\' => AsciiCell::Mirror(MirrorKind::Backward),
                    '(' => AsciiCell::Reflector(MirrorKind::Forward),
                    ')' => AsciiCell::Reflector(MirrorKind::Backward),
                    '^' => AsciiCell::ForceDirection(Direction::Up),
                    '>' => AsciiCell::ForceDirection(Direction::Right),
                    'v' => AsciiCell::ForceDirection(Direction::Down),
                    '<' => AsciiCell::ForceDirection(Direction::Left),
                    '*' => AsciiCell::Duplicate,
                    '~' => AsciiCell::Conditional(matches!(get_char(x, y + 1), Some('!'))),
                    '!' => AsciiCell::Negate,
                    '&' => AsciiCell::StopProgram,
                    '#' => AsciiCell::ValueMarker,
                    '=' => AsciiCell::Assertion,
                    '@' => {
                        let name = get_name(x, y);
                        if let Some(name) = name {
                            functiondefs.push(AsciiFunc::new(name.clone(), (x, y)));
                            AsciiCell::DefRet(name)
                        } else {
                            AsciiCell::AddrMarker
                        }
                    }
                    '%' => match get_name(x, y) {
                        Some(name) if name == "display" => {
                            let id = display_counter;
                            display_counter += 1;
                            AsciiCell::Display(id)
                        }
                        Some(name) => {
                            if let Some(original) = aliases.get(&name) {
                                AsciiCell::Call(Arc::from(original.clone()))
                            } else {
                                AsciiCell::Call(Arc::from(name.clone()))
                            }
                        }
                        None => {
                            diagnostics.push(ParsingError::MissingNameForCallOperator {
                                _span: token.span(),
                            });
                            AsciiCell::Empty
                        }
                    },
                    '$' => AsciiCell::Write,
                    '?' => AsciiCell::Read,
                    '"' => AsciiCell::Quote,
                    '\'' => AsciiCell::QuoteStream,
                    ':' => AsciiCell::Filter(false),
                    ';' => AsciiCell::Filter(true),
                    c if c.is_dec_digit() => AsciiCell::Digit(c as u8 - b'0'),
                    c if c.is_alphabetic() && warps.contains(&c) => AsciiCell::Warp {
                        repr: c,
                        other: (0, 0).into(),
                    },
                    c if c.is_alphabetic() => AsciiCell::Char(c),
                    c if c.is_whitespace() => {
                        diagnostics.push(ParsingError::InvalidWhitespace {
                            _c: c,
                            _span: token.span(),
                        });
                        AsciiCell::Empty
                    }
                    c => {
                        diagnostics.push(ParsingError::UnknownCharacter {
                            ch: c,
                            _span: token.span(),
                        });
                        AsciiCell::Empty
                    }
                };
                let spanned = SpanToken::new(cell, token.repr(), token.span());
                field.set_at(spanned, &(x, y).into());
            }
        }
        // Link Warps
        let mut pairs = Vec::with_capacity(warps.len());
        let mut seen_warp: FxHashSet<char> = FxHashSet::default();
        'outer: for (x, y, spanned) in field.into_iter() {
            if let AsciiCell::Warp { repr, .. } = spanned.inner() {
                if seen_warp.contains(repr) {
                    continue;
                }
                // Foward Search other Warp
                for dy in y..height {
                    for dx in 0..width {
                        if x == dx && y == dy {
                            continue;
                        }
                        let spanned = field.at_spanned(&(dx, dy).into());
                        if let AsciiCell::Warp { repr: o_repr, .. } = spanned.inner()
                            && repr == o_repr
                        {
                            seen_warp.insert(*repr);
                            pairs.push(((x, y), (dx, dy)));
                            continue 'outer;
                        }
                    }
                }
                diagnostics.push(ParsingError::MissingDestinationForWarp {
                    _c: spanned.repr(),
                    _span: spanned.span(),
                })
            }
        }
        for ((x, y), (dx, dy)) in pairs {
            let pos1: Position = (x, y).into();
            let pos2: Position = (dx, dy).into();
            if let Some(spanned) = field.at_spanned_mut(&pos1)
                && let AsciiCell::Warp { other, .. } = spanned.inner_mut()
            {
                *other = pos2;
            }
            if let Some(spanned) = field.at_spanned_mut(&pos2)
                && let AsciiCell::Warp { other, .. } = spanned.inner_mut()
            {
                *other = pos1;
            }
        }
        // dbg!(self.module.preamble());
        // dbg!(&field);
        if diagnostics.is_empty() {
            Ok(DotsModule {
                field,
                functions: functiondefs,
                native_names: Vec::new(),
            })
        } else {
            Err(ParsingErrorCollection::new(diagnostics).into())
        }
    }
}
