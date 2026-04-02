// TODO: TUI visualization for the graph runtime — display a grid where each cell is a Node
// (using node.position), and dots are rendered as interpolated positions between their
// from-node and to-node based on ticks_left/ticks. Reuse the existing asciidots-tui
// infrastructure but feed it graph IR instead of the raw AsciiField.

use std::collections::VecDeque;

use log::trace;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use asciidots_core::{
    engine::{
        dot_trait::{EngineDot, IsTrue},
        operator::{ConditionalOperator, Operator},
    },
    interpreter::{ColorMode, DisplayCommand, InterpreterCommand, InterpreterResult},
    native::NativeFunction,
    primitives::{Directed, Direction},
};

use crate::ir::{
    dot::GraphDot,
    graph::{CallKind, Graph, NodeId, NodeType, PortType, PrintKind, SetKind},
};

const DEBUG_BUFFER_LIMIT: usize = 128;

#[derive(Debug, Clone)]
pub struct NodeVisitRecord {
    pub step: u64,
    pub node_id: NodeId,
    pub direction: Direction,
    pub selected: f64,
    pub addr: f64,
}

#[derive(Debug, Clone)]
pub struct DeadEndRecord {
    pub step: u64,
    pub from_node: NodeId,
    pub direction: Direction,
    pub selected: f64,
    pub addr: f64,
}

#[derive(Debug, Clone)]
struct PendingDot {
    dot: GraphDot,
    from_node: NodeId,
    dest_node: NodeId,
    total_ticks: u64,
    ticks_left: u64,
}

#[derive(Debug, Clone)]
pub struct PendingDotSnapshot {
    pub from_node: NodeId,
    pub dest_node: NodeId,
    pub total_ticks: u64,
    pub ticks_left: u64,
    pub direction: Direction,
    pub selected: f64,
    pub addr: f64,
}

struct DeferredPrint {
    result: InterpreterResult,
    dot: GraphDot,
    from_node: NodeId,
    ticks_left: u64,
}

pub struct GraphExecutor {
    graph: Graph,
    /// Outgoing edge indices per node, indexed by NodeId.
    outgoing: Vec<Vec<usize>>,
    pending: Vec<PendingDot>,
    deferred_prints: Vec<DeferredPrint>,
    arrived: Vec<PendingDot>,  // Reused buffer for dots that arrived this step
    operator_queues: FxHashMap<NodeId, Operator<GraphDot, f64>>,
    conditional_queues: FxHashMap<NodeId, ConditionalOperator<GraphDot>>,
    native_registry: FxHashMap<String, Box<dyn NativeFunction<GraphDot>>>,
    native_returned: Vec<GraphDot>,
    /// Dots suspended waiting for user input, keyed by request id.
    waiting_for_input: FxHashMap<u64, (GraphDot, NodeId)>,
    input_counter: u64,
    results: Vec<InterpreterResult>,
    finished: bool,
    /// Consecutive steps where no dots were active; finish after threshold.
    empty_steps: usize,
    step_counter: u64,
    recent_visits: VecDeque<NodeVisitRecord>,
    recent_dead_ends: VecDeque<DeadEndRecord>,
    /// Track peak memory for tuning allocations
    max_pending_size: usize,
}

impl GraphExecutor {
    pub fn new(
        graph: Graph,
        native_registry: FxHashMap<String, Box<dyn NativeFunction<GraphDot>>>,
        capacity_hint: Option<usize>,
    ) -> Self {
        let n = graph.nodes.len();

        let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, edge) in graph.edges.iter().enumerate() {
            outgoing[edge.from.0].push(i);
        }
        
        let mut operator_queues: FxHashMap<NodeId, Operator<GraphDot, f64>> = FxHashMap::default();
        let mut conditional_queues: FxHashMap<NodeId, ConditionalOperator<GraphDot>> =
            FxHashMap::default();
        for (id, node) in graph.nodes.iter().enumerate() {
            match &node.t {
                NodeType::Operator(c, orientation) => {
                    operator_queues.insert(
                        id,
                        Operator {
                            c: *c,
                            queue: Default::default(),
                            kind: *orientation,
                            _marker: std::marker::PhantomData,
                        },
                    );
                }
                NodeType::Conditional(negated) => {
                    conditional_queues.insert(
                        id,
                        ConditionalOperator {
                            queue: Default::default(),
                            negated: *negated,
                        },
                    );
                }
                _ => {}
            }
        }

        let mut executor = Self {
            outgoing,
            pending: Vec::with_capacity(capacity_hint.unwrap_or(2048)),
            deferred_prints: Vec::with_capacity(64),
            arrived: Vec::with_capacity(256),
            operator_queues,
            conditional_queues,
            native_registry,
            native_returned: Vec::with_capacity(256),
            waiting_for_input: FxHashMap::default(),
            input_counter: 0,
            results: Vec::with_capacity(64),
            finished: false,
            empty_steps: 0,
            step_counter: 0,
            recent_visits: VecDeque::with_capacity(DEBUG_BUFFER_LIMIT),
            recent_dead_ends: VecDeque::with_capacity(DEBUG_BUFFER_LIMIT),
            graph,
            max_pending_size: 0,
        };

        // Seed initial dots from CreateDot nodes.
        for node_id in 0..executor.graph.nodes.len() {
            if let NodeType::CreateDot(dir) = executor.graph.nodes[node_id].t {
                let dot = GraphDot::new(node_id, dir);
                executor.forward_dot(dot, node_id);
            }
        }

        executor
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn max_pending_size(&self) -> usize {
        self.max_pending_size
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn pending_snapshots(&self) -> Vec<PendingDotSnapshot> {
        self.pending
            .iter()
            .map(|pd| PendingDotSnapshot {
                from_node: pd.from_node,
                dest_node: pd.dest_node,
                total_ticks: pd.total_ticks,
                ticks_left: pd.ticks_left,
                direction: pd.dot.direction(),
                selected: pd.dot.selected(),
                addr: pd.dot.addr(),
            })
            .collect()
    }

    pub fn step(&mut self) -> &[InterpreterResult] {
        trace!("--- Step {} ---", self.step_counter);
        trace!("Pending dots: {:#?}", self.pending);
        trace!("Waiting for input: {}", self.waiting_for_input.len());
        trace!("Deferred prints: {}", self.deferred_prints.len());
        trace!("Native returned: {}", self.native_returned.len());
        trace!("Results: {:?}", self.results);
        self.step_counter += 1;
        self.results.clear();

        // Process deferred prints
        let mut i = 0;
        while i < self.deferred_prints.len() {
            if self.deferred_prints[i].ticks_left == 0 {
                let deferred = self.deferred_prints.swap_remove(i);
                self.results.push(deferred.result);
                self.forward_dot(deferred.dot, deferred.from_node);
            } else {
                self.deferred_prints[i].ticks_left -= 1;
                i += 1;
            }
        }

        // Collect returns from native functions.
        self.collect_native_returns();

        // Fire dots that have arrived (ticks_left == 0), decrement the rest.
        let mut i = 0;
        while i < self.pending.len() {
            if self.pending[i].ticks_left == 0 {
                self.arrived.push(self.pending.swap_remove(i));
            } else {
                self.pending[i].ticks_left -= 1;
                i += 1;
            }
        }

        // Track peak memory for tuning pre-allocation
        self.max_pending_size = self.max_pending_size.max(self.pending.len());

        if self.arrived.is_empty()
            && self.native_returned.is_empty()
            && self.waiting_for_input.is_empty()
        {
            self.empty_steps += 1;
            if self.empty_steps >= 1000 {
                self.finished = true;
            }
        } else {
            self.empty_steps = 0;
        }

        while let Some(pd) = self.arrived.pop() {
            self.process_dot(pd.dot, pd.dest_node);
        }

        &self.results
    }

    pub fn debug_recent_visits(&self) -> Vec<NodeVisitRecord> {
        self.recent_visits.iter().cloned().collect()
    }

    pub fn debug_recent_dead_ends(&self) -> Vec<DeadEndRecord> {
        self.recent_dead_ends.iter().cloned().collect()
    }

    pub fn debug_node_summary(&self, node_id: NodeId) -> String {
        let node = &self.graph.nodes[node_id];
        format!(
            "node#{node_id} @ ({}, {}) {}",
            node.position.x,
            node.position.y,
            node_type_name(&node.t)
        )
    }

    pub fn debug_node_ports(&self, node_id: NodeId) -> (Vec<PortType>, Vec<PortType>) {
        let incoming = self
            .graph
            .edges
            .iter()
            .filter(|edge| edge.to.0 == node_id)
            .map(|edge| edge.to.1)
            .collect();
        let outgoing = self
            .graph
            .edges
            .iter()
            .filter(|edge| edge.from.0 == node_id)
            .map(|edge| edge.from.1)
            .collect();
        (incoming, outgoing)
    }

    pub fn handle_command(&mut self, command: InterpreterCommand) {
        match command {
            InterpreterCommand::InputAnswer(id, value) => {
                if let Some((mut dot, node_id)) = self.waiting_for_input.remove(&id) {
                    dot.set_selected(value);
                    self.forward_dot(dot, node_id);
                }
            }
        }
    }

    fn collect_native_returns(&mut self) {
        self.native_returned.clear();
        for func in self.native_registry.values_mut() {
            func.tick_into(&mut self.native_returned);
        }
        let returned = std::mem::take(&mut self.native_returned);
        for mut dot in returned {
            // Pop callstack to find which Call node this dot came from, then
            // follow that node's outgoing continuation edges.
            if let Some(call_node_id) = dot.pop_callstack() {
                let dir = dot.direction();
                self.send_in_direction(dot, call_node_id, dir);
            }
        }
    }

    #[inline]
    fn process_dot(&mut self, mut dot: GraphDot, node_id: NodeId) {
        self.push_visit(NodeVisitRecord {
            step: self.step_counter,
            node_id,
            direction: dot.direction(),
            selected: dot.selected(),
            addr: dot.addr(),
        });
        match &self.graph.nodes[node_id].t {
            NodeType::CreateDot(_) => {
                // Should not be reached at runtime — dots are seeded at construction.
                self.forward_dot(dot, node_id);
            }

            NodeType::EndProgram => {
                self.finished = true;
                self.results.push(InterpreterResult::EndProgram);
            }

            NodeType::Print(slots) => {
                if let Some(kind) = &slots[dir_index(dot.direction())] {
                    match kind {
                        PrintKind::Value => {
                            self.results
                                .push(InterpreterResult::PrintLn(dot.selected().to_string()));
                        }
                        PrintKind::Addr => {
                            self.results
                                .push(InterpreterResult::PrintLn(dot.addr().to_string()));
                        }
                        PrintKind::Normal(s) => {
                            self.deferred_prints.push(DeferredPrint {
                                result: InterpreterResult::PrintLn(s.clone()),
                                dot,
                                from_node: node_id,
                                ticks_left: s.chars().count() as u64,
                            });
                            return;
                        }
                        PrintKind::Stream(s) => {
                            for c in s.chars() {
                                self.results.push(InterpreterResult::PrintChar(c));
                            }
                        }
                    }
                }
                self.forward_dot(dot, node_id);
            }

            NodeType::Set(slots) => {
                if let Some(kind) = &slots[dir_index(dot.direction())] {
                    match kind {
                        SetKind::Value(v) => {
                            let v = *v;
                            dot.set_selected(v);
                        }
                        SetKind::Addr(v) => {
                            let v = *v;
                            dot.set_addr(v);
                        }
                        SetKind::Input => {
                            let id = self.input_counter;
                            self.input_counter += 1;
                            self.results.push(InterpreterResult::InputRequest(id));
                            self.waiting_for_input.insert(id, (dot, node_id));
                            return; // dot is suspended
                        }
                    }
                }
                self.forward_dot(dot, node_id);
            }

            NodeType::Select(sel) => {
                let sel = *sel;
                dot.apply_select(sel);
                self.forward_dot(dot, node_id);
            }

            NodeType::Duplicate => {
                // Send a clone of the dot on every outgoing edge except the one
                // the dot came from (entry port = the port opposite its direction).
                let _entry = entry_port(dot.direction());
                for &edge_idx in &self.outgoing[node_id] {
                    let edge = &self.graph.edges[edge_idx];
                    let leaving_dir = dir_from_exit_port(&edge.from.1);
                    if leaving_dir == dot.direction().reverse() {
                        continue; // skip back-the-way-it-came
                    }
                    let dest = edge.to.0;
                    let ticks = edge.ticks;
                    let arrival_dir = dir_from_entry_port(&edge.to.1);
                    let mut clone = dot.clone();
                    clone.set_direction(arrival_dir);
                    self.pending.push(PendingDot {
                        dot: clone,
                        from_node: node_id,
                        dest_node: dest,
                        total_ticks: ticks,
                        ticks_left: ticks,
                    });
                }
            }

            NodeType::Operator(_, _) => {
                if let Some(queue) = self.operator_queues.get_mut(&node_id) {
                    if let Some(result) = queue.enter(dot) {
                        self.forward_dot(result, node_id);
                    }
                }else{
                    unreachable!("Operator queue not found for node {node_id}");
                }
            }

            NodeType::Conditional(_) => {
                if let Some(queue) = self.conditional_queues.get_mut(&node_id) {
                    if let Some(result) = queue.enter(dot) {
                        let dir = result.direction();
                        self.send_in_direction(result, node_id, dir);
                    }
                }else{
                    unreachable!("Conditional queue not found for node {node_id}");
                }
            }

            NodeType::Filter(pass_if_true) => {
                let pass = if *pass_if_true {
                    dot.is_true()
                } else {
                    !dot.is_true()
                };
                if pass {
                    self.forward_dot(dot, node_id);
                }
                // else: dot is consumed
            }

            NodeType::Assertion(expected) => {
                if let Some(v) = expected {
                    let v = *v;
                    assert!(
                        (dot.selected() - v).abs() < f64::EPSILON,
                        "Assertion failed: expected {v}, got {}",
                        dot.selected()
                    );
                }
                self.forward_dot(dot, node_id);
            }

            NodeType::Display(display_id) => {
                let display_id = *display_id;
                let dir = dot.direction();
                let command = match dir {
                    Direction::Right => DisplayCommand::SetWidth(dot.selected().round() as usize),
                    Direction::Up => DisplayCommand::SetHeight(dot.selected().round() as usize),
                    Direction::Left => {
                        DisplayCommand::SetColorMode(ColorMode::from_f64(dot.selected()))
                    }
                    Direction::Down => DisplayCommand::SetPixel {
                        index: dot.addr() as usize,
                        value: dot.selected(),
                    },
                };
                self.results.push(InterpreterResult::Display {
                    display_id,
                    command,
                });
                // Down→SetPixel: reverse to Up and continue. All other directions: dot is consumed.
                if dir == Direction::Down {
                    dot.set_direction(Direction::Up);
                    self.forward_dot(dot, node_id);
                }
            }

            NodeType::CallEnter => {
                self.forward_dot(dot, node_id);
            }

            NodeType::Return => {
                // Pop callstack to get the Call node, then follow its continuation edges.
                if let Some(call_node_id) = dot.pop_callstack() {
                    for &edge_idx in &self.outgoing[call_node_id] {
                        let edge = &self.graph.edges[edge_idx];
                        // Check if dot can exit via this edge's from-port (opposite its direction).
                        let leaving_dir = dir_from_exit_port(&edge.from.1);
                        if leaving_dir != dot.direction() {
                            continue; // can't exit this way
                        }
                        let dest = edge.to.0;
                        let ticks = edge.ticks;
                        let arrival_dir = dir_from_entry_port(&edge.to.1);
                        let mut clone = dot.clone();
                        clone.set_direction(arrival_dir);
                        self.pending.push(PendingDot {
                            dot: clone,
                            from_node: call_node_id,
                            dest_node: dest,
                            total_ticks: ticks,
                            ticks_left: ticks,
                        });
                        break;
                    }
                }else{
                    unreachable!("Return dot with empty callstack at node {node_id}");
                }
            }

            NodeType::Call(kind) => match kind {
                CallKind::NativeCall(name) => {
                    let name = name.clone();
                    dot.push_callstack(node_id);
                    if let Some(func) = self.native_registry.get_mut(&name) {
                        func.dot_enter(dot);
                    }
                }
                CallKind::AsciiCall(func_id) => {
                    trace!("Call node {node_id} pushing to callstack and jumping to function {func_id} with direction {:?}", dot.direction());
                    let func_id = *func_id;
                    dot.push_callstack(node_id);
                    if let Some(&entry_node) = self.graph.func_entry_nodes.get(&func_id) {
                        trace!("Found entry node {entry_node} for function {func_id}, pushing pending dot with direction {:?}",
                            dot.direction());
                        trace!("Entry node details: {:#?}", self.graph.nodes[entry_node]);
                        self.pending.push(PendingDot {
                            dot,
                            from_node: node_id,
                            dest_node: entry_node,
                            total_ticks: 0,
                            ticks_left: 0,
                        });
                    }else{
                        unreachable!("Function entry node not found for function {func_id}");
                    }
                }
            },
        }
    }

    /// Send `dot` along the outgoing edge that exits `from_node` in `dot.direction()`.
    #[inline]
    fn forward_dot(&mut self, dot: GraphDot, from_node: NodeId) {
        let dir = dot.direction();
        self.send_in_direction(dot, from_node, dir);
    }

    /// Send `dot` along the outgoing edge that exits `from_node` via `exit_port(dir)`.
    #[inline]
    fn send_in_direction(&mut self, dot: GraphDot, from_node: NodeId, dir: Direction) {
        let port = exit_port(dir);
        for &edge_idx in &self.outgoing[from_node] {
            let edge = &self.graph.edges[edge_idx];
            if edge.from.1 == port {
                let dest = edge.to.0;
                let ticks = edge.ticks;
                let arrival_dir = dir_from_entry_port(&edge.to.1);
                let mut dot = dot;
                dot.set_direction(arrival_dir);
                self.pending.push(PendingDot {
                    dot,
                    from_node,
                    dest_node: dest,
                    total_ticks: ticks,
                    ticks_left: ticks,
                });
                return;
            }
        }
        self.push_dead_end(DeadEndRecord {
            step: self.step_counter,
            from_node,
            direction: dir,
            selected: dot.selected(),
            addr: dot.addr(),
        });
    }

    #[inline]
    fn push_visit(&mut self, visit: NodeVisitRecord) {
        if self.recent_visits.len() == DEBUG_BUFFER_LIMIT {
            self.recent_visits.pop_front();
        }
        self.recent_visits.push_back(visit);
    }

    #[inline]
    fn push_dead_end(&mut self, dead_end: DeadEndRecord) {
        if self.recent_dead_ends.len() == DEBUG_BUFFER_LIMIT {
            self.recent_dead_ends.pop_front();
        }
        self.recent_dead_ends.push_back(dead_end);
    }
}

fn node_type_name(node_type: &NodeType) -> &'static str {
    match node_type {
        NodeType::CreateDot(_) => "CreateDot",
        NodeType::Print(_) => "Print",
        NodeType::Set(_) => "Set",
        NodeType::Duplicate => "Duplicate",
        NodeType::EndProgram => "EndProgram",
        NodeType::Call(_) => "Call",
        NodeType::CallEnter => "CallEnter",
        NodeType::Return => "Return",
        NodeType::Operator(_, _) => "Operator",
        NodeType::Conditional(_) => "Conditional",
        NodeType::Filter(_) => "Filter",
        NodeType::Assertion(_) => "Assertion",
        NodeType::Select(_) => "Select",
        NodeType::Display(_) => "Display",
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

#[inline]
fn exit_port(dir: Direction) -> PortType {
    match dir {
        Direction::Up => PortType::NORTH,
        Direction::Down => PortType::SOUTH,
        Direction::Left => PortType::WEST,
        Direction::Right => PortType::EAST,
    }
}

#[inline]
fn entry_port(dir: Direction) -> PortType {
    match dir {
        Direction::Up => PortType::SOUTH,
        Direction::Down => PortType::NORTH,
        Direction::Left => PortType::EAST,
        Direction::Right => PortType::WEST,
    }
}

/// Direction a dot is traveling when it arrives through `port`.
#[inline]
fn dir_from_entry_port(port: &PortType) -> Direction {
    match port {
        PortType::SOUTH => Direction::Up,
        PortType::NORTH => Direction::Down,
        PortType::EAST => Direction::Left,
        PortType::WEST => Direction::Right,
    }
}

/// Direction a dot is traveling when it leaves through `port`.
#[inline]
fn dir_from_exit_port(port: &PortType) -> Direction {
    match port {
        PortType::SOUTH => Direction::Down,
        PortType::NORTH => Direction::Up,
        PortType::EAST => Direction::Right,
        PortType::WEST => Direction::Left,
    }
}
