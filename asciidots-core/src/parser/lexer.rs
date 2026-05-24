use super::error::LexError;
use miette::SourceSpan;
use nom::{
    AsChar, IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_till, take_while1},
    character::{
        complete::{alpha1, anychar, line_ending, space0, space1},
        streaming::alphanumeric1,
    },
    combinator::opt,
    multi::{many0, many1, separated_list1},
    sequence::{preceded, terminated},
};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str>;

fn into_span(span: Span) -> SourceSpan {
    (span.location_offset(), span.fragment().len()).into()
}

#[derive(Clone, Debug)]
pub enum ImportSource {
    Path(String),
    Native,
}

#[derive(Clone, Debug)]
pub enum PreambleStatement {
    Import {
        source: ImportSource,
        name: String,
        alias: Option<String>,
        span: SourceSpan,
    },
    Warps {
        warps: Vec<char>,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone)]
pub struct AsciiToken {
    repr: char,
    span: SourceSpan,
}

impl AsciiToken {
    pub fn repr(&self) -> char {
        self.repr
    }
    pub fn span(&self) -> SourceSpan {
        self.span
    }
}

pub struct DotsSource {
    preamble: Vec<PreambleStatement>,
    grid: Vec<Vec<AsciiToken>>,
}

impl DotsSource {
    pub fn preamble(&self) -> &Vec<PreambleStatement> {
        &self.preamble
    }

    pub fn grid(&self) -> &Vec<Vec<AsciiToken>> {
        &self.grid
    }
}

pub struct Lexer<'a> {
    source: &'a str,
}

fn import_alias_parser(i: Span) -> IResult<Span, String> {
    let (i, _) = tag("as")(i)?;
    let (i, _) = space1(i)?;
    let (i, name) = alphanumeric1(i)?;
    Ok((i, name.to_string()))
}

fn import_source_parser(i: Span) -> IResult<Span, ImportSource> {
    let (i, source) = take_while1(|c: char| !c.is_whitespace())(i)?;
    let source = if source.fragment() == &"native" {
        ImportSource::Native
    } else {
        ImportSource::Path(source.to_string())
    };
    Ok((i, source))
}

fn import_name_parser(i: Span) -> IResult<Span, Span> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(i)
}

fn import_item_parser(i: Span) -> IResult<Span, (String, Option<String>, SourceSpan)> {
    let (i, name) = import_name_parser(i)?;
    let (i, alias) = opt(preceded(space1, import_alias_parser)).parse(i)?;
    Ok((i, (name.to_string(), alias, into_span(name))))
}

fn from_import_parser(i: Span) -> IResult<Span, Vec<PreambleStatement>> {
    let (i, _) = tag("from")(i)?;
    let (i, _) = space1(i)?;
    let (i, source) = import_source_parser(i)?;
    let (i, _) = space1(i)?;
    let (i, _) = tag("import")(i)?;
    let (i, _) = space1(i)?;
    let (i, imports) =
        separated_list1(terminated(tag(","), space0), import_item_parser).parse(i)?;

    let statements = imports
        .into_iter()
        .map(|(name, alias, span)| PreambleStatement::Import {
            source: source.clone(),
            name,
            alias,
            span,
        })
        .collect();
    Ok((i, statements))
}

fn warp_parser(i: Span) -> IResult<Span, Vec<PreambleStatement>> {
    let (i, _) = tag("warp")(i)?;
    let (i, _) = space1(i)?;
    let (i, chars) = alpha1(i)?;
    let (_, warps) = many1(anychar).parse(chars)?;
    let (i, _) = space0(i)?;
    Ok((
        i,
        vec![PreambleStatement::Warps {
            warps,
            span: into_span(chars),
        }],
    ))
}

fn ascii_token_parser(i: Span) -> IResult<Span, AsciiToken> {
    let span: SourceSpan = (i.location_offset(), 1).into();
    let (i, c) = anychar(i)?;
    Ok((i, AsciiToken { repr: c, span }))
}

fn grid_line_parser(i: Span) -> IResult<Span, Vec<AsciiToken>> {
    let (i, line) = take_till(AsChar::is_newline).parse(i)?;
    let (_, x) = many0(ascii_token_parser).parse(line)?;
    Ok((i, x))
}

fn grid_parser(i: Span) -> IResult<Span, Vec<Vec<AsciiToken>>> {
    let (i, lines) = separated_list1(line_ending, grid_line_parser).parse(i)?;
    // let (i, mut lines) = many0(terminated(grid_line_parser, line_ending)).parse(i)?;
    // let (i, last_line) = grid_line_parser(i)?;
    // if !last_line.is_empty() {
    //     lines.push(last_line);
    // }
    Ok((i, lines))
}

fn dot_module_parser(i: Span) -> IResult<Span, DotsSource> {
    let (i, preamble_parts) = many0(terminated(
        alt((from_import_parser, warp_parser)),
        line_ending,
    ))
    .parse(i)?;
    let preamble = preamble_parts.into_iter().flatten().collect();
    let (i, tokens) = grid_parser(i)?;
    Ok((
        i,
        DotsSource {
            preamble,
            grid: tokens,
        },
    ))
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    pub fn run(self) -> miette::Result<DotsSource> {
        match dot_module_parser(self.source.into()) {
            Ok((_, source)) => Ok(source),
            Err(e) => match e {
                nom::Err::Incomplete(_) => unreachable!("Should not happen"),
                nom::Err::Error(e) | nom::Err::Failure(e) => {
                    let span = into_span(e.input);
                    Err(LexError::LexingFailure {
                        span,
                        _help: format!(
                            "unexpected '{}' at line {}, column {}",
                            e.input.fragment().chars().next().unwrap_or('?'),
                            e.input.location_line(),
                            e.input.get_column(),
                        ),
                    }
                    .into())
                }
            },
        }
    }
}
