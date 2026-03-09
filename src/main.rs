use std::fs::File;
use std::io::{prelude::*, stdout};
use std::time::Instant;

use AsciidotsCompiler::{AsciiField, ProgramState};
use crossterm::cursor::MoveTo;
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::style::{Print, Stylize};
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{event, execute, terminal};

fn main() {
    let mut program = ProgramState::from_file("samples/exponential.dots");
    let mut stdout = stdout();

    terminal::enable_raw_mode().unwrap();
    execute!(stdout, EnterAlternateScreen).unwrap();
    execute!(stdout, Clear(ClearType::All)).unwrap();

    let mut iteration = 0;
    let mut iterations_per_step = 1;
    // Print loop
    while program.num_dots() > 0 {
        if event::poll(std::time::Duration::from_millis(16)).unwrap() {
            match event::read().unwrap() {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('w') => iterations_per_step += 10,
                    KeyCode::Char('s') => iterations_per_step -= 10,
                    _ => {}
                },
                _ => {}
            }
        }
        if iterations_per_step < 1 {
            iterations_per_step = 1;
        }

        execute!(stdout, Clear(ClearType::All)).unwrap();
        execute!(stdout, MoveTo(0, 0)).unwrap();
        print!(
            "Iteration: {:>6} StepSize: {:>6}\r\n",
            iteration, iterations_per_step
        );
        print!("{}\r\n", program);
        let start = Instant::now();
        for _ in 0..iterations_per_step {
            program.step();
        }
        print!("Elapsed: {:>10?}\r\n", start.elapsed());

        std::thread::sleep(std::time::Duration::from_millis(10));
        iteration += iterations_per_step;
    }

    execute!(stdout, LeaveAlternateScreen).unwrap();
    terminal::disable_raw_mode().unwrap();
}
