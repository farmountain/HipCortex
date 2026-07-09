## Why

HipCortex is currently a powerful causal memory engine, but its intelligence capabilities suffer from four core architectural bottlenecks: lack of full goal/task graph planning (`Executive Cortex`), simplified causal/counterfactual math stubs (`Deeper World Model Simulation`), basic FIFO-style context eviction with manual skill encoding (`Advanced Attention / Skill Compilation`), and lock contention risks on shared memory (`Arc<RwLock<T>>`) under high write loads. Transitioning HipCortex into a full, production-grade Cognitive Operating System requires formalizing exact goal management, rigorous Pearl do-calculus and MCTS simulation, topological vector decay eviction with automatic skill compilation, and lockless actor-model concurrency.

## What Changes

- **Hierarchical Task Graph (HTN) Solver**: Introduce `TaskGraph` and `TaskNode` (with UUIDs, preconditions, effects, and costs) powered by `petgraph::DiGraph`, running an A* search solver (`solve_planning_problem`) to transition from current belief states to target goals.
- **Active Goal Stack & Scheduler Kernel**: Introduce `ExecutiveScheduler` with `tick()` execution loop that pre-empts execution upon SelfModel health faults or CoherenceChecker gate failures by pushing a pre-emptive Diagnostic Goal Frame.
- **Continuation Checkpoint Persistence**: Extend `ContinuationCheckpoint` with `persist_to_store()` to serialize stack frames (`ContinuationCheckpointData`) as `MemoryType::Reflexion` records into `MemoryStore<B>`.
- **Empirical do-Calculus & SEM Counterfactuals**: Replace mock adjustment heuristics (`0.3 + val * 0.5`) in `CausalGraph` with exact Backdoor Adjustment marginalizing over parent transitions ($P(Y \mid do(X)) = \sum_z P(Y \mid X, Z) P(Z)$), and Pearl's Abduction-Action-Prediction counterfactual pipeline using linear SEM weights (`compute_scm_counterfactual`).
- **MCTS Trajectory Rollout Simulator**: Introduce `MctsSimulator` running UCB1 selection ($\frac{v}{n} + c \sqrt{\frac{\ln N}{n}}$), tree expansion via `TransitionModel::predict()`, and reward backpropagation up to configurable depth limits.
- **Topological & Recency Vector Decay Evictor**: Replace basic eviction in `SessionContext` with `evict_with_topological_decay()`, weighting items via personalized PageRank over `CausalTopoGraph` combined with exponential age decay ($0.7 \times \text{PR} + 0.3 \times e^{-0.001 \times \text{age}}$).
- **Skill Sequence Extractor & FSM Compiler**: Introduce `SkillCompiler` to scan episodic traces above `success_threshold`, parameterize concrete target entities into positional variables (`$arg0`, `$arg1`), and compile templates directly into `ProceduralCache` `FSMTransition` rules.
- **Actor-Model Concurrency Infrastructure**: Introduce `WorldModelActorClient` and `WorldModelMessage` over `tokio::sync::mpsc` and `oneshot` channels to isolate `WorldModelEnhanced` inside a dedicated thread without `Arc<RwLock<T>>` contention.
- **Asynchronous Write-Gating WAL & CoW Audits**: Introduce `CoherenceWriteActor` for non-blocking write buffering and `check_consistency_cow()` to run long background graph verifications on lightweight cloned snapshots (`Arc` clones) of internal databases.

## Capabilities

### New Capabilities
- `executive-cortex`: Covers hierarchical task graph planning (`TaskGraph`, A* state-space solver), active goal stack scheduling (`ExecutiveScheduler::tick`), and continuation checkpoint persistence.
- `procedural-attention`: Covers topological PageRank plus exponential recency decay eviction (`SessionContext::evict_with_topological_decay`), automated skill parameterization (`SkillCompiler::parameterize_trace`), and dynamic procedural FSM transition compilation (`compile_and_register_skill`).
- `actor-concurrency`: Covers lockless actor-model concurrency (`WorldModelActorClient`), asynchronous write-gating WAL (`CoherenceWriteActor`), and Copy-on-Write background consistency auditing (`check_consistency_cow`).

### Modified Capabilities
- `worldmodel-rollout`: Replaces approximate heuristics in causal intervention queries and rollouts with exact empirical backdoor adjustment ($P(Y \mid do(X)) = \sum_z P(Y \mid X, Z) P(Z)$), linear Structural Equation Model counterfactual solvers, and multi-step UCB1 MCTS trajectory simulations (`MctsSimulator`).

## Impact

- **Core Library & Module Map**: Updates `src/lib.rs` to register and export `task_graph`, `executive_scheduler`, `mcts_simulator`, `skill_compiler`, and `actors` sub-modules.
- **Memory Store & Session Management**: Extends `src/modules/session_context.rs`, `src/modules/continuation_checkpoint.rs`, and `src/modules/coherence/` (`mod.rs`, `checker.rs`) with new eviction, WAL buffering, and CoW background auditing APIs.
- **World Model & Procedural Systems**: Extends `src/modules/world_model_enhanced/causal.rs` and `src/modules/procedural_cache/` with empirical causal inference and FSM compilation capabilities.
- **Thread Safety & Performance**: Eliminates read-write lock contention on heavy intelligence operations, significantly improving throughput (`p50` latency) while preserving zero-telemetry, self-hosted guarantees.
