use asciidots_core::interpreter::DotsInterpreter;
use asciidots_core::state_machine::RuntimeDiagnostic;

#[allow(unused)]
pub type DotPos = (i32, i32);
pub type DotValueAtPos = (i32, i32, f64);

#[derive(Debug, Clone, Eq, PartialEq)]
enum Status {
    Success,
    Failure(Vec<String>),
}

impl Status {
    fn from_diagnostics(diagnostics: &[RuntimeDiagnostic]) -> Self {
        Status::Failure(diagnostics.iter().map(ToString::to_string).collect())
    }

    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Status::Success, Status::Success) => Status::Success,
            (Status::Success, Status::Failure(messages)) => Status::Failure(messages),
            (Status::Failure(messages), Status::Success) => Status::Failure(messages),
            (Status::Failure(mut left), Status::Failure(right)) => {
                left.extend(right);
                Status::Failure(left)
            }
        }
    }

    fn is_success(&self) -> bool {
        matches!(self, Status::Success)
    }

    fn messages(&self) -> &[String] {
        match self {
            Status::Success => &[],
            Status::Failure(messages) => messages,
        }
    }
}

#[allow(unused)]
fn sorted_positions(interpreter: &DotsInterpreter) -> Vec<DotPos> {
    let mut positions: Vec<DotPos> = interpreter
        .get_dots()
        .iter()
        .map(|dot| {
            let pos = dot.position();
            (pos.x, pos.y)
        })
        .collect();
    positions.sort_unstable();
    positions
}

fn sorted_position_values(interpreter: &DotsInterpreter) -> Vec<DotValueAtPos> {
    let mut values: Vec<DotValueAtPos> = interpreter
        .get_dots()
        .iter()
        .map(|dot| {
            let pos = dot.position();
            (pos.x, pos.y, dot.dot().value())
        })
        .collect();
    values.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    values
}

fn step_n(interpreter: &mut DotsInterpreter, n: usize) -> Status {
    let mut status = Status::Success;
    for _ in 0..n {
        let step_status = match interpreter.step() {
            Ok(_) => Status::Success,
            Err(diagnostics) => Status::from_diagnostics(diagnostics),
        };
        status = status.merge(step_status);
        if !status.is_success() {
            break;
        }
    }
    status
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct RunSummary {
    ticks: usize,
    status: Status,
    has_assertion_failure: bool,
}

fn has_assertion_failed_event(diagnostics: &[RuntimeDiagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic, RuntimeDiagnostic::AssertionFailed { .. }))
}

#[allow(unused)]
pub fn assert_positions_trace(interpreter: &mut DotsInterpreter, expected: &[&[DotPos]]) {
    for (tick, expected_positions) in expected.iter().enumerate() {
        let got = sorted_positions(interpreter);
        assert_eq!(
            got, *expected_positions,
            "position mismatch at tick {tick}: expected {:?}, got {:?}",
            expected_positions, got
        );
        if tick + 1 < expected.len() {
            let status = step_n(interpreter, 1);
            assert!(
                status.is_success(),
                "program failed while collecting position trace at tick {}: {}",
                tick + 1,
                status.messages().join(" | ")
            );
        }
    }
}

pub fn assert_values_trace(interpreter: &mut DotsInterpreter, expected: &[&[DotValueAtPos]]) {
    for (tick, expected_values) in expected.iter().enumerate() {
        let got = sorted_position_values(interpreter);
        assert_eq!(
            got.len(),
            expected_values.len(),
            "value trace length mismatch at tick {tick}: expected {}, got {}",
            expected_values.len(),
            got.len()
        );
        for (index, (got_pos_x, got_pos_y, got_value)) in got.iter().enumerate() {
            let (exp_pos_x, exp_pos_y, exp_value) = expected_values[index];
            assert_eq!(
                (*got_pos_x, *got_pos_y),
                (exp_pos_x, exp_pos_y),
                "position mismatch in value trace at tick {tick}, index {index}"
            );
            assert!(
                (*got_value - exp_value).abs() < 1e-3,
                "value mismatch at tick {tick}, index {index}: expected {}, got {}",
                exp_value,
                got_value
            );
        }
        if tick + 1 < expected.len() {
            let status = step_n(interpreter, 1);
            assert!(
                status.is_success(),
                "program failed while collecting value trace at tick {}: {}",
                tick + 1,
                status.messages().join(" | ")
            );
        }
    }
}

fn run_interpreter_until_halt(interpreter: &mut DotsInterpreter, max_ticks: usize) -> RunSummary {
    let mut ticks = 0usize;
    let mut status = Status::Success;
    let mut has_assertion_failure = false;

    while !interpreter.is_finished() {
        assert!(
            ticks < max_ticks,
            "program did not halt within {max_ticks} ticks"
        );
        match interpreter.step() {
            Ok(_) => status = status.merge(Status::Success),
            Err(diagnostics) => {
                status = status.merge(Status::from_diagnostics(diagnostics));
                has_assertion_failure |= has_assertion_failed_event(diagnostics);
            }
        }
        ticks += 1;
    }

    RunSummary {
        ticks,
        status,
        has_assertion_failure,
    }
}

#[allow(dead_code)]
pub fn assert_interpreter_ticks_to_halt(
    interpreter: &mut DotsInterpreter,
    expected_ticks: usize,
    max_ticks: usize,
) {
    let summary = run_interpreter_until_halt(interpreter, max_ticks);
    assert_eq!(
        summary.ticks, expected_ticks,
        "halt tick count mismatch: expected {expected_ticks}, got {}",
        summary.ticks
    );
}

#[allow(dead_code)]
pub fn assert_interpreter_success_with_ticks(
    interpreter: &mut DotsInterpreter,
    expected_ticks: usize,
    max_ticks: usize,
) {
    let summary = run_interpreter_until_halt(interpreter, max_ticks);
    assert!(
        summary.status.is_success(),
        "Expected program to succeed but it failed: {}",
        summary.status.messages().join(" | ")
    );
    assert_eq!(
        summary.ticks, expected_ticks,
        "halt tick count mismatch: expected {expected_ticks}, got {}",
        summary.ticks
    );
}

#[allow(dead_code)]
pub fn assert_interpreter_failure_with_ticks(
    interpreter: &mut DotsInterpreter,
    expected_ticks: usize,
    max_ticks: usize,
) {
    let summary = run_interpreter_until_halt(interpreter, max_ticks);
    assert!(
        !summary.status.is_success(),
        "Expected program to fail but it succeeded"
    );
    assert!(
        summary.has_assertion_failure,
        "Expected failure caused by assertion event, but no AssertionFailed diagnostic was reported"
    );
    assert_eq!(
        summary.ticks, expected_ticks,
        "halt tick count mismatch: expected {expected_ticks}, got {}",
        summary.ticks
    );
}
