use std::{hint::black_box, path::PathBuf};

use asciidots_core::{linker, native::nativelib::default_native_library_for};
use asciidots_graph::{
    compiler::tracer::Tracer,
    ir::dot::GraphDot,
    runtime::executor::GraphExecutor,
};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rustc_hash::FxHashSet;

const STEPS_PER_SAMPLE: usize = 500;

fn build_executor(path: &PathBuf) -> GraphExecutor {
    let available = FxHashSet::<String>::default();
    let linked = linker::link_file(path, &available).expect("link_file failed for benchmark");
    let mut tracer = Tracer::new(linked.module);
    tracer.trace();
    let graph = tracer.into_graph();
    let native_registry = default_native_library_for::<GraphDot>()
        .into_iter()
        .map(|f| (f.name().to_string(), f))
        .collect();
    GraphExecutor::new(graph, native_registry)
}

fn bench_executor_current_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("executor_current_state");

    let complex_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/complex.dots");
    group.bench_function("complex_500_steps", |b| {
        b.iter_batched(
            || build_executor(&complex_path),
            |mut executor| {
                for _ in 0..STEPS_PER_SAMPLE {
                    let _ = executor.step();
                }
                black_box(executor.pending_len());
            },
            BatchSize::SmallInput,
        )
    });

    let juliaset_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph_juliaset_small.dots");
    group.bench_function("juliaset_small_500_steps", |b| {
        b.iter_batched(
            || build_executor(&juliaset_path),
            |mut executor| {
                for _ in 0..STEPS_PER_SAMPLE {
                    let _ = executor.step();
                }
                black_box(executor.pending_len());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_executor_runtime_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("executor_runtime_only");

    let basic_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../samples/basic.dots");
    group.bench_function("basic_500_steps", |b| {
        let mut executor = build_executor(&basic_path);
        for _ in 0..64 {
            let _ = executor.step();
        }
        b.iter(|| {
            for _ in 0..STEPS_PER_SAMPLE {
                let _ = executor.step();
                if executor.is_finished() {
                    executor = build_executor(&basic_path);
                }
            }
            black_box(executor.pending_len());
        })
    });

    let juliaset_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph_juliaset_small.dots");
    group.bench_function("juliaset_small_500_steps", |b| {
        let mut executor = build_executor(&juliaset_path);
        for _ in 0..64 {
            let _ = executor.step();
        }
        b.iter(|| {
            for _ in 0..STEPS_PER_SAMPLE {
                let _ = executor.step();
                if executor.is_finished() {
                    executor = build_executor(&juliaset_path);
                }
            }
            black_box(executor.pending_len());
        })
    });

    group.finish();
}

fn bench_full_juliaset_completion(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_juliaset_completion");
    group.measurement_time(std::time::Duration::from_secs(25));
    group.sample_size(10);

    let juliaset_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../asciidots-core/tests/fixtures/juliaset/juliaset.dots");
    group.bench_function("juliaset_full_10000_steps", |b| {
        b.iter_batched(
            || build_executor(&juliaset_path),
            |mut executor| {
                for _ in 0..10_000 {
                    let _ = executor.step();
                    if executor.is_finished() {
                        break;
                    }
                }
                black_box(executor.pending_len());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_long_running_completion(c: &mut Criterion) {
    let mut group = c.benchmark_group("executor_to_completion");
    group.measurement_time(std::time::Duration::from_secs(60));
    group.sample_size(5);

    // Full juliaset_small from fixtures (more consistent than examples/)
    let juliaset_fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph_juliaset_small.dots");
    group.bench_function("juliaset_small_to_completion", |b| {
        b.iter_batched(
            || build_executor(&juliaset_fixture_path),
            |mut executor| {
                let mut step_count = 0;
                // juliaset runs indefinitely, so we cap at 10k steps
                while !executor.is_finished() && step_count < 10_000 {
                    let _ = executor.step();
                    step_count += 1;
                }
                black_box(step_count)
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_executor_current_state, bench_executor_runtime_only, bench_full_juliaset_completion, bench_long_running_completion);
criterion_main!(benches);
