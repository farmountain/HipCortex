## 1. Executive Cortex / Planner / Full Goal Management

- [x] 1.1 Implement `TaskState`, `TaskNode`, and `TaskGraph` (`petgraph::DiGraph<TaskNode, ()>`) in `src/modules/executive/task_graph.rs` with `solve_planning_problem` A* state-space search solver over belief maps.
- [x] 1.2 Implement `StackFrame` and `ExecutiveScheduler` in `src/modules/executive/scheduler.rs` with `tick()` kernel asserting `SelfModel::is_healthy()` and `CoherenceChecker::gate_write()`, pushing a pre-emptive Diagnostic Goal Frame on faults.
- [x] 1.3 Implement `ContinuationCheckpointData` serialization and `persist_to_store()` in `src/modules/continuation_checkpoint.rs` to write active `goal_stack` frames as `MemoryType::Reflexion` records into `MemoryStore<B>`.

## 2. Deeper World Model Simulation

- [x] 2.1 Implement `CausalGraph::compute_empirical_intervention` in `src/modules/world_model_enhanced/causal.rs` using exact Backdoor Adjustment marginalizing Dirichlet transition counts ($P(Y \mid do(X)) = \sum_z P(Y \mid X, Z) P(Z)$).
- [x] 2.2 Implement `CausalGraph::compute_scm_counterfactual` in `src/modules/world_model_enhanced/causal.rs` implementing Pearl's three-step Abduction-Action-Prediction pipeline via linear structural equation weights.
- [x] 2.3 Implement `SimulatorNode` and `MctsSimulator` in `src/modules/world_model_enhanced/simulator.rs` with `search()` trajectory rollout engine using UCB1 branch selection and `TransitionModel::predict()`.

## 3. Advanced Attention / Skill Compilation

- [x] 3.1 Implement `SessionContext::evict_with_topological_decay` in `src/modules/session_context.rs`, computing personalized PageRank over `CausalTopoGraph` combined with exponential recency decay ($0.7 \times \text{PR} + 0.3 \times e^{-0.001 \times \text{age}}$).
- [x] 3.2 Implement `SkillTemplate` and `SkillCompiler::parameterize_trace` in `src/modules/procedural_cache/skill_compiler.rs` to extract repeating action traces above `success_threshold` and abstract concrete target entities into `$arg` variables.
- [x] 3.3 Implement `SkillCompiler::compile_and_register_skill` in `src/modules/procedural_cache/skill_compiler.rs` to compile parameterized `SkillTemplate` actions into `FSMTransition` rules inside `ProceduralCache`.

## 4. Concurrency, Threading, and Locking Bottlenecks

- [x] 4.1 Implement `WorldModelMessage` and `WorldModelActorClient` in `src/actors/world_model_actor.rs` using `tokio::sync::mpsc` and `oneshot` channels to run owned `WorldModelEnhanced` state mutations inside a dedicated thread without `Arc<RwLock<T>>` contention.
- [x] 4.2 Implement `CoherenceWriteActor` (`tokio::spawn` worker) inside `src/modules/coherence/mod.rs` to buffer write mutations in an asynchronous WAL channel and trigger `SnapshotManager::rollback_to_latest()` on critical invariant violations.
- [x] 4.3 Implement `CoherenceChecker::check_consistency_cow` inside `src/modules/coherence/checker.rs` to clone `Arc` smart pointer snapshots (`temporal_indexer`, `symbolic_store`, `procedural_cache`, `CausalTopoGraph`) and run long background verifications on an isolated thread.

## 5. Module Wiring & Integration Verification

- [x] 5.1 Update `src/lib.rs` path mappings and re-exports (`task_graph`, `executive_scheduler`, `mcts_simulator`, `skill_compiler`, `actors`) to register all new modules cleanly into the crate hierarchy.
- [x] 5.2 Verify that `cargo check --no-default-features --features "petgraph_backend" --lib` and `cargo test --no-default-features --features "petgraph_backend" --lib` compile and pass all automated tests across the new and existing cognitive modules.

## 6. Verification Hardening & CI Feature Flags

- [x] 6.1 Add comprehensive `#[cfg(all(test, feature = "tokio"))]` unit tests to `src/actors/world_model_actor.rs` and `src/modules/coherence/write_actor.rs`, and update `.github/workflows/ci.yml` (`build-core`) to explicitly execute builds and test suites with `--features "petgraph_backend,tokio"`.
