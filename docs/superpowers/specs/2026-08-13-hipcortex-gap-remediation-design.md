# HipCortex Gap Remediation Design Spec

**Date:** 2026-08-13  
**Topic:** Cognitive State Infrastructure Remediation (Bugs 1-3, Actions A1-A11)  
**Status:** PROPOSED  

## 1. Executive Summary

Based on a 1st-principles evaluation of HipCortex against its Acceptance Criteria (acting as an independently shippable Cognitive State Infrastructure S = (M, W, Self, G, Sk, B, T, P)), several critical gaps were identified. This design document specifies the architecture and implementation path to remediate these gaps across 4 sequenced sprints, prioritizing the elimination of silent mathematical failures before completing the S-tuple components.

## 2. Sprint 1: Fix Silent Failures (Highest Priority)

### 2.1 The Rollout API (Bug 3 / A1)
**Problem:** `WorldModelEnhanced` has multiple prediction methods (`rollout_dirichlet`, `rollout_mcts_goal`, `predict_multi_step`), but the REST API `/worldmodel/rollout` is missing, causing external agent calls to 404.
**Design:**
- Add `POST /worldmodel/rollout` to `web_server.rs`.
- Request schema includes `mode` (`"dirichlet" | "mcts" | "ensemble"`), `initial_state`, `actions`, `goal_state`, `iterations`, and `max_depth`.
- Enforce server-side caps (`iterations <= 200`, `max_depth <= 10`) to prevent resource exhaustion.
- Dispatcher cleanly routes to the existing internal Rust methods based on `mode`.

### 2.2 Causal Intervention Boundary (Bug 1 / A2)
**Problem:** `compute_intervention` uses a fabricated linear heuristic (`0.3 + 0.5 * x`) instead of true Pearl backdoor adjustment. `compute_counterfactual` uses a similar heuristic instead of the correct Structural Causal Model (SCM) pipeline.
**Design:**
- **Counterfactuals:** Redirect `compute_counterfactual` to call `compute_scm_counterfactual`, which correctly implements abduction-action-prediction.
- **Interventions:** Remove the heuristic from `compute_intervention`. If empirical distributions are absent, return `Err("Distributions not loaded. Call record_empirical_distribution() first.")` to establish an honest mathematical boundary.

### 2.3 Kalman Covariance Stability (Bug 2 / A3)
**Problem:** The standard Kalman update `P = (I - KH)P` is numerically unstable in floating-point math, leading to negative diagonal entries and NaN confidence intervals.
**Design:**
- Implement the Joseph form update: `P = (I - KH) P (I - KH)^T + K R K^T`.
- Add a symmetrization step `P = (P + P^T) / 2` to prevent asymmetric drift.
- This guarantees positive semi-definiteness (PSD) and preserves Mahalanobis anomaly detection.

### 2.4 Merkle Chain Test Correction (A10)
**Problem:** The test `assert_merkle_chain_integrity` uses a weak hashing formula `sha256(prev_hash + rec_id + content)`, incorrectly assuming the production system drops metadata and provenance fields.
**Design:** Update the Python test assertion to match the production `compute_hash` implementation (which safely serializes all non-hash fields).

## 3. Sprint 2: Provenance + Measurability

### 3.1 Provenance Graph (A4)
**Problem:** `MemoryRecord` tracks *origin* (`source`) but not *evidence* (observations supporting a belief) or *derivation* (how a consolidated belief was formed).
**Design:**
- Add `evidence: Vec<Uuid>` (default empty) to `MemoryRecord`.
- Add `derived_from: Option<Uuid>` (default None) to `MemoryRecord`.
- Ensure new fields are included in the Merkle hash preimage (already handled by default serde).
- Add REST endpoints: `GET /memory/{id}/evidence` and `GET /memory/{id}/derived_from`.

### 3.2 StateDiff API (A11)
**Problem:** Diffing memory snapshots only checks UUID additions/removals, missing mutations (e.g., confidence changes) and WorldModel/Entity state drift.
**Design:**
- Implement Level 2 Content-Aware Diff: detect field changes, confidence deltas, and version bumps for shared UUIDs.
- Include `EntityDelta` (changes to Kalman mean/covariance) and `TransitionDelta` for WorldModel state.
- Expose `POST /memory/checkpoint` and `GET /memory/diff` REST routes.

### 3.3 Safe Consolidation Level 1 (A8)
**Problem:** Consolidation destructively deletes the older record, destroying provenance.
**Design:** Change the consolidation loop to update the `status` of superseded records to `"archived"` rather than deleting them.

## 4. Sprint 3: Algorithmic Upgrades

### 4.1 Embedding Consolidation (A8 Level 2)
**Design:** Replace O(n²) Jaccard overlap with embedding-based cosine similarity (via `SemanticCache`). Consolidate clusters into a single new `Symbolic` record with `evidence` pointing to the archived cluster members.

### 4.2 WorldModel → SelfModel Metacognition (A9)
**Design:** Wire WorldModel calibration error (ECE) and entropy trends into a `CognitiveQuality` metric. The `SelfModel`'s `HealthAggregator` will consume this to degrade its overall health score when predictions lose accuracy.

### 4.3 Causal Auto-Population (A2b)
**Design:** When the `TransitionModel` observes `(s, a, s')`, auto-populate the empirical distributions for the `CausalGraph` to enable seamless backdoor adjustment without manual data entry.

### 4.4 Configurable Kalman Dynamics (B2c)
**Design:** Add `transition_model: Option<Vec<Vec<f64>>>` to `EntityTracker` to allow non-identity `F` matrices (e.g., constant velocity dynamics).

## 5. Sprint 4: S-Tuple Completeness (G, Sk, B)

### 5.1 Goal Component (G) (A5)
**Design:** Create a first-class `Goal` struct (target condition, priority, status, deadline, evidence links). Wire it into `DecisionEngine.evaluate()` to boost approval for operations aligned with active goals.

### 5.2 Skill Module (Sk) (A6)
**Design:** Create `src/modules/skill_library.rs`. Define `Skill` with pre-conditions, post-conditions, execution procedure (REST/MCP/Composite), and a learned competence score. Maintain `CapabilityDescriptor` strictly for resource profiling.

### 5.3 Belief Propagation (B) (A7)
**Design:** Separate persistent `Belief` from temporary `Hypothesis`. Implement true Bayesian soft-evidence updates `P(H|E)`. Add topological confidence propagation along `implies` edges.

---
**Self-Review Checklist:**
- [x] No ambiguous placeholders.
- [x] Clear scope (4 explicit sprints).
- [x] Trade-offs acknowledged (prioritized math fixes over features).
- [x] Mathematically sound.
