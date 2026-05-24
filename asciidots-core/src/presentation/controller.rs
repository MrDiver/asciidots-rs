use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::interpreter::{DotsInterpreter, InterpreterCommand, InterpreterResult};

use super::models::{AsciiMapModel, DotDebugTableModel, SettingsTableModel};

pub enum AppCommand {
    Step,
    Reverse,
    Pause,
    Start,
    Reset,
    /// Adjust tick delay in milliseconds (positive = slower, negative = faster).
    AdjustSpeed(i64),
    AdjustStepsPerFrame(i32),
    InputAnswer(u64, f64),
    StopProgram,
}

pub struct RuntimeController {
    interpreter: DotsInterpreter,
    source: String,
    source_file: Option<PathBuf>,
    /// Target delay between ticks. The stepping thread owns the clock; this is
    /// just the config it reads.
    tick_speed: Duration,
    steps_per_frame: usize,
    tick_count: usize,
    running: bool,
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
            running: false,
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
            running: false,
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
            self.running = false;
        }
    }

    /// Run up to `count` steps. Stops early on input request or end.
    pub fn run_steps(&mut self, count: usize) -> Vec<InterpreterResult> {
        let mut all_results = Vec::new();
        for _ in 0..count {
            if self.interpreter.is_finished() {
                break;
            }
            let results = self.interpreter.step().unwrap().to_vec();
            self.tick_count += 1;
            let needs_input = results.iter().any(|r| {
                matches!(
                    r,
                    InterpreterResult::InputRequest(_) | InterpreterResult::EndProgram
                )
            });
            all_results.extend(results);
            if needs_input {
                break;
            }
        }
        all_results
    }

    /// Run up to `steps_per_frame` steps if running, releasing early when `budget`
    /// elapses so the caller can give other threads a chance. The clock that gates
    /// whether it is *time* to call this lives in the stepping thread, not here.
    pub fn run_steps_budgeted(&mut self, budget: Duration) -> Vec<InterpreterResult> {
        if !self.running {
            return Vec::new();
        }
        let deadline = Instant::now() + budget;
        let mut all_results = Vec::new();
        for i in 0..self.steps_per_frame {
            if self.interpreter.is_finished() {
                break;
            }
            // Check deadline every 256 steps to avoid per-iteration syscall overhead.
            if i & 255 == 255 && Instant::now() >= deadline {
                break;
            }
            let results = self.interpreter.step().unwrap().to_vec();
            self.tick_count += 1;
            let needs_input = results.iter().any(|r| {
                matches!(
                    r,
                    InterpreterResult::InputRequest(_) | InterpreterResult::EndProgram
                )
            });
            all_results.extend(results);
            if needs_input {
                break;
            }
        }
        all_results
    }

    fn set_tick_speed_ms(&mut self, ms: u64) {
        self.tick_speed = Duration::from_millis(ms.clamp(Self::MIN_TICK_MS, Self::MAX_TICK_MS));
    }

    fn adjust_tick_speed_ms(&mut self, delta_ms: i64) {
        let next = (self.tick_speed.as_millis() as i64 + delta_ms)
            .clamp(Self::MIN_TICK_MS as i64, Self::MAX_TICK_MS as i64) as u64;
        self.set_tick_speed_ms(next);
    }

    fn adjust_steps_per_frame(&mut self, delta: i32) {
        let next = (self.steps_per_frame as i32 + delta).clamp(
            Self::MIN_STEPS_PER_FRAME as i32,
            Self::MAX_STEPS_PER_FRAME as i32,
        );
        self.steps_per_frame = next as usize;
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
            self.interpreter.num_dots(),
        )
    }

    pub fn dot_debug_table_model(&self) -> DotDebugTableModel<'_> {
        DotDebugTableModel::from_dots(
            self.interpreter.get_dots(),
            self.interpreter
                .queues()
                .values()
                .flat_map(|queue| queue.dots()),
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
}

pub trait DotsView {
    fn render(&mut self, controller: &RuntimeController);
    fn poll_command(&mut self) -> Option<AppCommand>;
}
