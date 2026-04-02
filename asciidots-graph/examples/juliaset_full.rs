use std::time::Instant;
use std::path::PathBuf;

use asciidots_core::linker;
use asciidots_core::native::nativelib::default_native_library_for;
use asciidots_graph::compiler::tracer::Tracer;
use asciidots_graph::ir::dot::GraphDot;
use asciidots_graph::runtime::executor::GraphExecutor;
use rustc_hash::FxHashSet;

fn build_executor(path: &PathBuf) -> GraphExecutor {
    let available = FxHashSet::<String>::default();
    let linked = linker::link_file(path, &available).expect("link_file failed");
    let mut tracer = Tracer::new(linked.module);
    tracer.trace();
    let max_pending = tracer.get_max_pending();
    let graph = tracer.into_graph();
    let native_registry = default_native_library_for::<GraphDot>()
        .into_iter()
        .map(|f| (f.name().to_string(), f))
        .collect();
    GraphExecutor::new(graph, native_registry, Some(max_pending))
}

fn main() {
    // Test with fixture juliaset_small
    let small_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/graph_juliaset_small.dots");
    
    println!("Benchmarking juliaset_small.dots to 10,000 steps...");
    let mut executor = build_executor(&small_path);
    let start = Instant::now();
    let mut step_count = 0;
    while !executor.is_finished() && step_count < 10_000 {
        let _ = executor.step();
        step_count += 1;
    }
    let elapsed = start.elapsed();
    println!(
        "✓ Completed {} steps in {:.3}s ({:.1} steps/sec, {:.2} µs/step)",
        step_count,
        elapsed.as_secs_f64(),
        step_count as f64 / elapsed.as_secs_f64(),
        elapsed.as_micros() as f64 / step_count as f64
    );
}
