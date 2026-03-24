mod common;

use asciidots_core::interpreter::DotsInterpreter;
use common::trace_harness::{assert_interpreter_ticks_to_halt, assert_values_trace};

const OP_ADD_HORIZONTAL: &str = include_str!("fixtures/operator_add_horizontal.dots");
const OP_ADD_VERTICAL: &str = include_str!("fixtures/operator_add_vertical.dots");
const OP_SUB_HORIZONTAL: &str = include_str!("fixtures/operator_sub_horizontal.dots");

fn interpreter_from(program: &str) -> DotsInterpreter {
    DotsInterpreter::from_string(program).expect("program should lex and initialize")
}

#[test]
fn operator_add_horizontal_values_trace() {
    let mut interpreter = interpreter_from(OP_ADD_HORIZONTAL);
    assert_values_trace(
        &mut interpreter,
        &[
            &[(0, 0, 0.0), (0, 1, 0.0)],
            &[(1, 0, 0.0), (1, 1, 0.0)],
            &[(2, 0, 0.0), (2, 1, 0.0)],
            &[(3, 0, 0.0), (3, 1, 0.0)],
            &[(4, 0, 2.0), (4, 1, 5.0)],
            &[(5, 0, 2.0), (5, 1, 5.0)],
            &[(6, 1, 5.0)],
            &[(6, 0, 5.0)],
            &[(7, 0, 7.0)],
            &[(8, 0, 7.0)],
            &[(9, 0, 7.0)],
            &[(10, 0, 7.0)],
            &[],
        ],
    );
}

#[test]
fn operator_add_horizontal_ticks_to_halt() {
    let mut interpreter = interpreter_from(OP_ADD_HORIZONTAL);
    assert_interpreter_ticks_to_halt(&mut interpreter, 12, 128);
}

#[test]
fn operator_add_vertical_values_trace() {
    let mut interpreter = interpreter_from(OP_ADD_VERTICAL);
    assert_values_trace(
        &mut interpreter,
        &[
            &[(0, 2, 0.0), (0, 3, 0.0)],
            &[(1, 2, 0.0), (1, 3, 0.0)],
            &[(2, 2, 0.0), (2, 3, 0.0)],
            &[(3, 2, 0.0), (3, 3, 0.0)],
            &[(4, 2, 2.0), (4, 3, 5.0)],
            &[(5, 2, 2.0), (5, 3, 5.0)],
            &[(6, 3, 5.0)],
            &[(6, 2, 5.0)],
            &[(6, 1, 7.0)],
            &[(6, 0, 7.0)],
            &[],
        ],
    );
}

#[test]
fn operator_add_vertical_ticks_to_halt() {
    let mut interpreter = interpreter_from(OP_ADD_VERTICAL);
    assert_interpreter_ticks_to_halt(&mut interpreter, 10, 128);
}

#[test]
fn operator_sub_horizontal_values_trace() {
    let mut interpreter = interpreter_from(OP_SUB_HORIZONTAL);
    assert_values_trace(
        &mut interpreter,
        &[
            &[(0, 0, 0.0), (0, 1, 0.0)],
            &[(1, 0, 0.0), (1, 1, 0.0)],
            &[(2, 0, 0.0), (2, 1, 0.0)],
            &[(3, 0, 0.0), (3, 1, 0.0)],
            &[(4, 0, 0.0), (4, 1, 0.0)],
            &[(5, 0, 2.0), (5, 1, 5.0)],
            &[(6, 0, 2.0), (6, 1, 5.0)],
            &[(7, 1, 5.0)],
            &[(7, 0, 5.0)],
            &[(8, 0, -3.0)],
            &[(9, 0, -3.0)],
            &[(10, 0, -3.0)],
            &[(11, 0, -3.0)],
            &[],
        ],
    );
}

#[test]
fn operator_sub_horizontal_ticks_to_halt() {
    let mut interpreter = interpreter_from(OP_SUB_HORIZONTAL);
    assert_interpreter_ticks_to_halt(&mut interpreter, 13, 128);
}
