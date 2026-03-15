use std::time::{Duration, Instant};

use crate::{
    interpreter::{DotsInterpreter, InterpreterResult},
    parser::LexError,
};

use super::models::AsciiMapModel;

pub enum AppCommand {
    Step,
    Reverse,
    Pause,
    Start,
    Reset,
    SetSpeed(Duration),
    StopProgram,
}

pub struct RuntimeController {
    interpreter: DotsInterpreter,
    source: String,
    tick_speed: Duration,
    steps_per_frame: usize,
    running: bool,
    clock: Instant,
}

impl RuntimeController {
    pub fn new(content: &str) -> Result<Self, Vec<LexError>> {
        let interpreter = DotsInterpreter::from_string(content)?;
        Ok(Self {
            interpreter,
            source: content.to_string(),
            tick_speed: Duration::from_millis(100),
            steps_per_frame: 1,
            running: false,
            clock: Instant::now(),
        })
    }

    pub fn reset(&mut self) {
        if let Ok(interpreter) = DotsInterpreter::from_string(&self.source) {
            self.interpreter = interpreter;
            self.running = false;
            self.clock = Instant::now();
        }
    }

    pub fn tick(&mut self) -> Vec<InterpreterResult> {
        if self.running && self.clock.elapsed() >= self.tick_speed {
            for _ in 0..self.steps_per_frame {
                let results = self.interpreter.step().unwrap().to_vec();
                if !results.is_empty() {
                    return results;
                }
            }
            self.clock = Instant::now();
        }
        Vec::new()
    }

    pub fn handle_command(&mut self, command: AppCommand) {
        match command {
            AppCommand::Step => {
                self.interpreter.step().unwrap();
            }
            AppCommand::Reverse => {
                todo!()
            }
            AppCommand::Pause => {
                self.running = false;
            }
            AppCommand::Start => {
                self.running = true;
            }
            AppCommand::Reset => {
                self.reset();
            }
            AppCommand::SetSpeed(speed) => {
                self.tick_speed = speed;
            }
            AppCommand::StopProgram => {
                // Handled by caller
            }
        }
    }

    pub fn grid_model(&self) -> AsciiMapModel<'_> {
        AsciiMapModel::new(self.interpreter.get_field(), self.interpreter.get_dots())
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn tick_speed(&self) -> Duration {
        self.tick_speed
    }

    pub fn steps_per_frame(&self) -> usize {
        self.steps_per_frame
    }

    pub fn num_dots(&self) -> usize {
        self.interpreter.num_dots()
    }
}

pub trait DotsView {
    fn render(&mut self, controller: &RuntimeController);
    fn poll_command(&mut self) -> Option<AppCommand>;
}
