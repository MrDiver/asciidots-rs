use asciidots_core::parser::{Lexer, Parser};
use asciidots_graph::{
    compiler::tracer::Tracer,
    ir::graph::{Graph, NodeType, PrintKind, SetKind},
};
use rustc_hash::FxHashSet;

fn build_graph(program: &str) -> Graph {
    let source = Lexer::new(program).run().expect("lex failed");
    let module = Parser::new(source).run().expect("parse failed");
    let mut tracer = Tracer::new(module, std::iter::empty(), FxHashSet::default());
    tracer.trace();
    tracer.into_graph()
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
