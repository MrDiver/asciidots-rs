use asciidots_core::interpreter::DotsInterpreter;

#[allow(unused)]
pub type DotPos = (i32, i32);
pub type DotValueAtPos = (i32, i32, f64);

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

fn step_n(interpreter: &mut DotsInterpreter, n: usize) {
    for _ in 0..n {
        interpreter
            .step()
            .expect("interpreter step should not produce runtime errors");
    }
}

#[allow(unused)]
pub fn assert_positions_trace(program: &str, expected: &[&[DotPos]]) {
    let mut interpreter = DotsInterpreter::new(program).expect("program should lex and initialize");

    for (tick, expected_positions) in expected.iter().enumerate() {
        let got = sorted_positions(&interpreter);
        assert_eq!(
            got, *expected_positions,
            "position mismatch at tick {tick}: expected {:?}, got {:?}",
            expected_positions, got
        );
        if tick + 1 < expected.len() {
            step_n(&mut interpreter, 1);
        }
    }
}

pub fn assert_values_trace(program: &str, expected: &[&[DotValueAtPos]]) {
    let mut interpreter = DotsInterpreter::new(program).expect("program should lex and initialize");

    for (tick, expected_values) in expected.iter().enumerate() {
        let got = sorted_position_values(&interpreter);
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
                (*got_value - exp_value).abs() < 1e-9,
                "value mismatch at tick {tick}, index {index}: expected {}, got {}",
                exp_value,
                got_value
            );
        }
        if tick + 1 < expected.len() {
            step_n(&mut interpreter, 1);
        }
    }
}

fn ticks_until_halt(program: &str, max_ticks: usize) -> usize {
    let mut interpreter = DotsInterpreter::new(program).expect("program should lex and initialize");
    let mut ticks = 0usize;
    while interpreter.num_dots() > 0 {
        assert!(
            ticks < max_ticks,
            "program did not halt within {max_ticks} ticks"
        );
        step_n(&mut interpreter, 1);
        ticks += 1;
    }
    ticks
}

pub fn assert_ticks_to_halt(program: &str, expected_ticks: usize, max_ticks: usize) {
    let got_ticks = ticks_until_halt(program, max_ticks);
    assert_eq!(
        got_ticks, expected_ticks,
        "halt tick count mismatch: expected {expected_ticks}, got {got_ticks}"
    );
}
