# HipCortex Cohesive Gap Remediation Design Spec

**Date:** 2026-08-13  
**Topic:** Cognitive State Infrastructure Remediation (Bugs 1-3, Actions A1-A11)  
**Status:** IMPLEMENTED  

## 1. Executive Summary

Based on a 1st-principles evaluation of HipCortex against its Acceptance Criteria (`S = (M, W, Self, G, Sk, B, T, P)`), several critical gaps were identified. 
To ensure a long-term, unified, and cohesive solution that does not hack new features onto incompatible structures or break existing functionality, this design proposes a **Tiered Cognitive Architecture**.

**Critical Architectural Constraint (The L0 Boundary):** HipCortex is strictly defined as **L0 Cognitive State Infrastructure**. It provides Storage, Topology, and 1-Step State Dynamics. It is NOT the runtime, the abstraction engine, or the reasoning model. All fixes must respect this boundary and avoid bleeding into L1 (DTCF), L2 (Kakeya ARM), or L4 (Cognitive Runtime).

We will introduce a **Cognitive Garbage Collector (GC)** for mathematical provenance integrity, **Tiered Storage (Hot vs Cold)** to prevent search pollution, and a **Polyglot Persistence Layer (`CognitiveRecord<T>`)** to support S-Tuple components cleanly.

## 2. Sprint 1: Fix Silent Failures (Mathematics & API)

### 2.1 The Rollout API (Bug 3 / A1)
**Design:** Add `POST /worldmodel/rollout` to `web_server.rs` with `mode` (`"dirichlet" | "mcts" | "ensemble"`). Enforce server-side caps (`iterations <= 200`, `max_depth <= 10`).

### 2.2 Causal Intervention Boundary (Bug 1 / A2)
**Design:** 
- Redirect `compute_counterfactual` to call the mathematically correct `compute_scm_counterfactual`.
- Remove the linear heuristic from `compute_intervention`. Return explicit `Err` if empirical distributions are missing.

### 2.3 Kalman Covariance Stability (Bug 2 / A3)
**Design:** Implement the numerically stable Joseph form update: `P = (I - KH) P (I - KH)^T + K R K^T`. Add a symmetrization step to prevent asymmetric float drift.

### 2.4 Merkle Chain Test Correction (A10)
**Design:** Update the Python test `assert_merkle_chain_integrity` to match the production `compute_hash` implementation (serializing all non-hash fields).

## 3. Sprint 2: The Unified Storage Foundation (Traps 2 & 3)

### 3.1 Polyglot Persistence Layer (Trap 3 / A5, A6, A7)
**Problem:** Forcing Goals, Skills, and Beliefs into the existing `MemoryRecord` destroys Rust type-safety. Building 5 separate databases duplicates persistence and Merkle logic.
**Cohesive Design:**
- Introduce a unified envelope: `CognitiveRecord<T>`.
- Define strongly-typed payloads: `MemoryPayload`, `GoalPayload`, `SkillPayload`, `BeliefPayload`.
- **Non-Breaking Migration:** Create an alias `pub type MemoryRecord = CognitiveRecord<MemoryPayload>;` and implement `Deref` to ensure all existing HipCortex code that depends on `MemoryRecord` continues to work without modification.

### 3.2 Tiered Storage (Trap 2 / A8)
**Problem:** Setting `status = "archived"` pollutes the RAG pipeline because `MemoryQuery::search` ignores status flags.
**Cohesive Design:** 
- Implement **Hot vs Cold Storage**.
- The existing `OptimizedMemoryStore` acts as the Hot Store (Active State).
- A new `ArchiveMemoryStore` (Cold Store) houses superseded beliefs and raw temporal traces.
- `MemoryQuery` targets the Hot Store by default, naturally solving search pollution while preserving historical data for explicit audit queries.

## 4. Sprint 3: Cognitive GC & Provenance (Trap 1)

### 4.1 Provenance Graph (A4)
**Design:** Add `evidence: Vec<Uuid>` and `derived_from: Option<Uuid>` to `CognitiveRecord<T>`. 

### 4.2 Cognitive Garbage Collector (Trap 1)
**Problem:** Temporal traces decay and are dropped. If they are dropped, Beliefs lose their evidence (dangling pointers).
**Cohesive Design:**
- Implement **Provenance Reachability (Tracing GC)**.
- When a record decays to `0.0`, the Cognitive GC checks its in-degree (is it referenced by any Belief's `evidence` array?).
- **If referenced:** Move to Cold Store (preserves referential integrity for `P` tuple).
- **If unreferenced:** Hard delete (frees space).

### 4.3 StateDiff API (A11)
**Design:** Implement Level 2 Content-Aware Diff operating on `CognitiveRecord<T>` snapshots. Captures field changes, confidence deltas, and WorldModel state drift.

## 5. Sprint 4: The L0 Purity Extractions (Refactoring Encroachments)

To prepare HipCortex for independent commercialization as a Cognitive State Infrastructure (L0), we must extract logic that violates architectural boundaries.

### 5.1 Extract Execution Logic to L4 (Cognitive Runtime)
**Problem:** `SkillProcedure` triggers and `DecisionEngine` goal evaluations (A5/A6) incorrectly execute within L0.
**Cohesive Design:** HipCortex will only provide the CRUD API for `SkillPayload` and `GoalPayload`. All ReAct loops, tool execution, and harness verification logic will be deferred to the external L4 Cognitive Runtime.

### 5.2 Extract Consolidation Logic to L1 (DTCF)
**Problem:** Embedding consolidation and semantic clustering (A8 Level 2) are Abstraction Formation tasks (L1).
**Cohesive Design:** HipCortex provides Tiered Storage (Hot/Cold) and basic GC. The semantic synthesis algorithms will be deferred to the external L1 DTCF engine.

### 5.3 Extract Planning Logic to L2 (Kakeya ARM)
**Problem:** MCTS Rollout (`rollout_mcts_goal`) is currently inside HipCortex (Bug 3).
**Cohesive Design:** HipCortex will expose basic N-step Taichi state evolution (`predict_multi_step`). The actual Counterfactual Branching, Planning, and Action Selection tree (MCTS) will be extracted to the L2 Kakeya engine.

### 5.4 Dynamic Kalman Transitions & Causal Auto-Population (B2c / A2b)
**Design:** Complete the pure L0 state dynamics:
1. Make the `EntityTracker` `F` transition matrix configurable for constant-velocity targets.
2. Auto-populate `CausalGraph` empirical distributions directly from the `TransitionModel` observations.

---
**Self-Review Checklist:**
- [x] Unified architecture (Cognitive GC, Tiered Storage, CognitiveRecord).
- [x] Backwards compatibility maintained via Rust type aliases and Deref.
- [x] No dangling pointers in the provenance graph.
- [x] Mathematically sound.
