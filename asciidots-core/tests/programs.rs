mod common;

use std::path::PathBuf;

use asciidots_core::interpreter::{DotsInterpreterBuilder, InterpreterResult};

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut i = 3u64;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

#[test]
fn primesieve_outputs_only_primes_in_order() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/primesieve.dots");
    let mut interpreter = DotsInterpreterBuilder::new()
        .build_from_file(&fixture_path)
        .expect("primesieve fixture should link and initialize");

    let max_ticks = 500_000usize;
    let mut ticks = 0usize;
    let mut printed: Vec<u64> = Vec::new();

    while !interpreter.is_finished() && ticks < max_ticks && printed.len() < 20 {
        let results = interpreter
            .step()
            .expect("primesieve should not produce diagnostics");
        for result in results {
            if let InterpreterResult::PrintLn(s) = result {
                if let Ok(n) = s.trim().parse::<f64>() {
                    printed.push(n as u64);
                }
            }
        }
        ticks += 1;
    }

    assert!(
        !printed.is_empty(),
        "primesieve produced no output within {max_ticks} ticks"
    );

    let expected_primes: Vec<u64> = (2u64..)
        .filter(|&n| is_prime(n))
        .take(printed.len())
        .collect();

    assert_eq!(
        printed, expected_primes,
        "primesieve output {:?} but expected primes {:?}",
        printed, expected_primes
    );
}
