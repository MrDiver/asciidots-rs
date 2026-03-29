// TODO: TUI visualization for the graph runtime — display a grid where each cell is a Node
// (using node.position), and dots are rendered as interpolated positions between their
// from-node and to-node based on ticks_left/ticks. Reuse the existing asciidots-tui
// infrastructure but feed it graph IR instead of the raw AsciiField.

use rustc_hash::FxHashMap;

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

struct PendingDot {
    dot: GraphDot,
    dest_node: NodeId,
    ticks_left: u64,
}

pub struct GraphExecutor {
    graph: Graph,
    /// Outgoing edge indices per node, indexed by NodeId.
    outgoing: Vec<Vec<usize>>,
    pending: Vec<PendingDot>,
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
}

impl GraphExecutor {
    pub fn new(
        graph: Graph,
        native_registry: FxHashMap<String, Box<dyn NativeFunction<GraphDot>>>,
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
            pending: Vec::new(),
            operator_queues,
            conditional_queues,
            native_registry,
            native_returned: Vec::new(),
            waiting_for_input: FxHashMap::default(),
            input_counter: 0,
            results: Vec::new(),
            finished: false,
            empty_steps: 0,
            graph,
        };

        // Seed initial dots from CreateDot nodes.
        for node_id in 0..executor.graph.nodes.len() {
            if let NodeType::CreateDot(dir) = executor.graph.nodes[node_id].t {
                let dot = GraphDot::new(dir);
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

    pub fn step(&mut self) -> &[InterpreterResult] {
        self.results.clear();

        // Collect returns from native functions.
        self.collect_native_returns();

        // Fire dots that have arrived (ticks_left == 0), decrement the rest.
        let mut i = 0;
        let mut arrived: Vec<PendingDot> = Vec::new();
        while i < self.pending.len() {
            if self.pending[i].ticks_left == 0 {
                arrived.push(self.pending.swap_remove(i));
            } else {
                self.pending[i].ticks_left -= 1;
                i += 1;
            }
        }

        if arrived.is_empty()
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

        for pd in arrived {
            self.process_dot(pd.dot, pd.dest_node);
        }

        &self.results
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

    fn process_dot(&mut self, mut dot: GraphDot, node_id: NodeId) {
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
                        PrintKind::Normal(s) | PrintKind::Stream(s) => {
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
                let edge_indices: Vec<usize> = self.outgoing[node_id].clone();
                for edge_idx in edge_indices {
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
                        dest_node: dest,
                        ticks_left: ticks,
                    });
                }
            }

            NodeType::Operator(_, _) => {
                if let Some(queue) = self.operator_queues.get_mut(&node_id) {
                    if let Some(result) = queue.enter(dot) {
                        self.forward_dot(result, node_id);
                    }
                }
            }

            NodeType::Conditional(_) => {
                if let Some(queue) = self.conditional_queues.get_mut(&node_id) {
                    if let Some(result) = queue.enter(dot) {
                        let dir = result.direction();
                        self.send_in_direction(result, node_id, dir);
                    }
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
                    let edge_indices: Vec<usize> = self.outgoing[call_node_id].clone();
                    for edge_idx in edge_indices {
                        let edge = &self.graph.edges[edge_idx];
                        let dest = edge.to.0;
                        let ticks = edge.ticks;
                        let arrival_dir = dir_from_entry_port(&edge.to.1);
                        let mut clone = dot.clone();
                        clone.set_direction(arrival_dir);
                        self.pending.push(PendingDot {
                            dot: clone,
                            dest_node: dest,
                            ticks_left: ticks,
                        });
                    }
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
                CallKind::AsciiCall(id) => {
                    let id = *id;
                    dot.push_callstack(node_id);
                    if let Some(&entry_node) = self.graph.func_entry_nodes.get(&id) {
                        self.pending.push(PendingDot {
                            dot,
                            dest_node: entry_node,
                            ticks_left: 0,
                        });
                    }
                }
            },
        }
    }

    /// Send `dot` along the outgoing edge that exits `from_node` in `dot.direction()`.
    fn forward_dot(&mut self, dot: GraphDot, from_node: NodeId) {
        let dir = dot.direction();
        self.send_in_direction(dot, from_node, dir);
    }

    /// Send `dot` along the outgoing edge that exits `from_node` via `exit_port(dir)`.
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
                    dest_node: dest,
                    ticks_left: ticks,
                });
                return;
            }
        }
        // No matching edge — dot travels off the graph and dies.
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

fn exit_port(dir: Direction) -> PortType {
    match dir {
        Direction::Up => PortType::NORTH,
        Direction::Down => PortType::SOUTH,
        Direction::Left => PortType::WEST,
        Direction::Right => PortType::EAST,
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

/// Direction a dot is traveling when it arrives through `port`.
fn dir_from_entry_port(port: &PortType) -> Direction {
    match port {
        PortType::SOUTH => Direction::Up,
        PortType::NORTH => Direction::Down,
        PortType::EAST => Direction::Left,
        PortType::WEST => Direction::Right,
    }
}

/// Direction a dot is traveling when it leaves through `port`.
fn dir_from_exit_port(port: &PortType) -> Direction {
    match port {
        PortType::SOUTH => Direction::Down,
        PortType::NORTH => Direction::Up,
        PortType::EAST => Direction::Right,
        PortType::WEST => Direction::Left,
    }
}
