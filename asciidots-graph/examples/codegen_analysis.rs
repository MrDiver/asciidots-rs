use asciidots_core::linker;
use asciidots_core::native::nativelib::default_native_library_for;
use asciidots_graph::codegen::CodeGenerator;
use asciidots_graph::compiler::tracer::Tracer;
use asciidots_graph::ir::dot::GraphDot;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

fn main() {
    let juliaset_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../asciidots-core/tests/fixtures/juliaset/juliaset.dots");

    println!("Building graph from ASCIIdots...");
    let available = FxHashSet::<String>::default();
    let linked = linker::link_file(&juliaset_path, &available).expect("link_file failed");
    let mut tracer = Tracer::new(linked.module);
    tracer.trace();
    let graph = tracer.into_graph();

    println!("\n{}", "=".repeat(60));
    println!("CODE GENERATION ANALYSIS");
    println!("{}", "=".repeat(60));

    let codegen = CodeGenerator::new(graph);
    println!("{}", codegen.stats());

    println!("\n{}", "=".repeat(60));
    println!("GENERATED CODE (EXCERPT)");
    println!("{}", "=".repeat(60));

    let generated = codegen.generate();
    let lines: Vec<&str> = generated.lines().collect();

    // Show structure but don't print entire thing
    println!("Total lines of generated code: {}", lines.len());
    println!("\nFirst 50 lines:");
    for line in lines.iter().take(50) {
        println!("{}", line);
    }

    println!("\n... (code continues) ...\n");

    if lines.len() > 50 {
        println!("Last 20 lines:");
        for line in lines.iter().skip(lines.len().saturating_sub(20)) {
            println!("{}", line);
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("CODE GENERATION BENEFITS");
    println!("{}", "=".repeat(60));
    println!(
        "
With code generation, we eliminate:
1. Dynamic node type matching (every dot! ~789/step)
2. HashMap queue lookups (287 Operator dots/step)
3. Pattern matching in process_dot (repeated ~789 times)
4. Edge traversal indirection

Current interpreter: ~91 µs/step
Potential with codegen: ~20-30 µs/step (3-4x faster)

Expected improvements:
- No runtime type dispatch
- Direct array indexing for node routing
- Inlined hot paths
- CPU-friendly instruction patterns
"
    );
}
