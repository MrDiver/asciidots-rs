// use std::collections::HashSet;
// use std::error::Error;
// use std::io::{BufWriter, Write, stdout};
// use std::path::PathBuf;
// use std::time::Instant;
// use std::{fs, iter};
//
// use AsciidotsCompiler::dot::Dot;
// use AsciidotsCompiler::interpreter::DotMachine;
// use AsciidotsCompiler::{AsciiField, ProgramState, parser};
// use clap::Parser;
// use crossterm::cursor::MoveTo;
// use crossterm::event::{Event, KeyCode, KeyEvent};
// use crossterm::style::Print;
// use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
// use crossterm::{event, execute, queue, terminal};
// use miette::IntoDiagnostic;
//
// #[derive(Parser, Debug)]
// #[command(name = "asciidots", about = "Run an AsciiDots program")]
// struct Args {
//     /// Path to the .dots file to execute
//     file: PathBuf,
//
//     /// Delay between rendered frames in milliseconds (default: 100)
//     #[arg(long, default_value_t = 100)]
//     ms: u64,
//
//     #[arg(long, default_value_t = false)]
//     paused: bool,
// }
//
// #[derive(Debug, miette::Diagnostic)]
// pub struct ProgramDiagnostics {
//     diagnostics: Vec<parser::Diagnostic>,
// }
//
// impl Error for ProgramDiagnostics {}
//
// impl std::fmt::Display for ProgramDiagnostics {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "The Program has {} diagnostics:", self.diagnostics.len())
//     }
// }
//
// struct Renderer {
//     grid: Vec<Vec<char>>,
//     out: BufWriter<std::io::Stdout>,
// }
//
// impl Renderer {
//     fn new(field: &AsciiField) -> Self {
//         let max_col = field
//             .get_field()
//             .iter()
//             .map(|row| row.len())
//             .max()
//             .unwrap_or(0);
//         let max_row = field.get_field().len();
//         let size = (max_col, max_row);
//         let mut grid = vec![vec![' '; size.0]; size.1];
//         for row in field.get_field() {
//             for token in row {
//                 grid[token.line()][token.col()] = token.repr();
//             }
//         }
//         Self {
//             grid,
//             out: BufWriter::new(stdout()),
//         }
//     }
//
//     fn render(&mut self, dots: &[DotMachine]) {
//         let dot_positions: HashSet<(usize, usize)> = dots
//             .iter()
//             .map(|d| (d.position().x as usize, d.position().y as usize))
//             .collect();
//
//         queue!(self.out, MoveTo(0, 0)).unwrap();
//
//         for (y, row) in self.grid.iter().enumerate() {
//             for (x, ch) in row.iter().enumerate() {
//                 if dot_positions.contains(&(x, y)) {
//                     queue!(self.out, Print(format!("\x1b[31m{}\x1b[0m", ch))).unwrap();
//                 } else {
//                     queue!(self.out, Print(ch)).unwrap();
//                 }
//             }
//             queue!(self.out, Print("\r\n")).unwrap();
//         }
//
//         self.out.flush().unwrap();
//     }
// }
//
// fn main() -> miette::Result<()> {
//     let args = Args::parse();
//     println!("Loading Map");
//     let content = fs::read_to_string(&args.file).into_diagnostic()?;
//     println!("Parsing content");
//     let mut program = match ProgramState::from_string(&content) {
//         Ok(p) => p,
//         Err(diagnostics) => return Err(miette::Report::new(ProgramDiagnostics { diagnostics })),
//     };
//     let mut renderer = Renderer::new(program.get_field());
//     println!("Loaded field with {} dots.", program.num_dots());
//     let mut stdout = stdout();
//
//     terminal::enable_raw_mode().unwrap();
//     execute!(stdout, EnterAlternateScreen).unwrap();
//     execute!(stdout, Clear(ClearType::All)).unwrap();
//
//     let mut iteration = 0;
//     let mut iterations_per_step = 1;
//     let mut step_ms = args.ms as i64;
//     let mut running = !args.paused;
//     // Print loop
//     while program.num_dots() > 0 {
//         if event::poll(std::time::Duration::from_millis(16)).unwrap()
//             && let Event::Key(KeyEvent { code, .. }) = event::read().unwrap()
//         {
//             match code {
//                 KeyCode::Char('q') => break,
//                 KeyCode::Char('w') => iterations_per_step += 1000,
//                 KeyCode::Char('s') => iterations_per_step -= 1000,
//                 KeyCode::Char('a') => step_ms -= 10,
//                 KeyCode::Char('d') => step_ms += 10,
//                 KeyCode::Char(' ') => running = !running,
//                 KeyCode::Char('r') => {
//                     for _ in 0..iterations_per_step {
//                         if let Err(d) = program.step() {
//                             return Err(miette::Report::new(ProgramDiagnostics {
//                                 diagnostics: Vec::from(d),
//                             }));
//                         }
//                     }
//                     iteration += iterations_per_step;
//                 }
//                 _ => {}
//             }
//         }
//         if iterations_per_step < 1 {
//             iterations_per_step = 1;
//         }
//         step_ms = step_ms.clamp(0, 1000);
//
//         let start = Instant::now();
//         if running {
//             for _ in 0..iterations_per_step {
//                 if let Err(d) = program.step() {
//                     return Err(miette::Report::new(ProgramDiagnostics {
//                         diagnostics: Vec::from(d),
//                     }));
//                 }
//             }
//         }
//         let elapsed = start.elapsed();
//         // let new_state_string = format!("{}", program);
//         queue!(stdout, Clear(ClearType::All)).unwrap();
//         renderer.render(program.get_dots());
//         queue!(
//             stdout,
//             Print(format!(
//                 "Iteration: {:>6} StepSize: {:>3} Num Dots: {}\r\n",
//                 iteration,
//                 iterations_per_step,
//                 program.num_dots()
//             ))
//         )
//         .unwrap();
//         queue!(stdout, Print(format!("Elapsed: {:>10?}\r\n", elapsed))).unwrap();
//         for dot in program.get_dots() {
//             queue!(stdout, Print(format!("{:?}\r\n", dot))).unwrap();
//         }
//         // for queue in program.queues() {
//         //     queue!(stdout, Print(format!("{:?}\r\n", queue.1))).unwrap();
//         // }
//         let skip_elements = program.output().lines().count().saturating_sub(10);
//         queue!(
//             stdout,
//             Print(
//                 program
//                     .output()
//                     .lines()
//                     .into_iter()
//                     .skip(skip_elements)
//                     .flat_map(|s| [s, "\r\n"])
//                     .collect::<String>()
//             )
//         )
//         .unwrap();
//
//         if step_ms > 0 {
//             std::thread::sleep(
//                 std::time::Duration::from_millis(step_ms as u64).saturating_sub(elapsed),
//             );
//         }
//         if running {
//             iteration += iterations_per_step;
//         }
//     }
//
//     execute!(stdout, LeaveAlternateScreen).unwrap();
//     terminal::disable_raw_mode().unwrap();
//     Ok(())
// }

use asciidots_tui::run;

pub fn main() {
    let program = ".--#23-$#-&";
    run(program);
}
