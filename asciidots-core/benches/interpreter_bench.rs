use asciidots_core::interpreter::{AsciiField, DotsInterpreter};
use asciidots_core::parser::{AsciiCell, Parser};
use asciidots_core::parser::{Lexer, SpanToken};
use asciidots_core::primitives::{Direction, Position};
use asciidots_core::state_machine::{Dot, DotMachine};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const BASIC: &str = include_str!("../samples/basic.dots");
const MOVEMENT: &str = include_str!("../samples/movement.dots");
const EXPONENTIAL: &str = include_str!("../samples/exponential.dots");
const COUNTER: &str = include_str!("../samples/counter.dots");
const FIBONACCI: &str = include_str!("../samples/fibonacci.dots");
const PRIMES: &str = include_str!("../samples/primes.dots");
const SUPERBIG: &str = include_str!("../samples/superbig.dots");
const DEATHFACTORY: &str = include_str!("../samples/deathfactory.dots");

fn lex_source(source: &str) {
    let lexer = Lexer::new(source);
    let tokens = lexer
        .run()
        .expect("lexing should succeed for benchmark input");
    black_box(tokens);
}

fn bench_lexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexing");
    group.bench_function("basic", |b| b.iter(|| lex_source(black_box(BASIC))));
    group.bench_function("movement", |b| b.iter(|| lex_source(black_box(MOVEMENT))));
    group.bench_function("counter", |b| b.iter(|| lex_source(black_box(COUNTER))));
    group.bench_function("fibonacci", |b| b.iter(|| lex_source(black_box(FIBONACCI))));
    group.bench_function("primes", |b| b.iter(|| lex_source(black_box(PRIMES))));
    group.bench_function("exponential", |b| {
        b.iter(|| lex_source(black_box(EXPONENTIAL)))
    });
    group.bench_function("deathfactory", |b| {
        b.iter(|| lex_source(black_box(DEATHFACTORY)))
    });
    group.finish();
}

fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");
    for (name, src) in &[
        ("basic", BASIC),
        ("movement", MOVEMENT),
        ("counter", COUNTER),
        ("fibonacci", FIBONACCI),
        ("primes", PRIMES),
        ("exponential", EXPONENTIAL),
        ("deathfactory", DEATHFACTORY),
    ] {
        let src = *src;
        group.bench_function(*name, |b| {
            b.iter(|| {
                let source = Lexer::new(src).run().expect("Should work");
                let _parsed = Parser::new(source).run().expect("Should parse fine");
            })
        });
    }
    group.finish();
}

fn bench_single_dot_interpret(c: &mut Criterion) {
    c.bench_function("interpreter_single_dot", |b| {
        b.iter(|| {
            let mut dot = DotMachine::from(Dot::new(Direction::Right, Position::new(10, 10), 0.0));
            let token = AsciiCell::ForceDirection(Direction::Down);
            let spanned = SpanToken::temporary(token);
            let mut pending_dots = Vec::with_capacity(2);

            dot.process(black_box(&spanned), &mut pending_dots)
                .expect("single-step interpreter benchmark should not fail");

            black_box(dot);
            black_box(pending_dots);
        })
    });
}

fn make_initialized_program(source: &str) -> DotsInterpreter {
    DotsInterpreter::from_string(source)
        .expect("program parsing should succeed for benchmark input")
}

/// Step N times without checking for termination.
/// Used for infinite-loop programs (counter, fibonacci, primes) to measure
/// sustained per-step throughput under continuous dot traffic.
fn bench_throughput_1000_steps(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_1000_steps");

    for (name, src) in &[
        ("counter", COUNTER),
        ("fibonacci", FIBONACCI),
        ("primes", PRIMES),
        ("deathfactory", DEATHFACTORY),
    ] {
        let src = *src;
        group.bench_function(*name, |b| {
            b.iter_batched(
                || make_initialized_program(src),
                |mut program| {
                    for _ in 0..1_000 {
                        let _ = program.step();
                    }
                    black_box(program.num_dots());
                },
                BatchSize::SmallInput,
            )
        });
    }

    // Exponential is capped to avoid OOM — at step 10 you already have ~thousands of dots.
    group.bench_function("exponential_10_steps", |b| {
        b.iter_batched(
            || make_initialized_program(EXPONENTIAL),
            |mut program| {
                for _ in 0..10 {
                    let _ = program.step();
                }
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_program_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("program_step");

    group.bench_function("basic_single_prepared_step", |b| {
        b.iter_batched(
            || make_initialized_program(BASIC),
            |mut program| {
                let _ = program.step();
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("movement_single_prepared_step", |b| {
        b.iter_batched(
            || make_initialized_program(MOVEMENT),
            |mut program| {
                let _ = program.step();
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("basic_run_to_completion", |b| {
        b.iter_batched(
            || make_initialized_program(BASIC),
            |mut program| {
                while program.num_dots() > 0 {
                    let _ = program.step();
                }
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("movement_run_to_completion", |b| {
        b.iter_batched(
            || make_initialized_program(MOVEMENT),
            |mut program| {
                while program.num_dots() > 0 {
                    let _ = program.step();
                }
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("exponential_max_10_steps", |b| {
        b.iter_batched(
            || make_initialized_program(EXPONENTIAL),
            |mut program| {
                for _ in 0..10 {
                    if program.num_dots() == 0 {
                        break;
                    }
                    let _ = program.step();
                }
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("deathfactory_single_prepared_step", |b| {
        b.iter_batched(
            || make_initialized_program(DEATHFACTORY),
            |mut program| {
                let _ = program.step();
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_cleanup(c: &mut Criterion) {
    let mut group = c.benchmark_group("cleanup");
    group.measurement_time(std::time::Duration::from_secs(20));
    group.sample_size(10);

    // superbig: 1000 parallel dots walk one step then die — second step compacts them.
    group.bench_function("superbig_compaction", |b| {
        b.iter_batched(
            || {
                let mut program = make_initialized_program(SUPERBIG);
                let _ = program.step();
                program
            },
            |mut program| {
                let _ = program.step();
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    // deathfactory: every step a wave of spawned dots immediately die, so
    // the live+dead mix is maximal and compaction runs at full stress every step.
    group.bench_function("deathfactory_steady_state", |b| {
        b.iter_batched(
            || {
                // Warm up for a few steps to reach steady state dot population.
                let mut program = make_initialized_program(DEATHFACTORY);
                for _ in 0..3 {
                    let _ = program.step();
                }
                program
            },
            |mut program| {
                let _ = program.step();
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_heavy_workloads(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_workloads");
    group.measurement_time(std::time::Duration::from_secs(30));
    group.sample_size(10);

    // superbig lex/parse: slow tokenization on large grid
    group.bench_function("superbig_lex", |b| {
        b.iter(|| lex_source(black_box(SUPERBIG)))
    });

    group.bench_function("superbig_parse", |b| {
        b.iter(|| {
            let parsed = AsciiField::new(black_box(SUPERBIG))
                .expect("parsing should succeed for benchmark input");
            black_box(parsed);
        })
    });

    // superbig has 1000 parallel dots each on an independent pipe — they all walk
    // and then die. Good measure of many-dot step + compaction cost.
    group.bench_function("superbig_run_to_completion", |b| {
        b.iter_batched(
            || make_initialized_program(SUPERBIG),
            |mut program| {
                while program.num_dots() > 0 {
                    let _ = program.step();
                }
                black_box(program.num_dots());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_lexing,
    bench_parsing,
    bench_single_dot_interpret,
    bench_throughput_1000_steps,
    bench_program_step
);
criterion_group!(benches_heavy, bench_cleanup, bench_heavy_workloads);
criterion_main!(benches, benches_heavy);
