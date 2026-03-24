mod common;

use asciidots_core::interpreter::DotsInterpreter;
use common::trace_harness::{
    assert_interpreter_ticks_to_halt, assert_positions_trace, assert_values_trace,
};

const STRAIGHT_RIGHT: &str = include_str!("fixtures/position_straight_right.dots");
const MIRROR_TURN: &str = include_str!("fixtures/position_mirror_turn.dots");
const DUPLICATE_SPLIT: &str = include_str!("fixtures/position_duplicate_split.dots");
const FORCE_DIRECTION_TURN: &str = include_str!("fixtures/position_force_direction_turn.dots");
const REFLECTOR_BOUNCE: &str = include_str!("fixtures/position_reflector_bounce.dots");

fn interpreter_from(program: &str) -> DotsInterpreter {
    DotsInterpreter::from_string(program).expect("program should lex and initialize")
}

#[test]
fn dot_moves_right_then_dies_at_boundary() {
    let mut interpreter = interpreter_from(STRAIGHT_RIGHT);
    assert_positions_trace(
        &mut interpreter,
        &[&[(0, 0)], &[(1, 0)], &[(2, 0)], &[(3, 0)], &[]],
    );
}

#[test]
fn mirror_turns_dot_downward() {
    let mut interpreter = interpreter_from(MIRROR_TURN);
    assert_positions_trace(&mut interpreter, &[&[(0, 1)], &[(1, 1)], &[(2, 1)], &[(2, 2)]]);
}

#[test]
fn duplicate_spawns_perpendicular_dots() {
    let mut interpreter = interpreter_from(DUPLICATE_SPLIT);
    assert_positions_trace(
        &mut interpreter,
        &[
            &[(0, 1)],
            &[(1, 1)],
            &[(2, 1)],
            &[(2, 0), (2, 2), (3, 1)],
            &[],
        ],
    );
}

#[test]
fn force_direction_turns_and_moves_down() {
    let mut interpreter = interpreter_from(FORCE_DIRECTION_TURN);
    assert_positions_trace(
        &mut interpreter,
        &[&[(0, 0)], &[(1, 0)], &[(2, 0)], &[(2, 1)], &[(2, 2)]],
    );
}

#[test]
fn reflector_reverses_horizontal_direction() {
    let mut interpreter = interpreter_from(REFLECTOR_BOUNCE);
    assert_positions_trace(
        &mut interpreter,
        &[&[(0, 0)], &[(1, 0)], &[(2, 0)], &[(1, 0)], &[(0, 0)], &[]],
    );
}

#[test]
fn value_trace_wrapper_validates_dot_values_per_tick() {
    let mut interpreter = interpreter_from(STRAIGHT_RIGHT);
    assert_values_trace(
        &mut interpreter,
        &[
            &[(0, 0, 0.0)],
            &[(1, 0, 0.0)],
            &[(2, 0, 0.0)],
            &[(3, 0, 0.0)],
            &[],
        ],
    );
}

#[test]
fn tick_wrapper_validates_total_execution_length() {
    let mut interpreter = interpreter_from(STRAIGHT_RIGHT);
    assert_interpreter_ticks_to_halt(&mut interpreter, 4, 32);
}
