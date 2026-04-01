use std::collections::VecDeque;
use std::sync::Arc;

use asciidots_core::{
    interpreter::AsciiField,
    parser::{AsciiCell, DotsModule, MirrorKind},
    primitives::{Direction, Orientation, Position},
    state_machine::ValueSelector,
};
use log::trace;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ir::graph::{
    CallKind, Edge, Graph, Node, NodeId, NodeType, PortType, PrintKind, SetKind,
};

#[derive(Debug, Clone)]
enum FunctionKind {
    Ascii(Position),
    Native(String),
}

pub struct Tracer {
    field: AsciiField,
    function_defs: Vec<FunctionKind>,
    function_name_to_id: FxHashMap<Arc<str>, u32>, // Maps function name to its index in function_defs
    native_names: FxHashSet<String>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    visited: FxHashSet<(Position, Direction)>,
    position_to_node: FxHashMap<Position, NodeId>,
    call_entry_nodes_by_pos: FxHashMap<Position, NodeId>,
    func_entry_nodes: FxHashMap<u32, NodeId>, // Maps function id to its CallEnter NodeId
    func_entry_dirs: FxHashSet<(u32, Direction)>,
    /// Tracks (node_id, exit_port) pairs that have already had an outgoing tracer spawned.
    /// Prevents duplicate edges when a node (e.g. Duplicate) is visited from multiple directions.
    spawned_from: FxHashSet<(NodeId, PortType)>,
}

pub struct TracingDot {
    pos: Position,
    direction: Direction,
    from_call: bool,
    edge_from: NodeId,
    edge_from_port: PortType,
    edge_ticks: u64,
}

impl TracingDot {
    pub fn new(pos: Position, direction: Direction) -> Self {
        Self {
            pos,
            direction,
            from_call: false,
            edge_from: 0,
            edge_from_port: exit_port(direction),
            edge_ticks: 0,
        }
    }
}

impl Tracer {
    pub fn new(
        module: DotsModule,
    ) -> Self {
        let (mut asciifield, functions, native_names) = module.unpack();
        let mut function_map: FxHashMap<String, FunctionKind> = functions
            .iter()
            .map(|f| {
                let (name, pos) = f.clone().unpack();
                (name, FunctionKind::Ascii(pos))
            })
            .collect();

        for name in &native_names {
            function_map.insert(name.clone(), FunctionKind::Native(name.clone()));
        }

        let mut function_name_to_id: FxHashMap<Arc<str>, u32> = FxHashMap::default();
        let mut function_defs = Vec::with_capacity(function_map.len());
        trace!("[Tracer] Function map: {function_map:#?}");
        for (index, (name, definition)) in function_map.into_iter().enumerate() {
            let call_id = index as u32;
            function_name_to_id.insert(Arc::from(name), call_id);
            function_defs.push(definition);
        }

        let position_to_node = FxHashMap::default();
        let mut call_entry_nodes_by_pos = FxHashMap::default();
        let mut func_entry_nodes = FxHashMap::default();
        let mut nodes = Vec::default();
        for (func_id,func) in function_defs.iter().enumerate() {
            if let FunctionKind::Ascii(pos) = func {
                if !call_entry_nodes_by_pos.contains_key(pos){
                    let id = nodes.len();
                    nodes.push(Node::new(*pos, NodeType::CallEnter));
                    call_entry_nodes_by_pos.insert(*pos, id);
                    func_entry_nodes.insert(func_id as u32, id);
                }else{
                    let id = *call_entry_nodes_by_pos.get(pos).unwrap();
                    func_entry_nodes.insert(func_id as u32, id);
                }
            }
        }
        trace!("[Tracer] Native names: {native_names:#?}");
        trace!("[Tracer] function_defs: {function_defs:#?}");
        trace!("[Tracer] function_name_to_id: {function_name_to_id:#?}");
        Self::resolve_call_sites(&mut asciifield, &function_name_to_id);

        Self {
            field: asciifield,
            function_defs,
            function_name_to_id,
            native_names: native_names.into_iter().collect(),
            nodes,
            edges: Vec::default(),
            visited: FxHashSet::default(),
            position_to_node,
            call_entry_nodes_by_pos,
            func_entry_nodes,
            func_entry_dirs: FxHashSet::default(),
            spawned_from: FxHashSet::default(),
        }
    }

    pub fn into_graph(self) -> Graph {
        trace!("[Tracer] Tracing complete. Nodes: {}, Edges: {}", self.nodes.len(), self.edges.len());
        trace!("Func entry map: {:#?}", self.func_entry_nodes);
        Graph {
            nodes: self.nodes,
            edges: self.edges,
            func_entry_nodes: self.func_entry_nodes,
        }
    }

    fn resolve_call_sites(field: &mut AsciiField, function_name_to_id: &FxHashMap<Arc<str>, u32>) {
        let width = field.width();
        let height = field.height();
        for y in 0..height {
            for x in 0..width {
                let position: Position = (x, y).into();
                if let Some(token) = field.at_spanned_mut(&position)
                    && let AsciiCell::Call(name) = token.inner_mut()
                    && let Some(call_id) = function_name_to_id.get(name)
                {
                    *token.inner_mut() = AsciiCell::CallById(*call_id);
                }
            }
        }
    }

    fn push_node(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    fn get_or_insert_pos(&mut self, pos: Position, node_type: NodeType) -> NodeId {
        if let Some(&id) = self.position_to_node.get(&pos) {
            return id;
        }
        let id = self.nodes.len();
        self.nodes.push(Node::new(pos, node_type));
        self.position_to_node.insert(pos, id);
        id
    }

    fn initial_dots(&self) -> Vec<TracingDot> {
        let mut initial_dots = Vec::new();
        let width = self.field.width();
        let height = self.field.height();
        for y in 0..height {
            for x in 0..width {
                let pos: Position = (x, y).into();
                if matches!(self.field.at(&pos), AsciiCell::Dot) {
                    for dir in [
                        Direction::Up,
                        Direction::Right,
                        Direction::Down,
                        Direction::Left,
                    ] {
                        let neighbor = pos + Position::from(dir);
                        if self.field.in_bounds(&neighbor)
                            && self.field.at(&neighbor).can_initialize(&dir.orientation())
                        {
                            initial_dots.push(TracingDot::new(pos, dir));
                            break;
                        }
                    }
                }
            }
        }
        initial_dots
    }

    pub fn trace(&mut self) {
        let mut queue: VecDeque<TracingDot> = VecDeque::new();

        for td in self.initial_dots() {
            let id = self.push_node(Node::new(td.pos, NodeType::CreateDot(td.direction)));
            self.position_to_node.insert(td.pos, id);
            queue.push_back(TracingDot {
                pos: td.pos,
                direction: td.direction,
                from_call: false,
                edge_from: id,
                edge_from_port: exit_port(td.direction),
                edge_ticks: 0,
            });
        }

        while let Some(td) = queue.pop_front() {
            self.step(td, &mut queue);
        }
        self.prune_duplicate_edges();
    }

    fn step(&mut self, td: TracingDot, queue: &mut VecDeque<TracingDot>) {
        // from_call dots start AT the DefRet cell — process it as a node directly,
        // no incoming edge (call jumps have no static edge)
        if td.from_call {
            let cell = self.field.at(&td.pos).clone();
            if let Some(node_id) = self.get_or_create_node(td.pos, td.direction, true, &cell) {
                let already_visited = !self.visited.insert((td.pos, td.direction));
                if !already_visited {
                    self.spawn_outgoing(td.pos, td.direction, node_id, true, &cell, queue);
                }
            }
            return;
        }

        let mut pos = td.pos;
        let mut dir = td.direction;
        let mut ticks = td.edge_ticks;

        loop {
            let next_pos = pos + Position::from(dir);

            if !self.field.in_bounds(&next_pos) {
                return; // dot dies, no edge created
            }

            match self.field.at(&next_pos) {
                AsciiCell::Empty => return,

                // --- Routing: update direction, tick, continue ---
                AsciiCell::Dot => {
                    ticks += 1;
                    pos = next_pos;
                }
                AsciiCell::Pipe(o) => {
                    if o != &dir.orientation() {
                        return;
                    }
                    ticks += 1;
                    pos = next_pos;
                }
                AsciiCell::Mirror(kind) => {
                    dir = apply_mirror(dir, *kind);
                    ticks += 1;
                    pos = next_pos;
                }
                AsciiCell::Reflector(kind) => {
                    if dir.orientation() != Orientation::Horizontal {
                        return;
                    }
                    dir = match kind {
                        MirrorKind::Forward => Direction::Right,
                        MirrorKind::Backward => Direction::Left,
                    };
                    ticks += 1;
                    pos = next_pos;
                }
                AsciiCell::ForceDirection(forced) => {
                    if dir.orthogonal(forced){
                        dir = *forced;
                    }
                    ticks += 1;
                    pos = next_pos;
                }
                AsciiCell::Warp { other, .. } => {
                    ticks += 1;
                    pos = *other;
                }
                AsciiCell::Read   // only meaningful after #/@; standalone is a passthrough
                | AsciiCell::Quote
                | AsciiCell::QuoteStream
                | AsciiCell::CrossPipe
                | AsciiCell::Comment(_)
                | AsciiCell::Negate
                | AsciiCell::Digit(_)
                | AsciiCell::Char(_) => {
                    ticks += 1;
                    pos = next_pos;
                }
                AsciiCell::OperatorBegin(_) | AsciiCell::OperatorEnd(_) => {
                    if dir.orientation() != Orientation::Horizontal {
                        return;
                    }
                    ticks += 1;
                    pos = next_pos;
                }

                // --- Node cell: finalise edge, create/find node, spawn outgoing ---
                cell => {
                    let cell = cell.clone();
                    if let Some(node_id) = self.get_or_create_node(next_pos, dir, false, &cell) {
                        let already_visited = !self.visited.insert((next_pos, dir));
                        self.edges.push(Edge {
                            from: (td.edge_from, td.edge_from_port),
                            to: (node_id, entry_port(dir)),
                            ticks,
                        });
                        if !already_visited {
                            self.spawn_outgoing(next_pos, dir, node_id, false, &cell, queue);
                        }
                        return;
                    }
                    // None: cell is present but produces no node (e.g. bare $) — treat as passthrough
                    ticks += 1;
                    pos = next_pos;
                }
            }
        }
    }

    fn get_or_create_node(
        &mut self,
        pos: Position,
        dir: Direction,
        from_call: bool,
        cell: &AsciiCell,
    ) -> Option<NodeId> {
        match cell {
            // --- Direction-slot nodes ---
            AsciiCell::Write => {
                let kind = self.collect_print_kind(pos, dir)?;
                if let Some(&id) = self.position_to_node.get(&pos) {
                    if let NodeType::Print(ref mut slots) = self.nodes[id].t {
                        slots[dir_index(dir)].get_or_insert(kind);
                    }
                    Some(id)
                } else {
                    let mut slots: [Option<PrintKind>; 4] = [None, None, None, None];
                    slots[dir_index(dir)] = Some(kind);
                    let id = self.nodes.len();
                    self.nodes.push(Node::new(pos, NodeType::Print(slots)));
                    self.position_to_node.insert(pos, id);
                    Some(id)
                }
            }

            AsciiCell::ValueMarker | AsciiCell::AddrMarker => {
                let kind = self.collect_set_kind(pos, dir, cell);
                if let Some(kind) = kind {
                    // has digits/input after — Set node
                    if let Some(&id) = self.position_to_node.get(&pos) {
                        if let NodeType::Set(ref mut slots) = self.nodes[id].t {
                            slots[dir_index(dir)].get_or_insert(kind);
                        }
                        Some(id)
                    } else {
                        let mut slots: [Option<SetKind>; 4] = [None, None, None, None];
                        slots[dir_index(dir)] = Some(kind);
                        let id = self.nodes.len();
                        self.nodes.push(Node::new(pos, NodeType::Set(slots)));
                        self.position_to_node.insert(pos, id);
                        Some(id)
                    }
                } else {
                    // no digits — Select node only if the next cell can use a selector
                    let next = pos + Position::from(dir);
                    if self.field.in_bounds(&next) && accepts_selector(self.field.at(&next), dir) {
                        let selector = match cell {
                            AsciiCell::ValueMarker => ValueSelector::Value,
                            _ => ValueSelector::Addr,
                        };
                        Some(self.get_or_insert_pos(pos, NodeType::Select(selector)))
                    } else {
                        None
                    }
                }
            }

            // --- Position-only nodes ---
            AsciiCell::Operator { repr, orientation } => {
                Some(self.get_or_insert_pos(pos, NodeType::Operator(*repr, *orientation)))
            }
            AsciiCell::Duplicate => Some(self.get_or_insert_pos(pos, NodeType::Duplicate)),
            AsciiCell::StopProgram => Some(self.get_or_insert_pos(pos, NodeType::EndProgram)),
            AsciiCell::Conditional(_) => {
                let negated = self.is_negated(pos);
                Some(self.get_or_insert_pos(pos, NodeType::Conditional(negated)))
            }
            AsciiCell::Filter(b) => Some(self.get_or_insert_pos(pos, NodeType::Filter(*b))),
            AsciiCell::Assertion => {
                let expected = self.collect_assertion_value(pos, dir);
                Some(self.get_or_insert_pos(pos, NodeType::Assertion(expected)))
            }
            AsciiCell::Display(id) => Some(self.get_or_insert_pos(pos, NodeType::Display(*id))),

            AsciiCell::Call(name) => {
                // Unresolved call — resolve_calls() couldn't match the name to a known function.
                unreachable!("Unresolved call to {:?} at {:?} — should have been resolved to CallById or caused a compile error", name, pos);
            }
            AsciiCell::CallById(func_id) => {
                trace!("Resolving call to func_id {} at {:?} with from_call={} in dir {:?} ", func_id, pos, from_call, dir);
                let call_kind = match self.function_defs.get(*func_id as usize) {
                    Some(FunctionKind::Native(name)) => CallKind::NativeCall(name.clone()),
                    Some(FunctionKind::Ascii(_)) | None => CallKind::AsciiCall(*func_id),
                };
                trace!("Resolved call kind: {:?}, func_entry_dirs: {:#?}", call_kind, self.func_entry_dirs);
                Some(self.get_or_insert_pos(pos, NodeType::Call(call_kind)))
            }

            AsciiCell::DefRet(name) => {
                if from_call {
                    trace!("Resolving call to {:?} at {:?} with from_call=true", name, pos);
                    let func_id = self
                        .function_name_to_id
                        .get(name.as_str())
                        .copied()
                        .expect("DefRet name not found in function_name_to_id — should have been resolved by resolve_calls");
                    if let Some(&node_id) = self.func_entry_nodes.get(&func_id) {
                        Some(node_id)
                    } else {
                        let node_id = self.nodes.len();
                        self.nodes.push(Node::new(pos, NodeType::CallEnter));
                        self.call_entry_nodes_by_pos.insert(pos, node_id);
                        self.func_entry_nodes.insert(func_id, node_id);
                        Some(node_id)
                    }
                } else {
                    Some(self.get_or_insert_pos(pos, NodeType::Return))
                }
            }

            _ => panic!("Cell type {:?} should have been handled by step() passthrough logic, not get_or_create_node", cell),
        }
    }

    fn spawn_outgoing(
        &mut self,
        pos: Position,
        dir: Direction,
        node_id: NodeId,
        from_call: bool,
        cell: &AsciiCell,
        queue: &mut VecDeque<TracingDot>,
    ) {
        match cell {
            // --- Single pass-through ---
            AsciiCell::Write => {
                // If the cell ahead is a #/@ modifier it was already consumed to
                // determine the PrintKind; skip over it so the outgoing dot doesn't
                // land on it and create a spurious Select node.
                let modifier_pos = pos + Position::from(dir);
                if self.field.in_bounds(&modifier_pos) {
                    match self.field.at(&modifier_pos) {
                        AsciiCell::ValueMarker | AsciiCell::AddrMarker => {
                            let port = exit_port(dir);
                            if self.spawned_from.insert((node_id, port)) {
                                queue.push_back(TracingDot {
                                    pos: modifier_pos,
                                    direction: dir,
                                    from_call: false,
                                    edge_from: node_id,
                                    edge_from_port: port,
                                    edge_ticks: 1,
                                });
                            }
                        }
                        AsciiCell::Quote | AsciiCell::QuoteStream => {
                            let string_len = self.collect_quoted(modifier_pos, dir).chars().count();
                            let mut end_quote = modifier_pos;
                            for _ in 0..=string_len {
                                end_quote = end_quote + Position::from(dir);
                            }
                            let port = exit_port(dir);
                            if self.spawned_from.insert((node_id, port)) {
                                queue.push_back(TracingDot {
                                    pos: end_quote,
                                    direction: dir,
                                    from_call: false,
                                    edge_from: node_id,
                                    edge_from_port: port,
                                    edge_ticks: 0,
                                });
                            }
                        }
                        _ => {
                            self.enqueue_tracer(pos, dir, node_id, queue);
                        }
                    }
                } else {
                    self.enqueue_tracer(pos, dir, node_id, queue);
                }
            }
            AsciiCell::ValueMarker
            | AsciiCell::AddrMarker
            | AsciiCell::Filter(_)
            | AsciiCell::Assertion => {
                self.enqueue_tracer(pos, dir, node_id, queue);
            }

            AsciiCell::DefRet(_) => {
                if from_call {
                    // CallEnter: pass through in all four directions (like Duplicate)
                    // to ensure dots arriving from any direction can continue
                    for out_dir in [
                        Direction::Up,
                        Direction::Down,
                        Direction::Left,
                        Direction::Right,
                    ] {
                        self.enqueue_tracer(pos, out_dir, node_id, queue);
                    }
                }
                // Return: terminal, VM handles via callstack
            }

            // --- Duplicate: all three non-entry directions ---
            AsciiCell::Duplicate => {
                for out_dir in [
                    Direction::Up,
                    Direction::Down,
                    Direction::Left,
                    Direction::Right,
                ] {
                    if out_dir != dir.reverse() {
                        self.enqueue_tracer(pos, out_dir, node_id, queue);
                    }
                }
            }

            // --- Conditional: horizontal payload exits NORTH + straight; vertical disappears ---
            AsciiCell::Conditional(_) => {
                if dir.orientation() == Orientation::Horizontal {
                    self.enqueue_tracer(pos, Direction::Up, node_id, queue);
                    self.enqueue_tracer(pos, dir, node_id, queue);
                }
            }

            // --- Operator: primary axis continues, secondary disappears ---
            AsciiCell::Operator { orientation, .. } => {
                if dir.orientation() == *orientation {
                    self.enqueue_tracer(pos, dir, node_id, queue);
                }
            }

            // --- CallById: native emits in all four directions; ascii continues + jumps to body ---
            AsciiCell::CallById(id) => {
                // Resolve the function kind before mutably borrowing self for enqueue_tracer.
                enum ResolvedCall {
                    Native,
                    Ascii(Position, u32),
                }
                let resolved = match self.function_defs.get(*id as usize) {
                    Some(FunctionKind::Native(_)) => ResolvedCall::Native,
                    Some(FunctionKind::Ascii(p)) => ResolvedCall::Ascii(*p, *id),
                    None => return,
                };
                match resolved {
                    ResolvedCall::Native => {
                        for out_dir in [
                            Direction::Up,
                            Direction::Down,
                            Direction::Left,
                            Direction::Right,
                        ] {
                            self.enqueue_tracer(pos, out_dir, node_id, queue);
                        }
                    }
                    ResolvedCall::Ascii(defret_pos, id) => {
                        //self.enqueue_tracer(pos, dir, node_id, queue);
                        for out_dir in [
                            Direction::Up,
                            Direction::Down,
                            Direction::Left,
                            Direction::Right,
                        ] {
                            self.enqueue_tracer(pos, out_dir, node_id, queue);
                        }
                        if self.func_entry_dirs.insert((id, dir)) {
                            queue.push_back(TracingDot {
                                pos: defret_pos,
                                direction: dir,
                                from_call: true,
                                edge_from: node_id,
                                edge_from_port: exit_port(dir),
                                edge_ticks: 0,
                            });
                        }
                    }
                }
            }

            AsciiCell::StopProgram => {}

            // Display: Down→SetPixel reverses to Up and continues; all other directions are sinks.
            AsciiCell::Display(_) => {
                if dir == Direction::Down {
                    self.enqueue_tracer(pos, Direction::Up, node_id, queue);
                }
            }

            _ => {}
        }
    }

    fn prune_duplicate_edges(&mut self) {
        let mut unique_edges = FxHashSet::default();
        self.edges.retain(|edge| unique_edges.insert((edge.from, edge.to)));
    }

    fn enqueue_tracer(
        &mut self,
        pos: Position,
        dir: Direction,
        from_node: NodeId,
        queue: &mut VecDeque<TracingDot>,
    ) {
        let port = exit_port(dir);
        if !self.spawned_from.insert((from_node, port)) {
            return; // already spawned a tracer from this node on this port
        }
        queue.push_back(TracingDot {
            pos,
            direction: dir,
            from_call: false,
            edge_from: from_node,
            edge_from_port: port,
            edge_ticks: 0,
        });
    }

    fn is_negated(&self, pos: Position) -> bool {
        let below = pos + Position::from(Direction::Down);
        self.field.in_bounds(&below) && matches!(self.field.at(&below), AsciiCell::Negate)
    }

    fn collect_print_kind(&self, pos: Position, dir: Direction) -> Option<PrintKind> {
        let modifier_pos = pos + Position::from(dir);
        if !self.field.in_bounds(&modifier_pos) {
            return None;
        }
        match self.field.at(&modifier_pos) {
            AsciiCell::ValueMarker => Some(PrintKind::Value),
            AsciiCell::AddrMarker => Some(PrintKind::Addr),
            AsciiCell::Quote => Some(PrintKind::Normal(self.collect_quoted(modifier_pos, dir))),
            AsciiCell::QuoteStream => {
                Some(PrintKind::Stream(self.collect_quoted(modifier_pos, dir)))
            }
            _ => None,
        }
    }

    fn collect_set_kind(&self, pos: Position, dir: Direction, cell: &AsciiCell) -> Option<SetKind> {
        let is_value = matches!(cell, AsciiCell::ValueMarker);
        let next = pos + Position::from(dir);
        if !self.field.in_bounds(&next) {
            return None;
        }
        match self.field.at(&next) {
            AsciiCell::Read => Some(SetKind::Input),
            AsciiCell::Digit(d) => {
                let mut acc: u64 = *d as u64;
                let mut cur = next + Position::from(dir);
                while self.field.in_bounds(&cur) {
                    match self.field.at(&cur) {
                        AsciiCell::Digit(d2) => {
                            acc = acc * 10 + *d2 as u64;
                            cur = cur + Position::from(dir);
                        }
                        _ => break,
                    }
                }
                let v = acc as f64;
                Some(if is_value {
                    SetKind::Value(v)
                } else {
                    SetKind::Addr(v)
                })
            }
            AsciiCell::Quote | AsciiCell::QuoteStream => {
                let s = self.collect_quoted(next, dir);
                let v: f64 = s.parse().unwrap_or(0.0);
                Some(if is_value {
                    SetKind::Value(v)
                } else {
                    SetKind::Addr(v)
                })
            }
            _ => None,
        }
    }

    fn collect_assertion_value(&self, pos: Position, dir: Direction) -> Option<f64> {
        let next = pos + Position::from(dir);
        if !self.field.in_bounds(&next) {
            return None;
        }
        match self.field.at(&next) {
            AsciiCell::Digit(d) => {
                let mut acc = *d as u64;
                let mut cur = next + Position::from(dir);
                while self.field.in_bounds(&cur) {
                    match self.field.at(&cur) {
                        AsciiCell::Digit(d2) => {
                            acc = acc * 10 + *d2 as u64;
                            cur = cur + Position::from(dir);
                        }
                        _ => break,
                    }
                }
                Some(acc as f64)
            }
            AsciiCell::Quote | AsciiCell::QuoteStream => {
                self.collect_quoted(next, dir).parse::<f64>().ok()
            }
            _ => None,
        }
    }

    /// Collects the string content between an opening quote at `quote_pos` and its matching close.
    /// The closing delimiter matches the same variant as the opening cell.
    fn collect_quoted(&self, quote_pos: Position, dir: Direction) -> String {
        let is_stream = matches!(self.field.at(&quote_pos), AsciiCell::QuoteStream);
        let mut acc = String::new();
        let mut cur = quote_pos + Position::from(dir);
        while self.field.in_bounds(&cur) {
            let cell = self.field.at(&cur);
            let is_close = if is_stream {
                matches!(cell, AsciiCell::QuoteStream)
            } else {
                matches!(cell, AsciiCell::Quote)
            };
            if is_close {
                break;
            }
            acc.push(char::from(cell));
            cur = cur + Position::from(dir);
        }
        acc
    }
}

fn dir_index(dir: Direction) -> usize {
    match dir {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

fn entry_port(dir: Direction) -> PortType {
    match dir {
        Direction::Up => PortType::SOUTH,
        Direction::Down => PortType::NORTH,
        Direction::Left => PortType::EAST,
        Direction::Right => PortType::WEST,
    }
}

fn exit_port(dir: Direction) -> PortType {
    match dir {
        Direction::Up => PortType::NORTH,
        Direction::Down => PortType::SOUTH,
        Direction::Left => PortType::WEST,
        Direction::Right => PortType::EAST,
    }
}

fn apply_mirror(dir: Direction, kind: MirrorKind) -> Direction {
    match (dir, kind) {
        (Direction::Up, MirrorKind::Forward) => Direction::Right,
        (Direction::Down, MirrorKind::Forward) => Direction::Left,
        (Direction::Right, MirrorKind::Forward) => Direction::Up,
        (Direction::Left, MirrorKind::Forward) => Direction::Down,
        (Direction::Up, MirrorKind::Backward) => Direction::Left,
        (Direction::Down, MirrorKind::Backward) => Direction::Right,
        (Direction::Right, MirrorKind::Backward) => Direction::Down,
        (Direction::Left, MirrorKind::Backward) => Direction::Up,
    }
}

/// Returns true if `cell` can receive a value/addr selector arriving from `dir`.
///
/// - `OperatorBegin`: entered from the left (dot traveling Right)
/// - `OperatorEnd`:   entered from the right (dot traveling Left)
/// - `Operator`:      vertical operands (dot traveling Up or Down)
/// - `Conditional`:   condition dot enters from below (dot traveling Up)
fn accepts_selector(cell: &AsciiCell, dir: Direction) -> bool {
    match cell {
        AsciiCell::Negate => dir == Direction::Up,
        AsciiCell::Assertion => true,
        AsciiCell::OperatorBegin(_) => dir == Direction::Right,
        AsciiCell::OperatorEnd(_) => dir == Direction::Left,
        AsciiCell::Operator { .. } => matches!(dir, Direction::Up | Direction::Down),
        AsciiCell::Conditional(_) => dir == Direction::Up,
        _ => false,
    }
}
