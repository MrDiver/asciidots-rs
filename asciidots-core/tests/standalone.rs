mod common;

use asciidots_core::interpreter::{DotsInterpreter, InterpreterCommand, InterpreterResult};
use asciidots_core::state_machine::RuntimeDiagnostic;

// .----@?--="2"--&
// @? routes input → addr register. value stays 0.
// ="2" asserts value == 2. Fails because input did NOT corrupt value.
#[test]
fn addr_input_does_not_set_value_register() {
    let program = r#".----@?--="2"--&"#;
    let mut interpreter =
        DotsInterpreter::from_string(program).expect("program should lex and initialize");

    let max_ticks = 128usize;
    let mut ticks = 0usize;
    let mut input_sent = false;
    let mut has_assertion_failure = false;

    while !interpreter.is_finished() {
        assert!(
            ticks < max_ticks,
            "program did not halt within {max_ticks} ticks"
        );

        let mut pending_input: Option<u64> = None;

        match interpreter.step() {
            Ok(results) => {
                for result in results {
                    if let InterpreterResult::InputRequest(uuid) = result {
                        assert!(!input_sent, "unexpected second input request");
                        pending_input = Some(*uuid);
                        input_sent = true;
                    }
                }
            }
            Err(diagnostics) => {
                has_assertion_failure |= diagnostics
                    .iter()
                    .any(|d| matches!(d, RuntimeDiagnostic::AssertionFailed { .. }));
                break;
            }
        }

        if let Some(uuid) = pending_input {
            interpreter.handle_command(InterpreterCommand::InputAnswer(uuid, 2.0));
        }

        ticks += 1;
    }

    assert!(input_sent, "interpreter never requested input");
    assert!(
        has_assertion_failure,
        "expected AssertionFailed diagnostic — addr input must not bleed into value register"
    );
}
