use std::{collections::BTreeSet, path::PathBuf};

use asciidots_core::{
    linker,
    parser::{Lexer, Parser},
};
use asciidots_graph::{
    compiler::tracer::Tracer,
    ir::graph::{Graph, NodeType, PortType, PrintKind, SetKind},
};
use rustc_hash::FxHashSet;

fn build_graph(program: &str) -> Graph {
    let source = Lexer::new(program).run().expect("lex failed");
    let module = Parser::new(source).run().expect("parse failed");
    let mut tracer = Tracer::new(module, std::iter::empty(), FxHashSet::default());
    tracer.trace();
    tracer.into_graph()
}

fn build_linked_graph(path: &PathBuf) -> Graph {
    let available = BTreeSet::new();
    let linked = linker::link_file(path, &available).expect("link_file failed");
    let native_names: FxHashSet<String> = linked.native_imports.keys().cloned().collect();
    let mut tracer = Tracer::new(linked.module, linked.native_imports, native_names);
    tracer.trace();
    tracer.into_graph()
}

fn count_node_types_all(
    graph: &Graph,
) -> (
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
) {
    let mut create = 0;
    let mut end = 0;
    let mut print = 0;
    let mut set = 0;
    let mut select = 0;
    let mut duplicate = 0;
    let mut operator = 0;
    let mut conditional = 0;
    let mut filter = 0;
    let mut assertion = 0;
    let mut display = 0;
    let mut callish = 0;

    for node in &graph.nodes {
        match &node.t {
            NodeType::CreateDot(_) => create += 1,
            NodeType::EndProgram => end += 1,
            NodeType::Print(_) => print += 1,
            NodeType::Set(_) => set += 1,
            NodeType::Select(_) => select += 1,
            NodeType::Duplicate => duplicate += 1,
            NodeType::Operator(..) => operator += 1,
            NodeType::Conditional(_) => conditional += 1,
            NodeType::Filter(_) => filter += 1,
            NodeType::Assertion(_) => assertion += 1,
            NodeType::Display(_) => display += 1,
            NodeType::Call(_) | NodeType::CallEnter | NodeType::Return => callish += 1,
        }
    }

    (
        create,
        end,
        print,
        set,
        select,
        duplicate,
        operator,
        conditional,
        filter,
        assertion,
        display,
        callish,
    )
}

fn count_node_types(graph: &Graph) -> (usize, usize, usize, usize, usize, usize) {
    let mut create = 0;
    let mut end = 0;
    let mut print = 0;
    let mut set = 0;
    let mut select = 0;
    let mut duplicate = 0;
    for node in &graph.nodes {
        match &node.t {
            NodeType::CreateDot(_) => create += 1,
            NodeType::EndProgram => end += 1,
            NodeType::Print(_) => print += 1,
            NodeType::Set(_) => set += 1,
            NodeType::Select(_) => select += 1,
            NodeType::Duplicate => duplicate += 1,
            _ => {}
        }
    }
    (create, end, print, set, select, duplicate)
}

// .-&
// Simple: dot straight right into EndProgram
#[test]
fn simple_path_creates_two_nodes_one_edge() {
    let g = build_graph(".-&");
    assert_eq!(g.nodes.len(), 2, "expected CreateDot + EndProgram");
    assert_eq!(g.edges.len(), 1);
    let (create, end, ..) = count_node_types(&g);
    assert_eq!(create, 1);
    assert_eq!(end, 1);
}

// .-#5-&
// Set value 5.0 into dot's value register
#[test]
fn set_value_creates_set_node() {
    let g = build_graph(".-#5-&");
    let (create, end, _, set, ..) = count_node_types(&g);
    assert_eq!(create, 1);
    assert_eq!(set, 1);
    assert_eq!(end, 1);
    assert_eq!(g.edges.len(), 2);

    let set_node = g.nodes.iter().find(|n| matches!(n.t, NodeType::Set(_))).unwrap();
    if let NodeType::Set(slots) = &set_node.t {
        // dot travels Right → dir_index(Right) = 3
        assert!(matches!(slots[3], Some(SetKind::Value(v)) if v == 5.0));
    }
}

// .-$-&
// Standalone $ (no modifier follows) must be a passthrough — no Print node created
#[test]
fn standalone_dollar_is_passthrough() {
    let g = build_graph(".-$-&");
    let (create, end, print, ..) = count_node_types(&g);
    assert_eq!(create, 1);
    assert_eq!(end, 1);
    assert_eq!(print, 0, "bare $ should not create a Print node");
    assert_eq!(g.nodes.len(), 2);
}

// .-$#-&
// $# → Print with ValueMarker modifier
#[test]
fn print_value_marker_creates_print_node() {
    let g = build_graph(".-$#-&");
    let (create, end, print, ..) = count_node_types(&g);
    assert_eq!(create, 1);
    assert_eq!(print, 1);
    assert_eq!(end, 1);
    assert_eq!(g.edges.len(), 2);

    let print_node = g.nodes.iter().find(|n| matches!(n.t, NodeType::Print(_))).unwrap();
    if let NodeType::Print(slots) = &print_node.t {
        assert!(matches!(slots[3], Some(PrintKind::Value)));
    }
}

// .-$"hi"-&
// Print with quoted string modifier
#[test]
fn print_string_modifier_stores_string() {
    let g = build_graph(".-$\"hi\"-&");
    let print_node = g.nodes.iter().find(|n| matches!(n.t, NodeType::Print(_))).unwrap();
    if let NodeType::Print(slots) = &print_node.t {
        assert!(matches!(&slots[3], Some(PrintKind::Normal(s)) if s == "hi"));
    }
}

// .-#---&
// # not followed by a digit/read/quote and not followed by an operator-accepting cell
// → no Set and no Select node; treated as passthrough
#[test]
fn value_marker_without_operand_or_selector_is_passthrough() {
    let g = build_graph(".-#---&");
    let (create, end, _, set, select, _) = count_node_types(&g);
    assert_eq!(create, 1);
    assert_eq!(end, 1);
    assert_eq!(set, 0, "no Set without digits");
    assert_eq!(select, 0, "no Select when next cell doesn't accept a selector");
    assert_eq!(g.nodes.len(), 2);
}

// .-*-&    (with nothing above/below so Up/Down die)
// Duplicate spawns 3 outgoing dots; only the Right one reaches &
#[test]
fn duplicate_node_spawns_multiple_outgoing_edges() {
    let g = build_graph(".-*-&");
    let (create, end, _, _, _, dup) = count_node_types(&g);
    assert_eq!(create, 1);
    assert_eq!(dup, 1);
    assert_eq!(end, 1);
    // CreateDot→Duplicate + Duplicate→EndProgram (Up and Down die out of bounds)
    assert_eq!(g.edges.len(), 2);
}

// .-$@-&
// $@ → Print with AddrMarker modifier
#[test]
fn print_addr_marker_creates_print_node() {
    let g = build_graph(".-$@-&");
    let print_node = g.nodes.iter().find(|n| matches!(n.t, NodeType::Print(_))).unwrap();
    if let NodeType::Print(slots) = &print_node.t {
        assert!(matches!(slots[3], Some(PrintKind::Addr)));
    }
}

// .-#?-&
// #? → Set with Input (read from stdin)
#[test]
fn set_read_creates_input_set_kind() {
    let g = build_graph(".-#?-&");
    let set_node = g.nodes.iter().find(|n| matches!(n.t, NodeType::Set(_))).unwrap();
    if let NodeType::Set(slots) = &set_node.t {
        assert!(matches!(slots[3], Some(SetKind::Input)));
    }
}

// Two dots in same program share no nodes
// .---&
// .---&
#[test]
fn two_independent_paths_produce_separate_nodes() {
    let g = build_graph(".---&\n.---&");
    let (create, end, ..) = count_node_types(&g);
    assert_eq!(create, 2);
    assert_eq!(end, 2);
    assert_eq!(g.edges.len(), 2);
}

#[test]
fn call_enter_collects_multiple_entry_directions_for_imported_function() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/call_enter_multi_direction_for.dots");
    let g = build_linked_graph(&path);

    let call_enter = g
        .nodes
        .iter()
        .enumerate()
        .find(|(_, node)| matches!(node.t, NodeType::CallEnter))
        .expect("expected imported @for CallEnter")
        .0;

    let outgoing: FxHashSet<PortType> = g
        .edges
        .iter()
        .filter(|edge| edge.from.0 == call_enter)
        .map(|edge| edge.from.1)
        .collect();

    assert!(
        outgoing.contains(&PortType::EAST),
        "expected Right-entry function body path from CallEnter, got {outgoing:?}"
    );
    assert!(
        outgoing.contains(&PortType::SOUTH),
        "expected Down-entry function body path from CallEnter, got {outgoing:?}"
    );
}

#[test]
fn conditional_node_traces_north_redirect_path() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/graph_conditional_false_goes_north.dots");
    let g = build_linked_graph(&path);

    let mut found_north_redirect = false;
    for (node_id, node) in g.nodes.iter().enumerate() {
        if !matches!(node.t, NodeType::Conditional(_)) {
            continue;
        }
        let outgoing: FxHashSet<PortType> = g
            .edges
            .iter()
            .filter(|edge| edge.from.0 == node_id)
            .map(|edge| edge.from.1)
            .collect();
        if outgoing.contains(&PortType::NORTH) {
            found_north_redirect = true;
            break;
        }
    }

    assert!(
        found_north_redirect,
        "expected at least one Conditional node with a NORTH redirect edge"
    );
}

#[test]
fn force_direction_sample_stays_routing_only() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../samples/force_direction.dots");
    let g = build_linked_graph(&path);

    let (
        create,
        end,
        print,
        set,
        select,
        duplicate,
        operator,
        conditional,
        filter,
        assertion,
        display,
        callish,
    ) = count_node_types_all(&g);

    assert_eq!(create, 4, "expected one CreateDot per mini-program");
    assert_eq!(end, 4, "expected one EndProgram per mini-program");
    assert_eq!(g.edges.len(), 4, "expected one edge per mini-program");
    assert_eq!(print, 0, "force-direction sample should not create Print nodes");
    assert_eq!(set, 0, "force-direction sample should not create Set nodes");
    assert_eq!(select, 0, "force-direction sample should not create Select nodes");
    assert_eq!(duplicate, 0, "force-direction sample should not create Duplicate nodes");
    assert_eq!(operator, 0, "force-direction sample should not create Operator nodes");
    assert_eq!(conditional, 0, "force-direction sample should not create Conditional nodes");
    assert_eq!(filter, 0, "force-direction sample should not create Filter nodes");
    assert_eq!(assertion, 0, "force-direction sample should not create Assertion nodes");
    assert_eq!(display, 0, "force-direction sample should not create Display nodes");
    assert_eq!(callish, 0, "force-direction sample should not create call-related nodes");
}
