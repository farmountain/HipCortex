## Context

HipCortex is a local-first, zero-telemetry, production-oriented causal memory engine written in Rust. While it currently provides multi-tiered storage (`MemoryStore`), topological associations (`CausalTopoGraph`), working-set context token budgeting (`WorkingSetBroker`), and Bayesian attribution (`LoopEngine`), its higher-order cognitive processing capabilities exhibit four significant gaps:
1. **Executive Cortex / Planner / Full Goal Management**: The system lacks a structured Hierarchical Task Graph (HTN) planner and goal stack to solve complex goal-directed tasks across multiple turns while handling pre-emptive interrupts (`SelfModel` health faults or `CoherenceChecker` violations) and persisting exact state checkpoints across restarts.
2. **Deeper World Model Simulation**: Causal graph intervention queries (`compute_intervention`) currently rely on mock heuristic adjustments (`0.3 + val * 0.5`). Furthermore, counterfactual analysis lacks formal Structural Equation Model (SEM) abduction-action-prediction pipelines, and multi-step trajectory rollouts lack Monte Carlo Tree Search (MCTS) tree exploration.
3. **Advanced Attention / Skill Compilation**: Context window eviction (`SessionContext`) uses basic recency/cost heuristics rather than topological centrality (personalized PageRank over causal topology graphs). Moreover, successful episodic traces must be manually compiled into procedural FSM rules (`ProceduralCache`) rather than automatically extracted, parameterized (`$arg0`, `$arg1`), and compiled.
4. **Concurrency, Threading, and Locking Bottlenecks**: Intelligence modules use `Arc<RwLock<T>>` across shared memory boundaries. Under high write concurrency, this creates lock contention and deadlock risks between `CoherenceChecker`, `WorldModelEnhanced`, `SelfModel`, and `MemoryStore`.

## Goals / Non-Goals

**Goals:**
- Implement `TaskGraph` (`petgraph::DiGraph<TaskNode, ()>`) with an A* search planning solver over state beliefs (`solve_planning_problem`).
- Implement `ExecutiveScheduler` with a `tick()` kernel that pre-empts execution on health or coherence checks by pushing pre-emptive Diagnostic Goal Frames, and serialize state via `ContinuationCheckpoint::persist_to_store()`.
- Implement exact empirical Backdoor Adjustment ($P(Y \mid do(X)) = \sum_z P(Y \mid X, Z) P(Z)$) over Dirichlet transition counts and Pearl's Abduction-Action-Prediction counterfactual pipeline in `CausalGraph`.
- Implement `MctsSimulator` using UCB1 selection ($\frac{v}{n} + c \sqrt{\frac{\ln N}{n}}$) over `TransitionModel::predict()`.
- Implement `SessionContext::evict_with_topological_decay()`, combining personalized PageRank over `CausalTopoGraph` with exponential age decay ($0.7 \times \text{PR} + 0.3 \times e^{-0.001 \times \text{age}}$).
- Implement `SkillCompiler` to extract and parameterize repeating action sequences (`parameterize_trace`) above `success_threshold` and compile them directly into `ProceduralCache` `FSMTransition` rules.
- Implement `WorldModelActorClient` and `WorldModelActor` over `tokio::sync::mpsc` and `oneshot` channels to run `WorldModelEnhanced` inside a dedicated, lockless thread.
- Implement `CoherenceWriteActor` for asynchronous WAL write buffering and `check_consistency_cow()` to execute long-running consistency audits on lightweight `Arc` clones of internal databases.

**Non-Goals:**
- Removing `petgraph`, `tokio`, or existing core dependencies.
- Breaking existing public API contracts on `MemoryStore`, `LoopEngine`, or `SessionContext` for current consumers.
- Introducing external telemetry, cloud services, or remote network dependencies.

## Decisions

- **Decision 1: `petgraph::DiGraph` for Task Graph & A* State-Space Search over Belief Maps**
  - *Rationale*: `petgraph` is already an established core dependency. Representing `TaskNode` with `HashMap<String, String>` precondition/effect maps allows fast state expansion using an A* open set prioritized by `path_cost + unmet_goals_count`.
  - *Alternatives Considered*: Manual graph pointers via `Rc<RefCell<TaskNode>>` or flat `Vec` indexing. `DiGraph` provides robust cycle detection (`toposort`) and neighbor traversal without borrow checker complexity.
- **Decision 2: Exact Backdoor Adjustment over Dirichlet Transition Tables (`transition.rs`)**
  - *Rationale*: Rather than maintaining separate structural causal tables, the `TransitionModel` already stores conjugate Dirichlet transition counts (`(from_state, action, to_state)`). We compute parent state marginals $P(Z)$ directly from total observed counts across parent variables $Z \in \text{Parents}(X)$, ensuring $P(Y \mid do(X))$ accurately reflects empirical observations.
  - *Alternatives Considered*: Propensity score matching or continuous Gaussian process regression. Exact tabular backdoor adjustments are exact for discrete causal variables and fit seamlessly with existing Laplace smoothing (`alpha`).
- **Decision 3: Linear Structural Equation Model (SEM) with Exogenous Noise Abduction for Counterfactuals**
  - *Rationale*: To answer counterfactual queries ($Y_{X=x}$ given observed $X=x', Y=y'$), we first abduct exogenous noise $U_i = Y_i - \sum_{j \in \text{Parents}(i)} w_{ij} X_j$, replace the structural equation for $X$ with the intervention value $x$, and propagate forward in topological order.
  - *Alternatives Considered*: Full Bayesian MCMC sampling over structural equations. Linear additive SEM with exogenous back-calculation is deterministically fast ($O(V + E)$) and stable for real-time agent execution.
- **Decision 4: Personalized PageRank + Exponential Recency Decay for Context Eviction**
  - *Rationale*: Evicting purely by age or cost removes critical foundational context (`CausalTopoGraph` roots). By computing personalized PageRank (`0.85` damping factor, `10` iterations) seeded with active `TaskGraph` node identifiers, and linearly combining with recency decay ($0.7 \times \text{PR} + 0.3 \times e^{-0.001 \times \text{age}}$), the eviction loop retains causality-critical items inside the context window.
  - *Alternatives Considered*: Pure LFU (Least Frequently Used) or pure LRU. Both fail to account for structural topological relevance to active tasks.
- **Decision 5: Dedicated Tokio Actor Event Loops (`mpsc` + `oneshot`) for World Model Concurrency**
  - *Rationale*: Under concurrent write loads (`observe_transition` while running `predict_next_state` or `search`), `Arc<RwLock<WorldModelEnhanced>>` experiences heavy write-lock starvation. Running `WorldModelEnhanced` inside a dedicated `tokio::spawn` task controlled by `mpsc::channel(buffer_size)` completely eliminates lock contention.
  - *Alternatives Considered*: Fine-grained `dashmap` or lock-free atomic pointers. `dashmap` only protects map entry buckets, while `WorldModelEnhanced` updates multiple dependent internal state maps (`TransitionModel`, `CausalGraph`) atomically per transition.

## Risks / Trade-offs

- **[Risk: High A* Search Complexity on Large Task Spaces]** → *Mitigation*: Limit maximum expanded nodes in `solve_planning_problem` (`max_iterations = 1000`) and enforce `max_depth` inside `ExecutiveScheduler`.
- **[Risk: Sparse Transition Counts in Backdoor Adjustment]** → *Mitigation*: Fall back to Dirichlet uniform priors (`1.0 / parent_states.len()`) whenever `xz_total == 0`.
- **[Risk: Actor Channel Backpressure under Extreme Write Bursts]** → *Mitigation*: Configure `WorldModelActorClient` with a bounded channel capacity (`buffer_size = 4096`) and apply `try_send` / backpressure throttling inside `CoherenceWriteActor`.
- **[Risk: Copy-on-Write Clone Overhead during `check_consistency_cow()`]** → *Mitigation*: The checker only clones lightweight `Arc` smart pointers pointing to underlying shared storage or read-only `CausalTopoGraph` snapshots, avoiding deep heap copies of database records.
