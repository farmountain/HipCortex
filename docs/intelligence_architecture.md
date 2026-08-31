# Intelligence Layer Architecture

The HipCortex intelligence layer adds metacognitive self-awareness, predictive world-modeling, and cross-module coherence to the storage engine. It operates **in-band** (hooked into storage operations) and **out-of-band** (background monitoring cycles).

## Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    INTELLIGENCE LAYER                         │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Self-Model  │  │ World-Model  │  │    Coherence     │   │
│  │              │  │  Enhanced    │  │    Checker       │   │
│  │ • Capability │  │ • Transition │  │ • Consistency    │   │
│  │ • Resource   │  │ • Entity     │  │ • Resolution     │   │
│  │ • Perf       │  │ • Causal     │  │ • Invariants     │   │
│  │ • Health     │  │ • Predict    │  │ • Write-Gating   │   │
│  │ • Decision   │  │ • Uncertain  │  │                  │   │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘   │
│         │                 │                    │             │
├─────────┼─────────────────┼────────────────────┼─────────────┤
│         ▼                 ▼                    ▼             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              STORAGE LAYER                            │   │
│  │  TemporalIndexer  SymbolicStore  ProceduralCache     │   │
│  │  PerceptionAdapter  AureusBridge  MemoryStore        │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Self-Model

**Purpose**: Runtime awareness of system state, capabilities, and health.

### Data Flow
```
Operation Request → can_execute? → Resource Predict → Health Check → Decision
                          │                │               │
                          ▼                ▼               ▼
                   CapabilityRegistry  ResourceMonitor  HealthAggregator
```

### Key Design Decisions
- **EWMA + Bayesian**: Latency uses EWMA smoothing; success rate uses Bayesian Beta-binomial with conjugate prior
- **Weighted Geometric Mean**: Health scores combine multiplicatively so one degraded module drags down the whole
- **Expected Utility**: Decisions maximize `priority × success_rate × health_score − resource_cost`

## World-Model Enhanced

**Purpose**: Learn transition dynamics, track entities, and reason causally.

### Data Flow
```
Observation → TransitionModel.record() → predict_next_state()
                                          │
Entity Observation → KalmanFilter.update() → predict_entity(steps)
                                          │
CausalEdge Addition → do_calculus → counterfactual()
```

### Key Design Decisions
- **Dirichlet-Multinomial**: Natural conjugate prior — predictions are Dirichlet-distributed over next states
- **Linear Kalman**: 2D/3D entity state with configurable process/measurement noise
- **DAG enforcement**: Cycle prevention at edge-addition time (pre-flight check), not post-hoc cleanup
- **Uncertainty decomposition**: Epistemic (model uncertainty, reducible) vs Aleatoric (inherent noise, irreducible)

## Causal SCM Continuous Substrate (v1.2.0)

**Purpose**: Make the causal graph the **primary dynamics driver**, not a side annotation. Discrete causal events are impulses that modify the continuous vector field. Introduced in v1.2.0 to complete AC-3 → AC-6.

### Architecture

```
CognitiveDelta::Intervene/Counterfactual/CreditAssign/RewriteEquation
         │
         ▼
CognitiveHandle.transact() → apply_delta(delta, actor)
         │                          │
         ├─ world.read()            ├─ WorldModelEnhanced.apply_intervention()
         │   .apply_intervention()  ├─ WorldModelEnhanced.counterfactual()
         │                          ├─ WorldModelEnhanced.credit_assign_trajectory()
         │                          └─ WorldModelEnhanced.rewrite_structural_equation()
         │
         └─ memory.lock() → MemoryRecord(Reflexion, actor, audit_action, ...)
```

### Lock Order (must not be violated)

```
world (RwLock<WME>) → causal_graph (RwLock<CausalGraph>) → memory (Mutex) — NEVER held simultaneously
```

### Key Design Decisions
- **`apply_intervention` mutates in-place** (not cloned like `do_operator`) — persistent shared-state changes that survive across transact calls
- **`DigitalTwin.step()` clamps after RK4** — `var_to_dim` map translates entity names to state-vector indices; clamping happens post-integration so ODE dynamics don't override pinned values
- **`rollout_hybrid` carries `causal_nodes`** — `HybridRolloutResult.causal_nodes` enables downstream provenance tracking without giving `SimulationFork` a direct WME reference (isolation preserved)
- **All 4 SCM operators write Reflexion audit records** — every causal graph mutation is traceable in the MemoryStore with actor attribution
- **OOD invariance via residual scoring** — `credit_assign` normalizes residuals by `noise_var`; the node with the highest normalized surprise is `broken_equation`; property-tested across 256 random scenarios

### Causal Graph Public API (v1.2.0 additions)

| Method | Purpose |
|--------|---------|
| `CausalGraph::apply_intervention(var, val)` | Mutating do-calculus: remove incoming edges + pin value |
| `CausalGraph::node_ids()` | Return all node IDs (used by WME wrapper + REST) |
| `WorldModelEnhanced::apply_intervention(var, val)` | Thread-safe wrapper via `causal_graph.write()` |
| `WorldModelEnhanced::rewrite_structural_equation(node, weights)` | Replace `LinearSE` weights in shared graph |
| `WorldModelEnhanced::causal_node_ids()` | Return node IDs for provenance injection |

## Coherence Checker

**Purpose**: Ensure consistency across memory subsystems with detection, resolution, and enforcement.

### Data Flow
```
Write Request → gate_write() → check_invariants()
                   │                  │
                   ├── deny ──────────┘ (critical violation)
                   │
                   └── allow → execute → background check_consistency() [every 60s]
```

### Key Design Decisions
- **Synchronous gating**: Writes are blocked pre-execution if they would violate invariants
- **Three resolution strategies**: Consensus (majority vote), Recency (newest wins), Confidence (highest score wins)
- **Background cycle**: CoherenceChecker runs `check_consistency()` every 60s via tokio::spawn
- **Four invariants**: MemoryConsistency, DecayMonotonicity, GraphAcyclicity, Conservation

## Cross-Module Wiring

Each storage module is wired to the intelligence layer through optional `Arc<>` references:

| Storage Module | Self-Model | World-Model | Coherence |
|---------------|-----------|-------------|-----------|
| TemporalIndexer | HealthReporter, can_execute | observe_transition | validate_entity |
| SymbolicStore | HealthReporter, check_resources | register_entity, update_entity | is_operation_valid |
| ProceduralCache | HealthReporter, can_execute | observe_transition | pre-transition check |
| AureusBridge | get_health | counterfactual, predict | belief-symbolic consistency |
| PerceptionAdapter | check_resources | update_entity | perception consistency |

## Thread Safety

All intelligence modules use `Arc<RwLock<...>>` for shared state. The locking order convention is:

1. CoherenceChecker (outermost)
2. WorldModelEnhanced
3. SelfModel
4. Storage modules (innermost)

Always acquire locks in this order to avoid deadlocks.
