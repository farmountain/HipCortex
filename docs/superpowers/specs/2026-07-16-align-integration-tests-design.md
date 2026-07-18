# Design: Align Integration & Property Tests with Finalized API Signatures

## Context
The core modules (`self_model`, `world_model_enhanced`, `coherence`) and web server endpoints are successfully implemented and pass unit tests. However, the system-level integration and property test suites fail to compile because they use legacy API signatures from prior iterations.

## Goals
- Align test suites with finalized signatures:
  - `DecisionContext`: priority (`f64`), deadline (`Option<Duration>`), user_facing (`bool`), cascading_impact (`bool`).
  - `EntityState` / `EntityObservation`: `properties: Vec<f64>`, `covariance / measurement_noise: Vec<Vec<f64>>`.
  - `DecisionEngine::evaluate`: expects 5 arguments.
  - `Decision`: check `should_execute` instead of `approved`.
- Ensure all 300+ integration and property tests compile and pass.

## Non-Goals
- Modify any core library logic or web server routes.

## Decisions

### 1. Update `tests/integration/intelligence_sit.rs`
- Construct `DecisionContext` with the correct fields.
- Call `DecisionEngine::evaluate` with all 5 arguments.
- Update `EntityState` and `EntityObservation` to use vector-based state and covariance.
- Replace `decision.approved` checks with `decision.should_execute`.

### 2. Update `tests/integration/intelligence_uat.rs`
- Apply the same signature updates to `DecisionContext`, `evaluate` calls, and `approved` checks.

### 3. Update `tests/integration/intelligence_wiring_sit.rs`
- Fix any remaining `DecisionContext` or `EntityState` mismatches.

### 4. Update `tests/property/self_model_props.rs`
- Align `DecisionContext` construction and `evaluate` calls within property-based test strategies.

## Risks & Trade-offs
- None. This is a pure test-alignment change.
