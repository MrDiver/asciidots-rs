mod common;

use std::path::PathBuf;

use asciidots_core::{
    interpreter::{DisplayCommand, DotsInterpreter, DotsInterpreterBuilder, InterpreterResult},
    native_fun::NativeFunction,
    state_machine::DotMachine,
};
use common::trace_harness::{assert_interpreter_failure_with_ticks, assert_values_trace};
use log::trace;

use crate::common::trace_harness::assert_interpreter_success_with_ticks;

#[derive(Default)]
struct SinNative {
    queued: Vec<DotMachine>,
}

impl NativeFunction<DotMachine> for SinNative {
    fn name(&self) -> &str {
        "sin"
    }

    fn dot_enter(&mut self, dot: DotMachine) {
        self.queued.push(dot);
    }

    fn tick_into(&mut self, output: &mut Vec<DotMachine>) {
        output.extend(self.queued.drain(..).map(|mut dot| {
            let selected = dot.selected();
            dot.dot_mut().set_selected(selected.sin());
            dot
        }));
    }

    fn reset(&mut self) {
        self.queued.clear();
    }
}

fn build_native_sin_interpreter() -> DotsInterpreter {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native_sin.dots");

    let mut builder = DotsInterpreterBuilder::new();
    builder.register_native_function(Box::new(SinNative::default()));
    builder
        .build_from_file(&fixture_path)
        .expect("fixture should link and initialize with native function")
}

#[test]
fn fixed_assertion_failure_is_detected() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assertion_fail_fixed.dots");
    let program = std::fs::read_to_string(&fixture_path).expect("fixture should be readable");
    let mut interpreter =
        DotsInterpreter::from_string(&program).expect("fixture should lex and initialize");

    assert_interpreter_failure_with_ticks(&mut interpreter, 10, 64);
}

#[test]
fn native_sin_assertion_succeeds() {
    let mut interpreter = build_native_sin_interpreter();

    assert_eq!(
        interpreter.num_dots(),
        2,
        "fixture should initialize two independent dots"
    );

    assert_interpreter_success_with_ticks(&mut interpreter, 22, 128);
}

#[test]
fn native_sin_values_trace_current_behavior() {
    let mut interpreter = build_native_sin_interpreter();

    assert_values_trace(
        &mut interpreter,
        &[
            &[(0, 1, 0.0), (0, 2, 0.0)],
            &[(1, 1, 0.0), (1, 2, 0.0)],
            &[(2, 1, 0.0), (2, 2, 0.0)],
            &[(3, 1, 0.0), (3, 2, 0.0)],
            &[(4, 1, 0.0), (4, 2, 4.0)],
            &[(5, 1, 0.0), (5, 2, 4.0)],
            &[],
            &[(7, 1, 0.0), (7, 2, -0.7568)],
            &[(8, 1, 0.0), (8, 2, -0.7568)],
            &[(9, 1, 0.0), (9, 2, -0.7568)],
            &[(10, 1, 0.0), (10, 2, -0.7568)],
            &[(11, 1, 0.0), (11, 2, -0.7568)],
            &[(12, 1, 0.0), (12, 2, -0.7568)],
            &[(13, 1, 0.0), (13, 2, -0.7568)],
            &[(14, 1, 0.0), (14, 2, -0.7568)],
            &[(15, 1, 0.0), (15, 2, -0.7568)],
            &[(16, 1, 0.0), (16, 2, -0.7568)],
            &[(17, 1, 0.0), (17, 2, -0.7568)],
            &[(18, 1, 0.0), (18, 2, -0.7568)],
            &[(19, 1, 0.0), (19, 2, -0.7568)],
            &[(20, 1, 0.0), (20, 2, -0.7568)],
            &[(21, 2, -0.7568)],
            &[],
        ],
    );
}

#[test]
fn scope_fixture_runs_to_completion() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scope_test.dots");
    let mut interpreter = DotsInterpreterBuilder::new()
        .build_from_file(&fixture_path)
        .expect("scope fixture should link and initialize");

    let max_ticks = 10_000usize;
    let mut ticks = 0usize;

    while !interpreter.is_finished() {
        assert!(
            ticks < max_ticks,
            "scope fixture did not halt within {max_ticks} ticks"
        );
        let step_result = interpreter.step();
        assert!(
            step_result.is_ok(),
            "scope fixture failed with diagnostics: {}",
            step_result
                .err()
                .map(|diagnostics| diagnostics
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" | "))
                .unwrap_or_default()
        );
        ticks += 1;
    }
}

#[test]
fn display_fixture_emits_correct_events() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/display.dots");
    let mut interpreter = DotsInterpreterBuilder::new()
        .build_from_file(&fixture_path)
        .expect("display fixture should link and initialize");

    let max_ticks = 10_000usize;
    let mut ticks = 0usize;
    let mut set_width: Option<usize> = None;
    let mut color_mode_set = false;
    let mut pixel_values: Vec<f64> = Vec::new();

    while !interpreter.is_finished() {
        assert!(
            ticks < max_ticks,
            "display fixture did not halt within {max_ticks} ticks"
        );
        let results = interpreter
            .step()
            .expect("display fixture should not error");
        for result in results {
            if let InterpreterResult::Display { command, .. } = result {
                match command {
                    DisplayCommand::SetWidth(w) => set_width = Some(*w),
                    DisplayCommand::SetColorMode(_) => color_mode_set = true,
                    DisplayCommand::SetPixel { value, .. } => pixel_values.push(*value),
                    _ => {}
                }
            }
        }
        ticks += 1;
    }

    assert_eq!(set_width, Some(10), "expected SetWidth(10)");
    assert!(color_mode_set, "expected at least one SetColorMode command");
    assert!(
        !pixel_values.is_empty(),
        "expected at least one SetPixel command"
    );
    trace!(
        "display fixture: width={:?}, {} pixels, values={:?}",
        set_width,
        pixel_values.len(),
        pixel_values
    );
}

#[test]
fn juliaset_small_emits_display_events() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/juliaset/juliaset_small.dots");
    let mut interpreter = DotsInterpreterBuilder::new()
        .build_from_file(&fixture_path)
        .expect("juliaset_small fixture should link and initialize");

    let max_ticks = 500_000usize;
    let mut ticks = 0usize;
    let mut set_pixel_count = 0usize;
    let mut set_width: Option<usize> = None;
    let mut set_height: Option<usize> = None;

    while !interpreter.is_finished() {
        assert!(
            ticks < max_ticks,
            "juliaset_small did not halt within {max_ticks} ticks"
        );
        let results = interpreter.step().expect("juliaset_small should not error");
        for result in results {
            if let InterpreterResult::Display {
                display_id,
                command,
            } = result
            {
                trace!("[tick={ticks}] id={display_id} {command:?}");
                match command {
                    DisplayCommand::SetWidth(w) => set_width = Some(*w),
                    DisplayCommand::SetHeight(h) => set_height = Some(*h),
                    DisplayCommand::SetPixel { .. } => set_pixel_count += 1,
                    _ => {}
                }
            }
        }
        ticks += 1;
    }

    assert!(
        set_width.is_some(),
        "expected at least one SetWidth display command"
    );
    assert!(
        set_height.is_some(),
        "expected at least one SetHeight display command"
    );
    assert!(
        set_pixel_count > 0,
        "expected at least one SetPixel display command, got 0"
    );
    trace!(
        "Display: {}x{}, {} pixels drawn",
        set_width.unwrap(),
        set_height.unwrap(),
        set_pixel_count
    );
}
