use std::io::stdout;
use std::time::Instant;

use AsciidotsCompiler::ProgramState;
use crossterm::cursor::MoveTo;
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{event, execute, terminal};

fn main() {
    let mut program = ProgramState::from_file("samples/basic.dots");
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

        let start = Instant::now();
        for _ in 0..iterations_per_step {
            program.step();
        }
        let elapsed = start.elapsed();
        let new_state_string = format!("{}", program);
        execute!(stdout, Clear(ClearType::All)).unwrap();
        execute!(stdout, MoveTo(0, 0)).unwrap();
        print!(
            "Iteration: {:>6} StepSize: {:>3} Num Dots: {}\r\n",
            iteration,
            iterations_per_step,
            program.num_dots()
        );
        print!("{}\r\n", new_state_string);
        print!("Elapsed: {:>10?}\r\n", elapsed);

        std::thread::sleep(std::time::Duration::from_millis(10).saturating_sub(elapsed));
        iteration += iterations_per_step;
    }

    execute!(stdout, LeaveAlternateScreen).unwrap();
    terminal::disable_raw_mode().unwrap();
}
