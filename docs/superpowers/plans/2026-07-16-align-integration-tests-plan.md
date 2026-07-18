# Align Integration & Property Tests with Finalized API Signatures Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align system-level integration and property tests with finalized API signatures of `SelfModel`, `DecisionEngine`, and `EntityState` to make the full test suite compile and pass.

**Architecture:** Update test construction patterns and assertions without altering production code.

**Tech Stack:** Rust, cargo, proptest

## Global Constraints
- Do not modify production code in `src/`.
- All tests must compile and pass cleanly via `cargo test --features "web-server"`.

---

### Task 1: Fix `tests/integration/intelligence_sit.rs`

**Files:**
- Modify: `tests/integration/intelligence_sit.rs`

**Interfaces:**
- Consumes: Finalized `DecisionContext`, `Decision`, `DecisionEngine`, `EntityState`, `EntityObservation` APIs in `src/modules/`.

- [ ] **Step 1.1: Fix DecisionContext constructions**
  Update fields in `DecisionContext` in `intelligence_self_model_capability_gating` (line 146) and `intelligence_resource_exhaustion_decision` (line 447) to use:
  `priority: 0.5` (f64 between 0.0 and 1.0), `deadline: None`, `user_facing: false`, `cascading_impact: false`.
- [ ] **Step 1.2: Fix DecisionEngine evaluate calls**
  Update calls to `engine.evaluate` (line 157, 274, 458) to provide 5 arguments matching the production signature:
  `engine.evaluate("operation_name", context, 1.0, resource_usage, 1.0)`
- [ ] **Step 1.3: Fix Decision field checks**
  Change `decision.approved` checks (line 463, 465) to `decision.should_execute`.
- [ ] **Step 1.4: Fix EntityState and EntityObservation constructions**
  In `intelligence_kalman_entity_tracking` (line 290) and observations loop, replace named fields with `properties: vec![0.0, 0.0, 1.0, 0.0]` and `covariance` / `measurement_noise` vector matrices matching the generic Kalman filter.
  Change `timestamp` to `Instant::now()` instead of `SystemTime::now()`.
- [ ] **Step 1.5: Verify compilation**
  Run `cargo test --test intelligence_sit --features "web-server"` to verify it compiles.

### Task 2: Fix `tests/integration/intelligence_uat.rs`

**Files:**
- Modify: `tests/integration/intelligence_uat.rs`

- [ ] **Step 2.1: Fix DecisionContext, evaluate calls, and Decision field checks**
  Update `DecisionContext` construction and `evaluate` calls in `uat_self_model_gating` to match the finalized signature. Replace `.approved` with `.should_execute`.
- [ ] **Step 2.2: Verify compilation**
  Run `cargo test --test intelligence_uat --features "web-server"` to verify UAT tests compile.

### Task 3: Fix `tests/integration/intelligence_wiring_sit.rs`

**Files:**
- Modify: `tests/integration/intelligence_wiring_sit.rs`

- [ ] **Step 3.1: Fix DecisionContext default context and can_execute calls**
  Ensure any `DecisionContext` usage in wiring tests utilizes `DecisionContext::default_context()`.
- [ ] **Step 3.2: Verify compilation**
  Run `cargo test --test intelligence_wiring_sit --features "web-server"` to verify wiring tests compile.

### Task 4: Fix `tests/property/self_model_props.rs`

**Files:**
- Modify: `tests/property/self_model_props.rs`

- [ ] **Step 4.1: Fix property Strategy for DecisionContext**
  Update proptest strategies generating `DecisionContext` to match the new fields.
- [ ] **Step 4.2: Update proptest evaluate calls**
  Update `evaluate` calls in property tests to supply all 5 arguments.
- [ ] **Step 4.3: Verify entire test suite**
  Run `cargo test --features "web-server"` and verify all 300+ tests compile and pass.
