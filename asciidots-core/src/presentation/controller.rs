use std::time::{Duration, Instant};

use crate::{
    interpreter::{DotsInterpreter, InterpreterCommand, InterpreterResult},
    parser::LexError,
};

use super::models::{AsciiMapModel, DotDebugTableModel, SettingsTableModel};

pub enum AppCommand {
    Step,
    Reverse,
    Pause,
    Start,
    Reset,
    AdjustSpeed(i64),
    AdjustStepsPerFrame(i32),
    InputAnswer(u64, f64),
    StopProgram,
}

pub struct RuntimeController {
    interpreter: DotsInterpreter,
    source: String,
    tick_speed: Duration,
    steps_per_frame: usize,
    tick_count: usize,
    last_step_duration: Duration,
    running: bool,
    clock: Instant,
}

impl RuntimeController {
    const MIN_TICK_MS: u64 = 0;
    const MAX_TICK_MS: u64 = 5_000;
    const MIN_STEPS_PER_FRAME: usize = 1;
    const MAX_STEPS_PER_FRAME: usize = 10_000;

    pub fn new(content: &str) -> Result<Self, Vec<LexError>> {
        let interpreter = DotsInterpreter::from_string(content)?;
        Ok(Self {
            interpreter,
            source: content.to_string(),
            tick_speed: Duration::from_millis(100),
            steps_per_frame: 1,
            tick_count: 0,
            last_step_duration: Duration::ZERO,
            running: false,
            clock: Instant::now(),
        })
    }

    pub fn reset(&mut self) {
        if let Ok(interpreter) = DotsInterpreter::from_string(&self.source) {
            self.interpreter = interpreter;
            self.tick_count = 0;
            self.last_step_duration = Duration::ZERO;
            self.running = false;
            self.clock = Instant::now();
        }
    }

    fn run_steps(&mut self, count: usize) -> Vec<InterpreterResult> {
        for _ in 0..count {
            if self.interpreter.num_dots() == 0 {
                break;
            }
            let start = Instant::now();
            let results = self.interpreter.step().unwrap().to_vec();
            self.last_step_duration = start.elapsed();
            self.tick_count += 1;
            if !results.is_empty() {
                return results;
            }
        }
        Vec::new()
    }

    fn set_tick_speed_ms(&mut self, ms: u64) {
        let clamped = ms.clamp(Self::MIN_TICK_MS, Self::MAX_TICK_MS);
        self.tick_speed = Duration::from_millis(clamped);
        self.clock = Instant::now();
    }

    fn adjust_tick_speed_ms(&mut self, delta_ms: i64) {
        let current = self.tick_speed.as_millis() as i64;
        let next = (current + delta_ms).clamp(Self::MIN_TICK_MS as i64, Self::MAX_TICK_MS as i64);
        self.set_tick_speed_ms(next as u64);
    }

    fn adjust_steps_per_frame(&mut self, delta: i32) {
        let current = self.steps_per_frame as i32;
        let next = (current + delta).clamp(
            Self::MIN_STEPS_PER_FRAME as i32,
            Self::MAX_STEPS_PER_FRAME as i32,
        );
        self.steps_per_frame = next as usize;
    }

    pub fn tick(&mut self) -> Vec<InterpreterResult> {
        if self.running && self.clock.elapsed() >= self.tick_speed {
            self.clock = Instant::now();
            return self.run_steps(self.steps_per_frame);
        }
        Vec::new()
    }

    pub fn handle_command(&mut self, command: AppCommand) -> Vec<InterpreterResult> {
        match command {
            AppCommand::Step => return self.run_steps(1),
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
            AppCommand::AdjustSpeed(delta_ms) => {
                self.adjust_tick_speed_ms(delta_ms);
            }
            AppCommand::AdjustStepsPerFrame(delta) => {
                self.adjust_steps_per_frame(delta);
            }
            AppCommand::StopProgram => {}
            AppCommand::InputAnswer(uuid, value) => {
                self.interpreter
                    .handle_command(InterpreterCommand::InputAnswer(uuid, value));
            }
        }
        Vec::new()
    }

    pub fn grid_model(&self) -> AsciiMapModel<'_> {
        AsciiMapModel::new(self.interpreter.get_field(), self.interpreter.get_dots())
    }

    pub fn settings_table_model(&self) -> SettingsTableModel {
        SettingsTableModel::from_runtime_state(
            self.running,
            self.tick_speed,
            self.steps_per_frame,
            self.tick_count,
            self.last_step_duration,
            self.interpreter.num_dots(),
        )
    }

    pub fn dot_debug_table_model(&self) -> DotDebugTableModel {
        DotDebugTableModel::from_dots(self.interpreter.get_dots())
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

    pub fn tick_count(&self) -> usize {
        self.tick_count
    }

    pub fn last_step_duration(&self) -> Duration {
        self.last_step_duration
    }
}

pub trait DotsView {
    fn render(&mut self, controller: &RuntimeController);
    fn poll_command(&mut self) -> Option<AppCommand>;
}
