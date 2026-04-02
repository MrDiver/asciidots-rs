use std::{collections::BTreeMap, path::PathBuf};

use asciidots_core::{
    interpreter::{ColorMode, DisplayCommand, DotsInterpreterBuilder, InterpreterResult},
    linker,
    native::nativelib::default_native_library_for,
};
use asciidots_graph::{
    compiler::tracer::Tracer,
    ir::{dot::GraphDot, graph::{Graph, NodeType}},
    runtime::executor::GraphExecutor,
};
use rustc_hash::FxHashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DisplayEventRecord {
    step: usize,
    display_id: u32,
    command: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn build_graph(path: &PathBuf) -> Graph {
    let available = FxHashSet::<String>::default();
    let linked = linker::link_file(path, &available).expect("link_file failed");
    let mut tracer = Tracer::new(linked.module);
    tracer.trace();
    tracer.into_graph()
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

#[derive(Debug, Default, PartialEq, Eq)]
struct DisplayEventCounts {
    widths: usize,
    heights: usize,
    color_modes: usize,
    pixels: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct NodeTypeCounts {
    create_dot: usize,
    end_program: usize,
    print: usize,
    set: usize,
    select: usize,
    duplicate: usize,
    operator: usize,
    conditional: usize,
    filter: usize,
    assertion: usize,
    display: usize,
    call: usize,
    call_enter: usize,
    r#return: usize,
}

impl NodeTypeCounts {
    fn add(&mut self, node_type: &NodeType) {
        match node_type {
            NodeType::CreateDot(_) => self.create_dot += 1,
            NodeType::EndProgram => self.end_program += 1,
            NodeType::Print(_) => self.print += 1,
            NodeType::Set(_) => self.set += 1,
            NodeType::Select(_) => self.select += 1,
            NodeType::Duplicate => self.duplicate += 1,
            NodeType::Operator(..) => self.operator += 1,
            NodeType::Conditional(_) => self.conditional += 1,
            NodeType::Filter(_) => self.filter += 1,
            NodeType::Assertion(_) => self.assertion += 1,
            NodeType::Display(_) => self.display += 1,
            NodeType::Call(_) => self.call += 1,
            NodeType::CallEnter => self.call_enter += 1,
            NodeType::Return => self.r#return += 1,
        }
    }

    fn labeled_counts(self) -> [(&'static str, usize); 14] {
        [
            ("Assertion", self.assertion),
            ("Call", self.call),
            ("CallEnter", self.call_enter),
            ("Conditional", self.conditional),
            ("CreateDot", self.create_dot),
            ("Display", self.display),
            ("Duplicate", self.duplicate),
            ("EndProgram", self.end_program),
            ("Filter", self.filter),
            ("Operator", self.operator),
            ("Print", self.print),
            ("Return", self.r#return),
            ("Select", self.select),
            ("Set", self.set),
        ]
    }
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

fn collect_graph_counts(path: &PathBuf, max_ticks: usize) -> DisplayEventCounts {
    let mut executor = build_executor(path);
    let mut counts = DisplayEventCounts::default();
    for _ in 0..max_ticks {
        if executor.is_finished() {
            break;
        }
        for result in executor.step().to_vec() {
            if let InterpreterResult::Display { command, .. } = result {
                match command {
                    DisplayCommand::SetWidth(_) => counts.widths += 1,
                    DisplayCommand::SetHeight(_) => counts.heights += 1,
                    DisplayCommand::SetColorMode(_) => counts.color_modes += 1,
                    DisplayCommand::SetPixel { .. } => counts.pixels += 1,
                }
            }
        }
    }
    counts
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

fn collect_graph_output(path: &PathBuf, max_ticks: usize) -> Vec<String> {
    let mut executor = build_executor(path);
    let mut output = Vec::new();
    for _ in 0..max_ticks {
        if executor.is_finished() {
            break;
        }
        for result in executor.step().to_vec() {
            match result {
                InterpreterResult::PrintChar(c) => output.push(c.to_string()),
                InterpreterResult::PrintLn(line) => output.push(line.to_string()),
                _ => {}
            }
        }
    }
    output
}

fn collect_core_output(path: &PathBuf, max_ticks: usize) -> Vec<String> {
    let mut interpreter = DotsInterpreterBuilder::new()
        .build_from_file(path)
        .expect("core interpreter failed to initialize");
    let mut output = Vec::new();
    for _ in 0..max_ticks {
        if interpreter.is_finished() {
            break;
        }
        let results = interpreter.step().expect("core interpreter step error");
        for result in results {
            match result {
                InterpreterResult::PrintChar(c) => output.push(c.to_string()),
                InterpreterResult::PrintLn(line) => output.push(line.to_string()),
                _ => {}
            }
        }
    }
    output
}

fn collect_core_counts(path: &PathBuf, max_ticks: usize) -> DisplayEventCounts {
    let mut interpreter = DotsInterpreterBuilder::new()
        .build_from_file(path)
        .expect("core interpreter failed to initialize");
    let mut counts = DisplayEventCounts::default();
    for _ in 0..max_ticks {
        if interpreter.is_finished() {
            break;
        }
        let results = interpreter.step().expect("core interpreter step error");
        for result in results {
            if let InterpreterResult::Display { command, .. } = result {
                match command {
                    DisplayCommand::SetWidth(_) => counts.widths += 1,
                    DisplayCommand::SetHeight(_) => counts.heights += 1,
                    DisplayCommand::SetColorMode(_) => counts.color_modes += 1,
                    DisplayCommand::SetPixel { .. } => counts.pixels += 1,
                }
            }
        }
    }
    counts
}

fn format_display_command(command: &DisplayCommand) -> String {
    match command {
        DisplayCommand::SetWidth(v) => format!("SetWidth({v})"),
        DisplayCommand::SetHeight(v) => format!("SetHeight({v})"),
        DisplayCommand::SetColorMode(mode) => format!("SetColorMode({mode:?})"),
        DisplayCommand::SetPixel { index, value } => {
            format!("SetPixel(index={index}, value={value})")
        }
    }
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

#[test]
fn duplicate_sample_graph_matches_core_output() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../samples/duplicate.dots");
    let core_output = collect_core_output(&path, 1_000);
    let graph_output = collect_graph_output(&path, 1_000);

    assert_eq!(
        core_output, graph_output,
        "duplicate sample output mismatch\ncore={core_output:?}\ngraph={graph_output:?}"
    );
}

#[test]
fn duplicate_one_dir_sample_graph_matches_core_output() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../samples/duplicate_one_dir.dots");
    let core_output = collect_core_output(&path, 1_000);
    let graph_output = collect_graph_output(&path, 1_000);

    assert_eq!(
        core_output, graph_output,
        "duplicate_one_dir sample output mismatch\ncore={core_output:?}\ngraph={graph_output:?}"
    );
}

#[test]
fn juliaset_small_print_output_matches_core_first_500_steps() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph_juliaset_small.dots");
    let core_output = collect_core_output(&path, 500);
    let graph_output = collect_graph_output(&path, 500);

    assert_eq!(
        core_output, graph_output,
        "print output mismatch in first 500 steps\ncore={core_output:?}\ngraph={graph_output:?}"
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

#[test]
fn juliaset_small_event_counts_match_expected() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph_juliaset_small.dots");
    let core_counts = collect_core_counts(&path, 5_000);
    let graph_counts = collect_graph_counts(&path, 5_000);

    let expected = DisplayEventCounts {
        widths: 1,
        heights: 1,
        color_modes: 1,
        pixels: 100,
    };

    assert_eq!(core_counts, expected, "core event counts changed unexpectedly");
    assert_eq!(graph_counts, expected, "graph event counts changed unexpectedly");
}

/// Diagnostic: traces both runtimes on juliaset_small for 500 steps, printing a summary
/// every 100 steps of what events each produces. Used to diagnose display output differences.
#[test]
fn juliaset_small_trace() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph_juliaset_small.dots");
    let mut core_events: Vec<DisplayEventRecord> = Vec::new();
    let mut graph_events: Vec<DisplayEventRecord> = Vec::new();

    // --- Graph: inspect node composition ---
    {
        let available = FxHashSet::<String>::default();
        let linked = linker::link_file(&path, &available).expect("link failed");
        let mut tracer = Tracer::new(linked.module);
        tracer.trace();
        let graph = tracer.into_graph();
        let mut initial_dots = Vec::new();

        let mut counts = NodeTypeCounts::default();
        for node in &graph.nodes {
            match &node.t {
                NodeType::CreateDot(_) => {
                    initial_dots.push(node);
                }
                _ => {}
            }
            counts.add(&node.t);
        }
        println!(
            "=== Graph node counts ({} total, {} edges) ===",
            graph.nodes.len(),
            graph.edges.len()
        );
        for (label, count) in counts.labeled_counts() {
            println!("  {label}: {count}");
        }
        println!("Initial Dots:");
        for dot in &initial_dots {
            println!("{dot:?}");
        }

        if let Some((display_id, _)) = graph
            .nodes
            .iter()
            .enumerate()
            .find(|(_, node)| matches!(node.t, NodeType::Display(_)))
        {
            println!("Display node: #{display_id} {:?}", graph.nodes[display_id]);
            println!("Display predecessors:");
            for edge in graph.edges.iter().filter(|edge| edge.to.0 == display_id) {
                println!(
                    "  from node#{} {:?} via {:?} ticks={} -> {:?}",
                    edge.from.0,
                    graph.nodes[edge.from.0],
                    edge.from.1,
                    edge.ticks,
                    edge.to.1
                );
            }
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
                    InterpreterResult::Display {
                        display_id,
                        command,
                    } => {
                        display_events += 1;
                        core_events.push(DisplayEventRecord {
                            step: step + 1,
                            display_id: *display_id,
                            command: format_display_command(command),
                        });
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
            if (step + 1) % 100 == 0 {
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
        let mut first_empty_pending_step: Option<usize> = None;

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
                    InterpreterResult::Display {
                        display_id,
                        command,
                    } => {
                        display_events += 1;
                        graph_events.push(DisplayEventRecord {
                            step: step + 1,
                            display_id,
                            command: format_display_command(&command),
                        });
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
            if (step + 1) % 100 == 0 {
                println!(
                    "  step {:>3}: pending={pending_before:>6} print_chars={print_chars} display={display_events} (width={} height={} colormode={} pixel={})",
                    step + 1,
                    display_by_kind[0],
                    display_by_kind[1],
                    display_by_kind[2],
                    display_by_kind[3]
                );
            }
            if executor.pending_len() == 0 && first_empty_pending_step.is_none() {
                first_empty_pending_step = Some(step + 1);
                println!("  graph pending drained at step {}", step + 1);
                println!("  recent node visits:");
                for visit in executor.debug_recent_visits().iter().rev().take(20).rev() {
                    println!(
                        "    step={} {} dir={:?} value={} addr={}",
                        visit.step,
                        executor.debug_node_summary(visit.node_id),
                        visit.direction,
                        visit.selected,
                        visit.addr
                    );
                }
                println!("  recent dead ends:");
                for dead_end in executor.debug_recent_dead_ends().iter().rev().take(20).rev() {
                    let (incoming, outgoing) = executor.debug_node_ports(dead_end.from_node);
                    println!(
                        "    step={} {} dir={:?} value={} addr={} incoming={incoming:?} outgoing={outgoing:?}",
                        dead_end.step,
                        executor.debug_node_summary(dead_end.from_node),
                        dead_end.direction,
                        dead_end.selected,
                        dead_end.addr
                    );
                }
            }
        }
    }

    println!("\n=== Display event comparison ===");
    println!("core events: {}", core_events.len());
    println!("graph events: {}", graph_events.len());
    for (idx, (core, graph)) in core_events.iter().zip(graph_events.iter()).enumerate() {
        if core != graph {
            println!("first differing shared event at index {idx}:");
            println!("  core : {core:?}");
            println!("  graph: {graph:?}");
            break;
        }
    }
    if graph_events.len() < core_events.len() {
        println!(
            "graph stopped early at event {} while core continued",
            graph_events.len()
        );
        if let Some(next_core) = core_events.get(graph_events.len()) {
            println!("  next core event: {next_core:?}");
        }
    }
    println!("last core events:");
    for event in core_events.iter().rev().take(10).rev() {
        println!("  {event:?}");
    }
    println!("last graph events:");
    for event in graph_events.iter().rev().take(10).rev() {
        println!("  {event:?}");
    }
}

#[test]
fn testmem_tracing_keeps_callenter_and_return_separate_and_finishes_quickly() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/testmem.dots");

    let graph = build_graph(&path);

    let call_enter = graph
        .nodes
        .iter()
        .enumerate()
        .find(|(_, node)| matches!(node.t, NodeType::CallEnter))
        .expect("expected a CallEnter node for imported mem function");
    let return_node = graph
        .nodes
        .iter()
        .enumerate()
        .find(|(_, node)| matches!(node.t, NodeType::Return))
        .expect("expected a Return node for imported mem function");

    assert_ne!(call_enter.0, return_node.0, "CallEnter and Return must be distinct nodes");
    assert_eq!(
        graph.nodes[call_enter.0].position,
        graph.nodes[return_node.0].position,
        "CallEnter and Return should share the same function entry position"
    );

    let mut core = DotsInterpreterBuilder::new()
        .build_from_file(&path)
        .expect("core interpreter failed to initialize");
    let mut executor = build_executor(&path);

    for _ in 0..100 {
        if core.is_finished() && executor.is_finished() {
            break;
        }
        if !core.is_finished() {
            core.step().expect("core interpreter step failed");
        }
        if !executor.is_finished() {
            executor.step();
        }
    }

    assert!(core.is_finished(), "core interpreter should finish within 100 steps");
    assert!(executor.is_finished(), "graph executor should finish within 100 steps");
}
