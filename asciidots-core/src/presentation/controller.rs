use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{
    interpreter::{DotsInterpreter, InterpreterCommand, InterpreterResult},
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
    source_file: Option<PathBuf>,
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
    const MAX_STEPS_PER_FRAME: usize = 100_000_000;

    pub fn new(content: &str) -> miette::Result<Self> {
        let interpreter = DotsInterpreter::from_string(content)?;
        Ok(Self {
            interpreter,
            source: content.to_string(),
            source_file: None,
            tick_speed: Duration::from_millis(100),
            steps_per_frame: 1,
            tick_count: 0,
            last_step_duration: Duration::ZERO,
            running: false,
            clock: Instant::now(),
        })
    }

    pub fn new_from_file(path: impl AsRef<Path>) -> miette::Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let interpreter = DotsInterpreter::from_file(&path_buf)?;
        Ok(Self {
            interpreter,
            source: String::new(),
            source_file: Some(path_buf),
            tick_speed: Duration::from_millis(100),
            steps_per_frame: 1,
            tick_count: 0,
            last_step_duration: Duration::ZERO,
            running: false,
            clock: Instant::now(),
        })
    }

    pub fn reset(&mut self) {
        let rebuilt = if let Some(path) = &self.source_file {
            DotsInterpreter::from_file(path)
        } else {
            DotsInterpreter::from_string(&self.source)
        };

        if let Ok(interpreter) = rebuilt {
            self.interpreter = interpreter;
            self.tick_count = 0;
            self.last_step_duration = Duration::ZERO;
            self.running = false;
            self.clock = Instant::now();
        }
    }

    fn run_steps(&mut self, count: usize) -> Vec<InterpreterResult> {
        let mut all_results = Vec::new();
        for _ in 0..count {
            if self.interpreter.is_finished() {
                break;
            }
            let start = Instant::now();
            let results = self.interpreter.step().unwrap().to_vec();
            self.last_step_duration = start.elapsed();
            self.tick_count += 1;
            let needs_input = results.iter().any(|r| {
                matches!(r, InterpreterResult::InputRequest(_) | InterpreterResult::EndProgram)
            });
            all_results.extend(results);
            if needs_input {
                break;
            }
        }
        all_results
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

    /// Like [`tick`] but releases early if `budget` elapses, so callers that
    /// hold a lock can give other threads a chance to run.  `steps_per_frame`
    /// still acts as an upper bound on the number of steps per call.
    pub fn tick_budgeted(&mut self, budget: Duration) -> Vec<InterpreterResult> {
        if !self.running || self.clock.elapsed() < self.tick_speed {
            return Vec::new();
        }
        self.clock = Instant::now();
        let deadline = Instant::now() + budget;
        let mut all_results = Vec::new();
        let batch_start = Instant::now();
        let mut steps_run: usize = 0;
        for i in 0..self.steps_per_frame {
            if self.interpreter.is_finished() {
                break;
            }
            // Check the deadline every 256 steps to avoid calling Instant::now()
            // on every iteration — the syscall overhead adds up at high step counts.
            if i & 255 == 255 && Instant::now() >= deadline {
                break;
            }
            let results = self.interpreter.step().unwrap().to_vec();
            self.tick_count += 1;
            steps_run += 1;
            let needs_input = results.iter().any(|r| {
                matches!(r, InterpreterResult::InputRequest(_) | InterpreterResult::EndProgram)
            });
            all_results.extend(results);
            if needs_input {
                break;
            }
        }
        if steps_run > 0 {
            self.last_step_duration = batch_start.elapsed() / steps_run as u32;
        }
        all_results
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
        let operator_with_dots = self
            .interpreter
            .queues()
            .iter()
            .filter(|(_, q)| q.waiting_len() > 0)
            .map(|((x, y), _)| (*x as i32, *y as i32))
            .collect();
        AsciiMapModel::new(
            self.interpreter.get_field(),
            self.interpreter.get_dots(),
            operator_with_dots,
        )
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

    pub fn dot_debug_table_model(&self) -> DotDebugTableModel<'_> {
        DotDebugTableModel::from_dots(
            self.interpreter.get_dots(),
            self.interpreter.queues().values().flat_map(|queue| queue.dots()),
        )
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
