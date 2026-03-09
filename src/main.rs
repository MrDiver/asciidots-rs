use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::io::{BufWriter, Write, stdout};
use std::time::Instant;

use AsciidotsCompiler::dot::Dot;
use AsciidotsCompiler::{AsciiField, ProgramState, parser};
use crossterm::cursor::MoveTo;
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::style::{Color, Print, SetForegroundColor, Stylize};
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{event, execute, queue, terminal};
use miette::IntoDiagnostic;

#[derive(Debug, miette::Diagnostic)]
pub struct ProgramDiagnostics {
    diagnostics: Vec<parser::Diagnostic>,
}

impl Error for ProgramDiagnostics {}

impl std::fmt::Display for ProgramDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The Program has {} diagnostics:", self.diagnostics.len())
    }
}

struct Renderer {
    size: (usize, usize),
    grid: Vec<Vec<char>>,
    out: BufWriter<std::io::Stdout>,
}

impl Renderer {
    fn new(field: &AsciiField) -> Self {
        let max_col = field.get_field().keys().map(|(x, _)| *x).max().unwrap_or(0);
        let max_row = field.get_field().keys().map(|(_, y)| *y).max().unwrap_or(0);
        let size = (max_col + 1, max_row + 1);
        let mut grid = vec![vec![' '; size.0]; size.1];
        for ((x, y), token) in field.get_field().iter() {
            grid[*y][*x] = token.repr();
        }
        Self { size, grid, out: BufWriter::new(stdout()) }
    }

    fn render(&mut self, dots: &[Dot]) {
  let dot_positions: HashSet<(usize, usize)> = dots
        .iter()
        .map(|d| (d.position.x as usize, d.position.y as usize))
        .collect();

    queue!(self.out, MoveTo(0, 0)).unwrap();

    for (y, row) in self.grid.iter().enumerate() {
        for (x, ch) in row.iter().enumerate() {
            if dot_positions.contains(&(x, y)) {
                queue!(self.out, Print(format!("\x1b[31m{}\x1b[0m", ch))).unwrap();
            } else {
                queue!(self.out, Print(ch)).unwrap();
            }
        }
        queue!(self.out, Print("\r\n")).unwrap();
    }

    self.out.flush().unwrap();
    }
}

fn main() -> miette::Result<()> {
    println!("Loading Map");
    let content = fs::read_to_string("samples/basic.dots").into_diagnostic()?;
    println!("Parsing content");
    let mut program = match ProgramState::from_string(&content) {
        Ok(p) => p,
        Err(diagnostics) => return Err(miette::Report::new(ProgramDiagnostics { diagnostics })),
    };
    program.initalize_dots();
    let mut renderer = Renderer::new(program.get_field());
    println!("{:#?}", program);
    println!("Loaded field with {} dots.", program.num_dots());
    let mut stdout = stdout();

    terminal::enable_raw_mode().unwrap();
    execute!(stdout, EnterAlternateScreen).unwrap();
    execute!(stdout, Clear(ClearType::All)).unwrap();

    let mut iteration = 0;
    let mut iterations_per_step = 1;
    // Print loop
    while program.num_dots() > 0 {
        if event::poll(std::time::Duration::from_millis(16)).unwrap()
            && let Event::Key(KeyEvent { code, .. }) = event::read().unwrap()
        {
            match code {
                KeyCode::Char('q') => break,
                KeyCode::Char('w') => iterations_per_step += 10,
                KeyCode::Char('s') => iterations_per_step -= 10,
                _ => {}
            }
        }
        if iterations_per_step < 1 {
            iterations_per_step = 1;
        }

        let start = Instant::now();
        for _ in 0..iterations_per_step {
            if let Err(d) = program.step() {
                return Err(miette::Report::new(ProgramDiagnostics {
                    diagnostics: Vec::from(d),
                }));
            }
        }
        let elapsed = start.elapsed();
        // let new_state_string = format!("{}", program);
        queue!(stdout, Clear(ClearType::All)).unwrap();
        renderer.render(program.get_dots());
        queue!(
            stdout,
            Print(format!(
                "Iteration: {:>6} StepSize: {:>3} Num Dots: {}\r\n",
                iteration,
                iterations_per_step,
                program.num_dots()
            ))
        )
        .unwrap();
        queue!(stdout, Print(format!("Elapsed: {:>10?}\r\n", elapsed))).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10).saturating_sub(elapsed));
        iteration += iterations_per_step;
    }

    execute!(stdout, LeaveAlternateScreen).unwrap();
    terminal::disable_raw_mode().unwrap();
    Ok(())
}
