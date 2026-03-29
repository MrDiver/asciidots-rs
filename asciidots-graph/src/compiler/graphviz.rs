use crate::ir::graph::{CallKind, Graph, NodeType, PortType, PrintKind, SetKind};

pub fn to_graphviz(graph: &Graph) -> String {
    let mut out = String::from("digraph asciidots {\n    rankdir=LR;\n    node [shape=record fontname=\"monospace\"];\n\n");

    for (id, node) in graph.nodes.iter().enumerate() {
        let label = node_label(&node.t);
        let pos = &node.position;
        out.push_str(&format!(
            "    n{id} [label=\"{{{{{label}}}|({x},{y})}}\" tooltip=\"({x},{y})\"];\n",
            id = id,
            label = escape_label(&label),
            x = pos.x,
            y = pos.y,
        ));
    }

    out.push('\n');

    for edge in &graph.edges {
        let (from_id, from_port) = &edge.from;
        let (to_id, to_port) = &edge.to;
        let label = if edge.ticks > 0 {
            format!("{}", edge.ticks)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "    n{from_id}:{fp} -> n{to_id}:{tp} [label=\"{label}\"];\n",
            from_id = from_id,
            fp = port_name(from_port),
            to_id = to_id,
            tp = port_name(to_port),
            label = label,
        ));
    }

    out.push_str("}\n");
    out
}

fn port_name(port: &PortType) -> &'static str {
    match port {
        PortType::NORTH => "n",
        PortType::SOUTH => "s",
        PortType::EAST  => "e",
        PortType::WEST  => "w",
    }
}

fn node_label(t: &NodeType) -> String {
    match t {
        NodeType::CreateDot(dir) => format!("CreateDot({dir:?})"),
        NodeType::Print(slots) => {
            let parts: Vec<String> = slots
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    s.as_ref().map(|k| format!("{}: {}", dir_label(i), print_kind_label(k)))
                })
                .collect();
            format!("Print | {}", parts.join(" | "))
        }
        NodeType::Set(slots) => {
            let parts: Vec<String> = slots
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    s.as_ref().map(|k| format!("{}: {}", dir_label(i), set_kind_label(k)))
                })
                .collect();
            format!("Set | {}", parts.join(" | "))
        }
        NodeType::Duplicate => "Duplicate".to_string(),
        NodeType::EndProgram => "EndProgram".to_string(),
        NodeType::Call(kind) => match kind {
            CallKind::NativeCall(name) => format!("NativeCall({name})"),
            CallKind::AsciiCall(id) => format!("Call(fn#{id})"),
        },
        NodeType::CallEnter => "CallEnter".to_string(),
        NodeType::Return => "Return".to_string(),
        NodeType::Operator(ch, orient) => format!("Op({ch}|{orient:?})"),
        NodeType::Conditional(negated) => {
            if *negated { "Conditional(!)".to_string() } else { "Conditional".to_string() }
        }
        NodeType::Filter(b) => format!("Filter({b})"),
        NodeType::Assertion(Some(v)) => format!("Assertion(== {v})"),
        NodeType::Assertion(None) => "Assertion".to_string(),
        NodeType::Select(sel) => format!("Select({sel:?})"),
        NodeType::Display(id) => format!("Display({id})"),
    }
}

fn dir_label(index: usize) -> &'static str {
    match index {
        0 => "U",
        1 => "D",
        2 => "L",
        _ => "R",
    }
}

fn print_kind_label(k: &PrintKind) -> String {
    match k {
        PrintKind::Value => "Val".to_string(),
        PrintKind::Addr  => "Addr".to_string(),
        PrintKind::Normal(s) => format!("\\\"{}\\\"", escape_label(s)),
        PrintKind::Stream(s) => format!("stream:\\\"{}\\\"", escape_label(s)),
    }
}

fn set_kind_label(k: &SetKind) -> String {
    match k {
        SetKind::Input     => "?".to_string(),
        SetKind::Value(v)  => format!("{v}"),
        SetKind::Addr(v)   => format!("@{v}"),
    }
}

fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('|', "\\|")
}
