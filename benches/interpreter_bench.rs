use AsciidotsCompiler::{AsciiField, ProgramState};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn interpreter_step(c: &mut Criterion) {
    let mut program = ProgramState::from_string("samples/superbig.dots");
    c.bench_function("interpreter", |b| {
        b.iter(|| {
            program.step();
        })
    });
}

fn interpreter_full(c: &mut Criterion) {
    let mut program = ProgramState::from_string("samples/superbig.dots");
    c.bench_function("interpreter", |b| {
        b.iter(|| {
            while program.num_dots() > 0 {
                program.step();
            }
        })
    });
}

criterion_group!(benches, interpreter_step, interpreter_full);
criterion_main!(benches);
