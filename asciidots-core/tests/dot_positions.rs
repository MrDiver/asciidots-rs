mod common;

use common::trace_harness::{assert_positions_trace, assert_ticks_to_halt, assert_values_trace};

const STRAIGHT_RIGHT: &str = include_str!("fixtures/position_straight_right.dots");
const MIRROR_TURN: &str = include_str!("fixtures/position_mirror_turn.dots");
const DUPLICATE_SPLIT: &str = include_str!("fixtures/position_duplicate_split.dots");
const FORCE_DIRECTION_TURN: &str = include_str!("fixtures/position_force_direction_turn.dots");
const REFLECTOR_BOUNCE: &str = include_str!("fixtures/position_reflector_bounce.dots");

#[test]
fn dot_moves_right_then_dies_at_boundary() {
    assert_positions_trace(
        STRAIGHT_RIGHT,
        &[&[(0, 0)], &[(1, 0)], &[(2, 0)], &[(3, 0)], &[]],
    );
}

#[test]
fn mirror_turns_dot_downward() {
    assert_positions_trace(MIRROR_TURN, &[&[(0, 1)], &[(1, 1)], &[(2, 1)], &[(2, 2)]]);
}

#[test]
fn duplicate_spawns_perpendicular_dots() {
    assert_positions_trace(
        DUPLICATE_SPLIT,
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
    assert_positions_trace(
        FORCE_DIRECTION_TURN,
        &[&[(0, 0)], &[(1, 0)], &[(2, 0)], &[(2, 1)], &[(2, 2)]],
    );
}

#[test]
fn reflector_reverses_horizontal_direction() {
    assert_positions_trace(
        REFLECTOR_BOUNCE,
        &[&[(0, 0)], &[(1, 0)], &[(2, 0)], &[(1, 0)], &[(0, 0)], &[]],
    );
}

#[test]
fn value_trace_wrapper_validates_dot_values_per_tick() {
    assert_values_trace(
        STRAIGHT_RIGHT,
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
    assert_ticks_to_halt(STRAIGHT_RIGHT, 4, 32);
}
