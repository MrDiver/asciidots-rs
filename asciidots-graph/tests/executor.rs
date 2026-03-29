use std::{collections::BTreeMap, path::PathBuf};

use asciidots_core::{
    interpreter::{ColorMode, DisplayCommand, DotsInterpreterBuilder, InterpreterResult},
    linker,
    native::nativelib::default_native_library_for,
};
use asciidots_graph::{
    compiler::tracer::Tracer,
    ir::{dot::GraphDot, graph::NodeType},
    runtime::executor::GraphExecutor,
};
use rustc_hash::FxHashSet;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_executor(path: &PathBuf) -> GraphExecutor {
    let available: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let linked = linker::link_file(path, &available).expect("link_file failed");
    let native_names: FxHashSet<String> = linked.native_imports.keys().cloned().collect();
    let mut tracer = Tracer::new(linked.module, linked.native_imports, native_names);
    tracer.trace();
    let graph = tracer.into_graph();
    let native_registry = default_native_library_for::<GraphDot>()
        .into_iter()
        .map(|f| (f.name().to_string(), f))
        .collect();
    GraphExecutor::new(graph, native_registry)
}

/// The final display state after applying all commands in order.
/// Pixel map is (display_id, pixel_index) → last value written (as bits for exact equality).
#[derive(Debug, Default, PartialEq, Eq)]
struct DisplayState {
    /// (display_id) → last width set
    widths: BTreeMap<u32, usize>,
    /// (display_id) → last height set
    heights: BTreeMap<u32, usize>,
    /// (display_id) → last color mode set (stored as its discriminant)
    color_modes: BTreeMap<u32, u8>,
    /// (display_id, pixel_index) → last value written (f64 bits)
    pixels: BTreeMap<(u32, usize), u64>,
}

fn color_mode_discriminant(m: &ColorMode) -> u8 {
    match m {
        ColorMode::Rgb => 0,
        ColorMode::Hsv => 1,
        ColorMode::Hsl => 2,
        ColorMode::HueOnly => 3,
        ColorMode::BlackAndWhite => 4,
    }
}

impl DisplayState {
    fn apply(&mut self, display_id: u32, cmd: &DisplayCommand) {
        match cmd {
            DisplayCommand::SetWidth(w) => {
                self.widths.insert(display_id, *w);
            }
            DisplayCommand::SetHeight(h) => {
                self.heights.insert(display_id, *h);
            }
            DisplayCommand::SetColorMode(m) => {
                self.color_modes
                    .insert(display_id, color_mode_discriminant(m));
            }
            DisplayCommand::SetPixel { index, value } => {
                self.pixels.insert((display_id, *index), value.to_bits());
            }
        }
    }
}

fn collect_graph(path: &PathBuf, max_ticks: usize) -> DisplayState {
    let mut executor = build_executor(path);
    let mut state = DisplayState::default();
    for _ in 0..max_ticks {
        if executor.is_finished() {
            break;
        }
        for result in executor.step().to_vec() {
            if let InterpreterResult::Display {
                display_id,
                command,
            } = result
            {
                state.apply(display_id, &command);
            }
        }
    }
    state
}

fn collect_core(path: &PathBuf, max_ticks: usize) -> DisplayState {
    let mut interpreter = DotsInterpreterBuilder::new()
        .build_from_file(path)
        .expect("core interpreter failed to initialize");
    let mut state = DisplayState::default();
    for _ in 0..max_ticks {
        if interpreter.is_finished() {
            break;
        }
        let results = interpreter.step().expect("core interpreter step error");
        for result in results {
            if let InterpreterResult::Display {
                display_id,
                command,
            } = result
            {
                state.apply(*display_id, command);
            }
        }
    }
    state
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn display_fixture_emits_correct_events() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/display.dots");
    let mut executor = build_executor(&path);

    let max_ticks = 10_000usize;
    let mut ticks = 0usize;
    let mut set_width: Option<usize> = None;
    let mut color_mode_set = false;
    let mut pixel_values: Vec<f64> = Vec::new();

    while !executor.is_finished() {
        assert!(
            ticks < max_ticks,
            "display fixture did not halt within {max_ticks} ticks"
        );
        for result in executor.step().to_vec() {
            println!("Pending len: {}", executor.pending_len());
            if let InterpreterResult::Display { command, .. } = result {
                match command {
                    DisplayCommand::SetWidth(w) => set_width = Some(w),
                    DisplayCommand::SetColorMode(_) => color_mode_set = true,
                    DisplayCommand::SetPixel { value, .. } => pixel_values.push(value),
                    _ => {}
                }
            }
        }
        ticks += 1;
    }

    assert_eq!(set_width, Some(10), "expected SetWidth(10)");
    assert!(color_mode_set, "expected at least one SetColorMode command");
    assert!(
        !pixel_values.is_empty(),
        "expected at least one SetPixel command"
    );
    println!(
        "display fixture: width={:?}, {} pixels, values={:?}",
        set_width,
        pixel_values.len(),
        pixel_values
    );
}

/// Runs both the core interpreter and the graph executor on display.dots and
/// asserts that the final framebuffer state (last write per pixel index) is identical.
#[test]
fn display_fixture_graph_matches_core() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/display.dots");
    let core_state = collect_core(&path, 10_000);
    let graph_state = collect_graph(&path, 10_000);

    assert_eq!(
        core_state, graph_state,
        "graph executor display output differs from core interpreter\n\
         core:  {core_state:?}\n\
         graph: {graph_state:?}"
    );
}

/// Runs both runtimes on juliaset_small.dots for 1000 steps and asserts the accumulated
/// display state matches (juliaset runs indefinitely so we snapshot at a fixed step count).
#[test]
fn juliaset_small_graph_matches_core() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph_juliaset_small.dots");
    let core_state = collect_core(&path, 1_000);
    let graph_state = collect_graph(&path, 1_000);

    assert_eq!(
        core_state.widths, graph_state.widths,
        "SetWidth mismatch: core={:?} graph={:?}",
        core_state.widths, graph_state.widths
    );
    assert_eq!(
        core_state.heights, graph_state.heights,
        "SetHeight mismatch: core={:?} graph={:?}",
        core_state.heights, graph_state.heights
    );
    assert_eq!(
        core_state.pixels.len(),
        graph_state.pixels.len(),
        "pixel count mismatch: core={} graph={}",
        core_state.pixels.len(),
        graph_state.pixels.len()
    );
    assert_eq!(
        core_state.pixels, graph_state.pixels,
        "pixel values differ between core and graph executor"
    );
    println!(
        "juliaset_small: {}x{}, {} pixels match after 1000 steps",
        core_state.widths.values().next().copied().unwrap_or(0),
        core_state.heights.values().next().copied().unwrap_or(0),
        core_state.pixels.len(),
    );
}

/// Diagnostic: traces both runtimes on juliaset_small for 500 steps, printing a summary
/// every 100 steps of what events each produces. Used to diagnose display output differences.
#[test]
fn juliaset_small_trace() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph_juliaset_small.dots");

    // --- Graph: inspect node composition ---
    {
        let available: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let linked = linker::link_file(&path, &available).expect("link failed");
        let native_names: FxHashSet<String> = linked.native_imports.keys().cloned().collect();
        let mut tracer = Tracer::new(linked.module, linked.native_imports, native_names);
        tracer.trace();
        let graph = tracer.into_graph();
        let mut initial_dots = Vec::new();

        let mut counts: std::collections::BTreeMap<&str, usize> = Default::default();
        for node in &graph.nodes {
            let label = match &node.t {
                NodeType::CreateDot(_) => {
                    initial_dots.push(node);
                    "CreateDot"
                }
                NodeType::EndProgram => "EndProgram",
                NodeType::Print(_) => "Print",
                NodeType::Set(_) => "Set",
                NodeType::Select(_) => "Select",
                NodeType::Duplicate => "Duplicate",
                NodeType::Operator(..) => "Operator",
                NodeType::Conditional(_) => "Conditional",
                NodeType::Filter(_) => "Filter",
                NodeType::Assertion(_) => "Assertion",
                NodeType::Display(_) => "Display",
                NodeType::Call(_) => "Call",
                NodeType::CallEnter => "CallEnter",
                NodeType::Return => "Return",
            };
            *counts.entry(label).or_default() += 1;
        }
        println!(
            "=== Graph node counts ({} total, {} edges) ===",
            graph.nodes.len(),
            graph.edges.len()
        );
        for (label, count) in &counts {
            println!("  {label}: {count}");
        }
        println!("Initial Dots:");
        for dot in &initial_dots {
            println!("{dot:?}");
        }
    }

    // --- Core trace ---
    {
        let mut interpreter = DotsInterpreterBuilder::new()
            .build_from_file(&path)
            .expect("core init failed");

        let initial_dots = interpreter.get_dots();

        println!(
            "Initial Dots ({}) : {:#?}",
            initial_dots.len(),
            initial_dots
        );

        let mut print_chars = 0u64;
        let mut display_events = 0u64;
        let mut display_by_kind = [0u64; 4]; // SetWidth, SetHeight, SetColorMode, SetPixel

        println!("\n=== Core interpreter (500 steps) ===");
        for step in 0..500usize {
            if interpreter.is_finished() {
                println!("  finished at step {step}");
                break;
            }
            let num_dots = interpreter.num_dots();
            let results = interpreter.step().expect("core step error");
            for r in results {
                match r {
                    InterpreterResult::PrintChar(_) | InterpreterResult::PrintLn(_) => {
                        print_chars += 1
                    }
                    InterpreterResult::Display { command, .. } => {
                        display_events += 1;
                        match command {
                            DisplayCommand::SetWidth(_) => display_by_kind[0] += 1,
                            DisplayCommand::SetHeight(_) => display_by_kind[1] += 1,
                            DisplayCommand::SetColorMode(_) => display_by_kind[2] += 1,
                            DisplayCommand::SetPixel { .. } => display_by_kind[3] += 1,
                        }
                    }
                    _ => {}
                }
            }
            if (step + 1) % 10 == 0 {
                println!(
                    "  step {:>3}: dots={num_dots} print_chars={print_chars} display={display_events} (width={} height={} colormode={} pixel={})",
                    step + 1,
                    display_by_kind[0],
                    display_by_kind[1],
                    display_by_kind[2],
                    display_by_kind[3]
                );
            }
        }
    }

    // --- Graph trace ---
    {
        let mut executor = build_executor(&path);

        let mut print_chars = 0u64;
        let mut display_events = 0u64;
        let mut display_by_kind = [0u64; 4];

        println!("\n=== Graph executor (500 steps) ===");
        for step in 0..500usize {
            if executor.is_finished() {
                println!("  finished at step {step}");
                break;
            }
            let pending_before = executor.pending_len();
            for r in executor.step().to_vec() {
                match r {
                    InterpreterResult::PrintChar(_) | InterpreterResult::PrintLn(_) => {
                        print_chars += 1
                    }
                    InterpreterResult::Display { command, .. } => {
                        display_events += 1;
                        match command {
                            DisplayCommand::SetWidth(_) => display_by_kind[0] += 1,
                            DisplayCommand::SetHeight(_) => display_by_kind[1] += 1,
                            DisplayCommand::SetColorMode(_) => display_by_kind[2] += 1,
                            DisplayCommand::SetPixel { .. } => display_by_kind[3] += 1,
                        }
                    }
                    _ => {}
                }
            }
            if (step + 1) % 10 == 0 {
                println!(
                    "  step {:>3}: pending={pending_before:>6} print_chars={print_chars} display={display_events} (width={} height={} colormode={} pixel={})",
                    step + 1,
                    display_by_kind[0],
                    display_by_kind[1],
                    display_by_kind[2],
                    display_by_kind[3]
                );
            }
        }
    }
}
