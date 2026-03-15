## Why

HipCortex is currently a sophisticated memory storage system but lacks true intelligence. It cannot reason about its own capabilities, predict future states, or maintain consistency across modules. This makes it a **tool** rather than an **intelligent agent**. To enable AGI-level cognitive capabilities, we need foundational intelligence: self-awareness (metacognition), predictive world modeling, and automatic coherence checking. Without these, HipCortex cannot introspect on its health, anticipate system behavior, or guarantee correctness—critical gaps for production-grade cognitive architectures.

## What Changes

- **Add Self-Model module**: Metacognitive system tracking capabilities, resources, performance, health, and making decisions about operation execution
- **Enhance World-Model**: Transform from simple state storage to predictive system with Bayesian state transitions, entity tracking with Kalman filters, causal graphs for counterfactual reasoning, and uncertainty quantification
- **Add Coherence Checker**: Cross-module consistency validation with automatic conflict resolution and invariant enforcement
- **Integrate intelligence layer**: Wire Self-Model, World-Model, and Coherence Checker to all existing subsystems (TemporalIndexer, SymbolicStore, ProceduralCache, AureusBridge, PerceptionAdapter)
- **Comprehensive testing**: Unit tests (600+), integration tests (300+), property-based tests (200+), UAT scenarios (11+) covering all intelligence capabilities

## Capabilities

### New Capabilities

- `self-model`: Metacognitive awareness including capability registry, resource monitoring with linear regression prediction, performance tracking with EWMA and Bayesian estimation, health aggregation with weighted geometric mean, and decision engine with expected utility maximization
- `world-model-enhanced`: Predictive state modeling with Dirichlet-Multinomial transition learning, Kalman filter entity tracking, causal graphs with do-calculus for interventions, learned predictive models, and uncertainty estimation with confidence intervals
- `coherence-checking`: Cross-module consistency validation detecting temporal-symbolic mismatches, procedural-world conflicts, causal violations, and entity permanence issues; automatic conflict resolution with consensus/recency/confidence strategies; system invariant enforcement (memory consistency, decay monotonicity, graph acyclicity)

### Modified Capabilities

- `temporal-indexer`: Add self-model capability checks before operations, report health metrics, feed observations to world-model
- `symbolic-store`: Add self-model resource checking, coherence validation hooks, sync with world-model entities
- `procedural-cache`: Check self-model before transitions, update world-model on FSM changes, coherence hooks
- `aureus-bridge`: Health checks before reflexion, use world-model for counterfactual reasoning
- `perception-adapter`: Resource validation before PCA, feed perceptions to world-model
- `integration-layer`: Add health endpoints, self-model decision gating, coherence status API

## Impact

**Code:**
- **New files**: 15 files (~8,400 lines) for self_model/, world_model_enhanced/, coherence/
- **Modified files**: 6 existing modules (~800 lines changes) for integration hooks
- **Test files**: 20+ test files (~5,000 lines) for comprehensive coverage

**APIs:**
- New endpoints: `/health/self-model`, `/health/world-model`, `/health/coherence`, `/predict/next-state`, `/query/counterfactual`, `/coherence/status`
- Modified endpoints: All operation endpoints now check self-model before execution

**Performance:**
- Overhead: <10ms per operation (target)
- Memory: +100MB for intelligence structures
- Latency: p95 <500ms for predictions

**Dependencies:**
- No new external dependencies (pure Rust)
- Requires existing modules: TemporalIndexer, SymbolicStore, ProceduralCache, AureusBridge

**Systems Affected:**
- All memory subsystems (Temporal, Symbolic, Procedural)
- All bridges (Aureus, MCP, OpenManus)
- Integration layer and monitoring services
- Test infrastructure (new test categories)

**Migration:**
- Backward compatible—existing code continues to work
- Opt-in intelligence features via configuration flags
- Gradual rollout: Alpha (week 11) → Beta (week 12) → Production (week 14)
- No data migration required (intelligence runs alongside existing data)
