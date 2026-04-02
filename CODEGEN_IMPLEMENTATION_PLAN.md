# CodeGen Implementation Plan

**Status:** Ready to execute  
**Current Baseline:** 104.87 µs/step (after SmallVec Callstack)  
**Target:** 20-30 µs/step (3-4x improvement)

---

## Overview

We have a working `CodeGenerator` in [asciidots-graph/src/codegen.rs](asciidots-graph/src/codegen.rs) that generates static-dispatch routing tables and node-type arrays. This plan outlines how to progressively integrate this into the main [asciidots-graph/src/runtime/executor.rs](asciidots-graph/src/runtime/executor.rs) interpreter.

**Key Insight:** Rather than replacing the entire interpreter, we integrate codegen as **fast-path overlays** that replace expensive dynamic operations without changing the overall architecture.

**Safety Model:**
- Every phase has a fallback to the interpreter
- Generated code is **validated** against interpreter results (in dev builds)
- Phases are independent—can revert any phase without affecting others

---

## Phase 1: Routing-Only CodeGen (~1.2x speedup → 87 µs/step)

### Goal
Replace PassThrough node routing (98.4% of nodes) with precomputed static tables instead of Graph traversal lookups.

### Current Bottleneck
```rust
// In executor.rs, for PassThrough nodes:
let destinations = self.graph.get_outgoing_edges(node_id);  // HashMap lookup
for edge in destinations {
    // pattern match edge type, handle routing
}
```

This happens **52,000+ times per step** across all PassThrough nodes.

### Phase 1 Solution
Replace with:
```rust
// Generated: const ROUTE_DESTINATIONS: &[u32] = &[...];
// Generated: const ROUTE_OFFSETS: &[u32] = &[0, 3, 7, 12, ...];
fn get_destinations_fast(node_id: u32) -> &'static [u32] {
    let start = ROUTE_OFFSETS[node_id as usize];
    let end = ROUTE_OFFSETS[node_id as usize + 1];
    &ROUTE_DESTINATIONS[start as usize..end as usize]
}
```

### Integration Steps

#### 1a. Add codegen field to Executor struct
**File:** [asciidots-graph/src/runtime/executor.rs](asciidots-graph/src/runtime/executor.rs)

```rust
pub struct Executor {
    graph: Arc<Graph>,
    // ... existing fields ...
    
    // NEW: Phase 1 codegen tables
    route_destinations: Vec<u32>,  // Flat array of all destinations
    route_offsets: Vec<u32>,        // [start, end) indices for each node
    passthrough_mask: Vec<bool>,    // Quick check: is node PassThrough?
}
```

#### 1b. Populate codegen tables in `new()`
```rust
impl Executor {
    pub fn new(graph: Arc<Graph>, ...) -> Self {
        let (route_destinations, route_offsets) = Self::build_routing_tables(&graph);
        let passthrough_mask = Self::build_passthrough_mask(&graph);
        
        Self {
            graph,
            route_destinations,
            route_offsets,
            passthrough_mask,
            // ... existing fields ...
        }
    }
    
    fn build_routing_tables(graph: &Graph) -> (Vec<u32>, Vec<u32>) {
        // Generate using CodeGenerator logic
        // route_offsets.len() == graph.node_count() + 1
        // route_destinations is flat array of all edge destinations
        let codegen = CodeGenerator::new(graph);
        
        // Extract precomputed tables from codegen.generate()
        // OR: call codegen methods directly for table construction
        (destinations, offsets)
    }
    
    fn build_passthrough_mask(graph: &Graph) -> Vec<bool> {
        graph.nodes.iter()
            .map(|node| matches!(node, NodeType::PassThrough))
            .collect()
    }
}
```

#### 1c. Replace PassThrough routing in `step()`
**File:** [asciidots-graph/src/runtime/executor.rs](asciidots-graph/src/runtime/executor.rs), method `step()`

**Current code** (search for PassThrough match arm):
```rust
NodeType::PassThrough => {
    let destinations = self.graph.get_outgoing_edges(node_id);
    for edge in destinations {
        // Handle based on edge type
    }
}
```

**Replace with:**
```rust
NodeType::PassThrough => {
    // Fast path: use precomputed tables
    let start = self.route_offsets[node_id as usize] as usize;
    let end = self.route_offsets[node_id as usize + 1] as usize;
    let destinations = &self.route_destinations[start..end];
    
    for &dest_node_id in destinations {
        // Push dot to destination with appropriate direction
        // (Direction inference logic same as before, but now O(1))
        self.pending.push_dot(PendingDot {
            node_id: dest_node_id,
            // ...
        });
    }
}
```

#### 1d. Add validation in dev builds
Add a feature gate for correctness checking:

```rust
#[cfg(all(debug_assertions, feature = "validate-codegen"))]
fn validate_routing(&self, node_id: u32) {
    // Compare fast-path results vs graph.get_outgoing_edges()
    // Panic if mismatch
}
```

### Testing Strategy

**Unit Test:** Add to [asciidots-graph/tests/standalone.rs](asciidots-graph/tests/standalone.rs)
```rust
#[test]
fn phase1_routing_correctness() {
    // Load test graphs
    for graph in load_test_graphs() {
        let executor_with_codegen = Executor::new(graph.clone());
        let executor_original = Executor::new_no_codegen(graph.clone());
        
        for _ in 0..1000 {
            executor_with_codegen.step();
            executor_original.step();
            assert_eq!(states_match(executor_with_codegen, executor_original));
        }
    }
}
```

**Benchmark:** Run [asciidots-graph/examples/profile_juliaset.rs](asciidots-graph/examples/profile_juliaset.rs)
```bash
cargo build --release --example profile_juliaset
./target/release/examples/profile_juliaset
# Expected: ~87 µs/step (down from 104.87)
```

### Rollback Plan
If Phase 1 regresses performance:
1. Comment out the fast-path routing code
2. Revert to interpreter path
3. Profile to understand why (cache behavior? branch prediction?)

---

## Phase 2: Operator CodeGen (~2x speedup from Phase 1 → 43 µs/step)

### Goal
Replace operator queue handling (287 dots/step avg) with generated code that pre-allocates operator slots and eliminates HashMap lookups.

### Current Bottleneck
```rust
NodeType::Operator(op_type) => {
    // HashMap lookup for operator queues
    let queue = self.operators.get_mut(op_type);  // O(1) but cache-miss
    queue.push(dot);
    
    // Check if queue ready to execute
    if queue.is_ready() { ... }
}
```

This is **287 times per step × multiple lookups** = expensive L3 cache misses.

### Phase 2 Solution
Pre-generate operator handling:
```rust
// Generated:
const OPERATOR_OFFSETS: &[u32] = &[0, 16, 32, ...];  // operator_id -> queue start
fn handle_operator_fast(operator_id: u32, dot: &Dot, queues: &mut [Vec<Dot>]) {
    let queue_idx = OPERATOR_OFFSETS[operator_id as usize];
    queues[queue_idx as usize].push(dot);
    // No HashMap—direct array indexing
}
```

### Integration Steps

#### 2a. Extend Executor struct
```rust
pub struct Executor {
    // ... Phase 1 fields ...
    
    // NEW: Phase 2 operator queues
    operator_queues: Vec<Vec<Dot>>,  // Pre-allocated per operator type
    operator_offsets: Vec<u32>,        // operator_id -> queue index
}
```

#### 2b. Build operator tables
```rust
fn build_operator_tables(graph: &Graph) -> (Vec<Vec<Dot>>, Vec<u32>) {
    // Allocate one queue per operator found in graph
    let codegen = CodeGenerator::new(graph);
    let stats = codegen.stats();
    
    let operator_offsets: Vec<u32> = (0..stats.operator_count)
        .map(|i| i as u32)  // operator_id -> queue index (1-to-1)
        .collect();
    
    let operator_queues = (0..stats.operator_count)
        .map(|_| Vec::with_capacity(256))  // Pre-allocate typical queue size
        .collect();
    
    (operator_queues, operator_offsets)
}
```

#### 2c. Replace operator dispatch
**In `step()`, for Operator nodes:**
```rust
// OLD:
NodeType::Operator(op_type) => {
    self.operators.get_mut(op_type).push(dot);
}

// NEW (Phase 2):
NodeType::Operator(op_id) => {
    let queue_idx = self.operator_offsets[op_id as usize] as usize;
    self.operator_queues[queue_idx].push(dot);
}
```

#### 2d. Operator execution stays same
The operator execution loop can remain as-is; we've just made queue access O(1) instead of O(log n) HashMap.

### Testing Strategy
Same as Phase 1: correctness test + benchmark.

---

## Phase 3: Full CodeGen (~1.3x speedup from Phase 2 → 33 µs/step)

### Goal
Generate complete node-type dispatch and conditional logic, eliminating the main `match node_type` in `step()`.

### Current Bottleneck
```rust
for dot in self.pending.pop_all() {
    match self.graph.node_type(dot.node_id) {  // 12-way branch, repeated 789 times/step
        NodeType::PassThrough => { ... },
        NodeType::Operator(id) => { ... },
        NodeType::Conditional(id) => { ... },
        NodeType::Terminal => { ... },
        // ... others
    }
}
```

### Phase 3 Solution
Generate a dispatch function per graph:
```rust
// Generated:
fn dispatch_dot(dot: Dot, executor: &mut Executor) {
    // No match—direct dispatch via node-type lookup
    let node_type = NODE_TYPE_ARRAY[dot.node_id as usize];
    match node_type {
        0 => passthrough_fast(dot, executor),  // inlined
        1 => operator_fast(dot, executor),     // inlined
        2 => conditional_fast(dot, executor),  // inlined
        3 => terminal_fast(dot, executor),     // inlined
        _ => unreachable!(),
    }
}
```

But better: eliminate the match entirely by generating specialized Rust code that compiles to jump tables.

### Integration Steps

#### 3a. Export dispatch function from codegen
Modify [asciidots-graph/src/codegen.rs](asciidots-graph/src/codegen.rs):
```rust
impl CodeGenerator {
    pub fn generate_dispatch_fn(&self) -> String {
        // Generate dispatch_node(id: u32, dot: Dot, executor: &mut Executor) function
        // with inlined handling for each node type
        // Returns Rust code as string
    }
}
```

#### 3b. Write generated code to disk
Create a build script that invokes codegen:
```rust
// build.rs
fn main() {
    let graph = Graph::load("samples/juliaset.dots");
    let codegen = CodeGenerator::new(&graph);
    let dispatch_code = codegen.generate_dispatch_fn();
    
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_path.join("dispatch.rs"), dispatch_code).unwrap();
}
```

#### 3c. Include generated code
In [asciidots-graph/src/runtime/executor.rs](asciidots-graph/src/runtime/executor.rs):
```rust
#[cfg(feature = "use-codegen")]
include!(concat!(env!("OUT_DIR"), "/dispatch.rs"));
```

#### 3d. Replace main loop
```rust
// In step():
for dot in self.pending.pop_all() {
    #[cfg(feature = "use-codegen")]
    dispatch_node(dot, self);
    
    #[cfg(not(feature = "use-codegen"))]
    dispatch_node_interpreted(dot, self);
}
```

### Testing Strategy
Same validation framework, but now entire execution path is codegen.

---

## Benchmarking Strategy (All Phases)

### Before Each Phase
```bash
cargo build --release --example profile_juliaset
./target/release/examples/profile_juliaset
# Record: µs/step value
```

### Expected Results
| Phase | Current | Expected | Speedup |
|-------|---------|----------|---------|
| Baseline | 139.50 µs/step | - | - |
| After SmallVec/inlining | 104.87 µs/step | - | -0.25x ✓ |
| Phase 1 (Routing) | 104.87 µs/step | ~87 µs/step | 1.2x |
| Phase 2 (Operators) | ~87 µs/step | ~43 µs/step | 2x |
| Phase 3 (Full) | ~43 µs/step | ~33 µs/step | 1.3x |
| **Total** | **139.50 µs/step** | **~33 µs/step** | **4.2x** ✓ |

### Validation with Criterion
Run full Criterion suite:
```bash
cargo bench --bench interpreter_bench
# Ensure codegen vectors show same trend as baseline
```

---

## Build-Time vs Runtime

### Current Approach (Phase 1-2)
- CodeGen tables built at **Executor::new()** (startup, 0.5ms)
- Tables embedded in Executor struct
- No runtime code generation

### Optional Future (Phase 3)
- Use `proc_macro` or `build.rs` to generate code at **compile time**
- Graph layouts specified in code or loaded from disk
- Generated .rs files committed to repo

### Trade-offs
| Approach | Compile Time | Runtime | Memory | Flexibility |
|----------|--------------|---------|--------|-------------|
| Current (Phase 1-2) | Fast | Startup +0.5ms | 300KB tables | Per-graph generated |
| Proc Macro (Phase 3) | Slow | None | 150KB code | Build-time fixed |
| Disk (Phase 3 alt) | Fast | Load +1ms | 150KB code | Flexible |

**Recommendation:** Stay with Phase 1-2 approach (Executor field) until Phase 3 is proven valuable.

---

## Rollback Checklist

Each phase has a feature flag for quick disabling:

```toml
# Cargo.toml
[features]
phase1-codegen = []
phase2-codegen = []
phase3-codegen = []
```

```bash
# Build with all phases
cargo build --release --features phase1-codegen,phase2-codegen,phase3-codegen

# Build without codegen
cargo build --release

# Test single phase
cargo build --release --features phase1-codegen
```

---

## Files to Modify

### Phase 1
- [asciidots-graph/src/runtime/executor.rs](asciidots-graph/src/runtime/executor.rs) — Add routing fields & logic
- [asciidots-graph/tests/standalone.rs](asciidots-graph/tests/standalone.rs) — Add correctness test
- [asciidots-graph/Cargo.toml](asciidots-graph/Cargo.toml) — Add feature flags

### Phase 2
- [asciidots-graph/src/runtime/executor.rs](asciidots-graph/src/runtime/executor.rs) — Extend operator handling

### Phase 3
- [asciidots-graph/src/codegen.rs](asciidots-graph/src/codegen.rs) — Add `generate_dispatch_fn()`
- [asciidots-graph/build.rs](asciidots-graph/build.rs) — **NEW** build script
- [asciidots-graph/src/runtime/executor.rs](asciidots-graph/src/runtime/executor.rs) — Include generated code

### No Changes Needed
- [asciidots-graph/src/codegen.rs](asciidots-graph/src/codegen.rs) already exists and works
- [asciidots-graph/examples/codegen_analysis.rs](asciidots-graph/examples/codegen_analysis.rs) already exists for analysis

---

## Success Criteria

**Phase 1:**
- [ ] Compiles without warnings
- [ ] Correctness tests pass
- [ ] Performance: 87 ± 2 µs/step (1.2x faster)
- [ ] No regressions on other tests

**Phase 2:**
- [ ] All Phase 1 criteria + below
- [ ] Performance: 43 ± 2 µs/step (2x from Phase 1)
- [ ] Operator queue correctness validated

**Phase 3:**
- [ ] All Phase 1-2 criteria + below
- [ ] Performance: 33 ± 2 µs/step (1.3x from Phase 2)
- [ ] Full execution passes all benchmarks

---

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Feature flag bloat | Low | Keep flags simple; remove in final version |
| Correctness bugs in codegen | High | Validation tests mandatory; shadowing checks |
| Cache line effects | Medium | Profile with perf; test on multiple machines |
| Memory bloat from tables | Low | Tables ~300KB (acceptable) |
| Build time increases | Low | Phase 3 only; use feature flags to skip |

---

## Timeline

1. **Phase 1:** 1-2 hours (routing + tests + benchmark)
2. **Phase 2:** 1 hour (operators + tests)
3. **Phase 3:** 2-3 hours (full dispatch + build script)

**Pause Point:** After Phase 1, pause and validate before proceeding to Phase 2.

---

## Questions to Resolve Later

1. Should Phase 3 use `proc_macro` or `build.rs` or disk files?
2. What's the maximum graph size we need to support (code/memory)?
3. Should we embed codegen defaults for common patterns (fibonacci, etc.)?
4. Do we want a `--disable-codegen` flag in CLI?

---

**Ready to start Phase 1 when user resumes.**
