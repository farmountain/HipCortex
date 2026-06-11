# HipCortex Intelligence Layer

**Metacognitive self-awareness, predictive world-modeling, and cross-module coherence for AI memory.**

## Overview

HipCortex is not just a memory store — it's a **cognitive engine** that understands its own state, predicts outcomes, and maintains consistency across its subsystems. The intelligence layer adds three capabilities:

1. **Self-Model** — The system knows what it can do, how healthy it is, and whether it should accept work
2. **World-Model Enhanced** — The system learns from observations, tracks entities through time, and reasons about cause and effect
3. **Coherence Checker** — The system detects conflicts between memory types, resolves them automatically, and enforces invariants

## Quick Start

```rust
use hipcortex::self_model::*;
use hipcortex::world_model_enhanced::*;
use hipcortex::coherence::*;
use std::sync::Arc;

// 1. Create intelligence modules
let self_model = Arc::new(SelfModel::new());
let world_model = Arc::new(WorldModelEnhanced::new());
let coherence = Arc::new(CoherenceChecker::new());

// 2. Wire into storage modules
let mut indexer = TemporalIndexer::new(100, 3600)
    .with_self_model(self_model.clone())
    .with_world_model(world_model.clone())
    .with_coherence(coherence.clone());

// 3. Operations are now intelligence-gated automatically
```

## Self-Model

The self-model gives HipCortex runtime awareness:

### Capability Registry
```rust
let mut registry = CapabilityRegistry::new();
registry.register(CapabilityDescriptor {
    name: "semantic_search".into(),
    description: "Vector similarity search over memory".into(),
    required_cpu_percent: 30.0,
    required_memory_mb: 512.0,
    limitations: vec!["Requires embedding index".into()],
}).unwrap();
```

### Resource Monitoring
```rust
let mut monitor = ResourceMonitor::new();
monitor.record("search", ResourceUsage {
    cpu_percent: 25.0, memory_mb: 512.0,
    disk_io_mbps: 5.0, network_io_mbps: 2.0,
    timestamp: Instant::now(),
}).unwrap();

let prediction = monitor.predict("search").unwrap();
// prediction.confidence → increases with more data
```

### Performance Tracking
Uses EWMA for latency and Beta-binomial Bayesian estimation for success rates:
```rust
let mut tracker = PerformanceTracker::new();
tracker.record(OperationOutcome {
    operation: "search".into(),
    duration: Duration::from_millis(12),
    success: true,
    timestamp: Instant::now(),
}).unwrap();

let metrics = tracker.predict("search").unwrap();
// metrics.latency_ms → EWMA smoothed
// metrics.success_rate → Bayesian estimate with credible interval
```

### Health Aggregation
Weighted geometric mean — one degraded module drags overall health down:
```rust
let mut health = HealthAggregator::new();
health.report("temporal".into(), ModuleHealth {
    latency_ms: 10.0, error_rate: 0.001, resource_usage: 0.3,
}).unwrap();

let overall = health.get_overall_health().unwrap();
assert!(overall.overall >= 0.0 && overall.overall <= 1.0);
```

### Decision Engine
Expected utility maximization:
```rust
let mut engine = DecisionEngine::new();
let decision = engine.evaluate(
    "heavy_computation",
    DecisionContext { priority: 0.3, deadline: None, user_facing: false, cascading_impact: false },
    0.5,   // historical success rate
    ResourceUsage { /* ... */ },
    0.3,   // health score (degraded)
);
// decision.confidence → low under degraded conditions
// decision.action → may reject the operation
```

## World-Model Enhanced

Learns and predicts system dynamics:

### Transition Model (Dirichlet-Multinomial)
```rust
let mut model = TransitionModel::new();
for _ in 0..90 {
    model.record_transition(StateTransition {
        from_state: "idle".into(), action: "process".into(), to_state: "busy".into(),
    }).unwrap();
}

let pred = model.predict("idle", "process").unwrap();
// pred.probabilities["busy"] → ~0.9 after training
let entropy = model.compute_entropy("idle", "process").unwrap();
// Low entropy = confident prediction
```

### Entity Tracking (Kalman Filter)
```rust
let mut wm = WorldModelEnhanced::new();
wm.register_entity("drone_1".into(), EntityState {
    properties: vec![0.0, 0.0, 10.0],  // x, y, altitude
    covariance: vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 2.0],
    ],
}).unwrap();

wm.update_entity("drone_1", EntityObservation {
    measured_properties: vec![0.5, 0.3, 9.8],
    measurement_noise: vec![
        vec![0.1, 0.0, 0.0],
        vec![0.0, 0.1, 0.0],
        vec![0.0, 0.0, 0.2],
    ],
    timestamp: Instant::now(),
}).unwrap();

// Predict 5 steps ahead
let future = wm.predict_entity("drone_1", 5).unwrap();
// Covariance grows with prediction horizon (uncertainty propagation)
```

### Causal Graph (do-calculus)
```rust
let mut graph = CausalGraph::new();
graph.add_node("treatment".into());
graph.add_node("recovery".into());
graph.add_edge("treatment".into(), "recovery".into()).unwrap();

// Cycle prevention: adding recovery→treatment would fail
assert!(graph.add_edge("recovery".into(), "treatment".into()).is_err());

// Query: does treatment affect recovery?
assert!(graph.has_path("treatment", "recovery").unwrap_or(false));
```

## Coherence Checker

Ensures memory subsystems stay consistent:

### Detection (5 types)
| Type | What it catches |
|------|----------------|
| TemporalSymbolicConflict | Entity in one store but not the other |
| ProceduralWorldConflict | FSM state ≠ world-model prediction |
| CausalInconsistency | Causal edge contradicts observed transitions |
| EntityPermanenceViolation | Entity disappeared without deletion |
| BeliefSymbolicInconsistency | Reflexion belief conflicts with fact |

### Resolution (3 strategies)
```rust
// Consensus: majority vote
checker.resolve_all(ResolutionStrategy::Consensus).unwrap();

// Recency: newest timestamp wins
checker.resolve_all(ResolutionStrategy::Recency).unwrap();

// Confidence: highest confidence score wins
checker.resolve_all(ResolutionStrategy::Confidence).unwrap();
```

### Invariants (4 enforced)
| Invariant | Critical? | Effect |
|-----------|-----------|--------|
| Memory Consistency | Yes | Halts operations |
| Decay Monotonicity | No | Logs warning |
| Graph Acyclicity | Yes | Halts operations |
| Conservation | No | Logs warning |

### Write Gating
```rust
match checker.gate_write("insert_record") {
    Ok(()) => { /* proceed with write */ },
    Err(rejection) => {
        eprintln!("Write blocked: {}", rejection.reason);
        // rejection.inconsistencies → what would have broken
        // rejection.violations → which invariants would have been violated
    }
}
```

## REST API

All intelligence capabilities are exposed via REST:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/self/health` | GET | Self-model health scores |
| `/self/capabilities` | GET/POST | List/register capabilities |
| `/self/can-execute` | GET | Check if operation can execute |
| `/worldmodel/predict` | POST | Predict next state |
| `/worldmodel/entities` | GET | List tracked entities |
| `/worldmodel/entity` | POST | Register new entity |
| `/predict/entity/:id` | GET | Predict entity state N steps ahead |
| `/worldmodel/causal` | GET | Dump causal graph |
| `/worldmodel/causal/counterfactual` | POST | "What if" reasoning |
| `/decide/batch` | POST | Batch execution decisions |
| `/coherence/check` | POST | Run full consistency check |
| `/coherence/inconsistencies` | GET | List active inconsistencies |
| `/coherence/resolve/:id` | POST | Resolve specific inconsistency |
| `/health/summary` | GET | Aggregated health from all modules |

## Benchmarks

Run with:
```sh
cargo bench --no-default-features --features "petgraph_backend"
```

| Benchmark | Target | File |
|-----------|--------|------|
| can_execute() latency | <2ms | `benches/self_model_bench.rs` |
| Health aggregation | <5ms | `benches/self_model_bench.rs` |
| State prediction | <10ms | `benches/world_model_bench.rs` |
| Entity tracking | <5ms | `benches/world_model_bench.rs` |
| Consistency check (1000 entities) | <100ms | `benches/coherence_bench.rs` |
| Integration overhead | <10ms | `benches/integration_bench.rs` |
| Throughput | >1000 ops/sec | `benches/integration_bench.rs` |

## Property-Based Tests

Mathematical invariants verified with proptest:

- **Self-Model**: Health ∈ [0,1], predictions non-negative, EWMA stable, success rates ∈ [0,1]
- **World-Model**: Probability sums to 1.0, causal graph acyclic, Kalman covariance PSD, uncertainty monotonic
- **Coherence**: Entity conservation, decay monotonicity, resolution completeness, graph acyclicity

```sh
cargo test --no-default-features --features "petgraph_backend" --test property_suite
```

## Examples

```sh
cargo run --example self_model_quickstart --no-default-features --features petgraph_backend
cargo run --example world_model_example --no-default-features --features petgraph_backend
cargo run --example coherence_checking --no-default-features --features petgraph_backend
```

## Design Decisions

1. **Opt-in architecture**: All intelligence hooks are `Option<Arc<>>` — backward compatible, no forced coupling
2. **Synchronous gating**: Write-gating blocks operations pre-execution, not post-hoc detection
3. **Weighted geometric mean**: Health is multiplicative — one failing module drags the whole system
4. **Dirichlet prior**: Natural conjugate for multinomial transition learning — mathematically clean
5. **DAG enforcement at edge time**: Cycle prevention at insertion, not periodic cleanup
6. **Background coherence**: 60-second cycle via `tokio::spawn` for continuous monitoring

## Further Reading

- [`docs/intelligence_architecture.md`](docs/intelligence_architecture.md) — Full architecture with data flows and thread safety
- [`docs/intelligence_integration_guide.md`](docs/intelligence_integration_guide.md) — Step-by-step guide for adding intelligence hooks to new modules
- [`openspec/changes/intelligence-foundation/`](openspec/changes/intelligence-foundation/) — Original design specs and task breakdown
