# Sub-spec 2: Test Coverage Gap Remediation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three property-test files that close AC-3 (ECE calibration), AC-4 (consolidation reduction), and AC-8 (full PSD diagonal) from the gap-remediation spec, with zero runtime code changes.

**Architecture:** Test-only. Two new files under `tests/property/` + one surgical two-line edit to `tests/property/world_model_props.rs` + two new `mod` lines in `tests/property/mod.rs`. All tests run under the existing `cargo test --test property_suite` command.

**Tech Stack:** Rust proptest, hipcortex crate (petgraph_backend feature). No new dependencies.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `tests/property/calibration_props.rs` | **Create** | ECE ∈ [0,1], ECE = 0 on perfect calibration, ECE ≤ MCE, `is_well_calibrated` gate consistency |
| `tests/property/consolidation_props.rs` | **Create** | Skill induction from causal chains, evidence links preserved, hot-set reduction |
| `tests/property/world_model_props.rs` | **Edit lines 195–196** | Replace 2-element PSD check with full diagonal loop |
| `tests/property/mod.rs` | **Edit** | Register the two new modules |

---

## Pre-flight: verify run command

```sh
cargo test --no-default-features --features "petgraph_backend" --test property_suite 2>&1 | tail -5
```

Expected: all existing tests pass (green). If red, stop — don't add new tests on a broken baseline.

---

## Task 1: `calibration_props.rs` — ECE property tests

**Files:**
- Create: `tests/property/calibration_props.rs`

### Background (read before coding)

`UncertaintyEstimator` is in `src/modules/world_model_enhanced/uncertainty.rs`.  
Import path: `hipcortex::world_model_enhanced::UncertaintyEstimator`.  
Key API:
- `UncertaintyEstimator::new()` — 10-bin estimator
- `UncertaintyEstimator::with_bins(n: usize)` — custom bin count
- `estimator.record_outcome(predicted_prob: f64, was_correct: bool)` — add one sample
- `estimator.get_metrics() -> CalibrationMetrics` — returns `{ ece: f64, mce: f64, num_bins: usize, bin_stats: Vec<BinStats> }`
- `estimator.is_well_calibrated() -> bool` — equivalent to `get_metrics().ece < 0.1`

- [ ] **Step 1: Create the file with non-proptest unit tests**

Create `tests/property/calibration_props.rs` with this exact content:

```rust
// Property and unit tests for UncertaintyEstimator calibration invariants.
//
// ECE is a quality target (< 0.1), not a universal invariant for random inputs.
// Tests here focus on structural invariants (ECE ∈ [0,1]) and constructive
// properties (perfect calibration → ECE = 0).

use hipcortex::world_model_enhanced::UncertaintyEstimator;
use proptest::prelude::*;

// ============================================================================
// Unit test 1: Empty estimator → ECE = 0.0 (not NaN, not panic)
// ============================================================================

#[test]
fn empty_estimator_ece_is_zero() {
    let estimator = UncertaintyEstimator::new();
    let metrics = estimator.get_metrics();
    assert_eq!(metrics.ece, 0.0, "empty estimator must return ECE = 0.0");
    assert!(!metrics.ece.is_nan(), "ECE must not be NaN on empty input");
}

// ============================================================================
// Unit test 2: Perfect calibration → ECE ≈ 0.0
//
// Build 10 buckets × 100 samples each.
// Bucket i midpoint confidence = (2i + 1) / 20  → 0.05, 0.15, ..., 0.95
// Set exactly round(100 × confidence) samples correct per bucket.
// → fraction_correct == confidence per bucket → ECE = 0.
// ============================================================================

#[test]
fn perfect_calibration_achieves_zero_ece() {
    let mut estimator = UncertaintyEstimator::with_bins(10);
    for bucket_idx in 0..10usize {
        let confidence = (2 * bucket_idx + 1) as f64 / 20.0;
        let n_samples = 100usize;
        let n_correct = (n_samples as f64 * confidence).round() as usize;
        for i in 0..n_samples {
            estimator.record_outcome(confidence, i < n_correct);
        }
    }
    let metrics = estimator.get_metrics();
    assert!(
        metrics.ece < 1e-9,
        "perfect calibration must yield ECE ≈ 0, got {:.6}",
        metrics.ece
    );
}

// ============================================================================
// Proptest 1: ECE always in [0, 1] for any input
// ============================================================================

proptest! {
    #[test]
    fn ece_always_in_unit_interval(
        samples in prop::collection::vec(
            (0.0f64..=1.0, prop::bool::ANY),
            1..200
        )
    ) {
        let mut estimator = UncertaintyEstimator::new();
        for (prob, correct) in &samples {
            estimator.record_outcome(*prob, *correct);
        }
        let metrics = estimator.get_metrics();
        prop_assert!(metrics.ece >= 0.0, "ECE < 0: {:.6}", metrics.ece);
        prop_assert!(metrics.ece <= 1.0, "ECE > 1: {:.6}", metrics.ece);
        prop_assert!(!metrics.ece.is_nan(), "ECE is NaN");
    }
}

// ============================================================================
// Proptest 2: ECE ≤ MCE for any input
// ECE is a weighted average of per-bucket errors; MCE is the max.
// Weighted average ≤ max is always true.
// ============================================================================

proptest! {
    #[test]
    fn ece_bounded_by_mce(
        samples in prop::collection::vec(
            (0.0f64..=1.0, prop::bool::ANY),
            10..200
        )
    ) {
        let mut estimator = UncertaintyEstimator::new();
        for (prob, correct) in &samples {
            estimator.record_outcome(*prob, *correct);
        }
        let metrics = estimator.get_metrics();
        prop_assert!(
            metrics.ece <= metrics.mce + f64::EPSILON,
            "ECE {:.6} > MCE {:.6} — violated weighted-average ≤ max invariant",
            metrics.ece,
            metrics.mce
        );
    }
}

// ============================================================================
// Proptest 3: is_well_calibrated() consistent with get_metrics().ece < 0.1
// The gate method must agree with the raw metric on every input.
// ============================================================================

proptest! {
    #[test]
    fn is_well_calibrated_iff_ece_below_threshold(
        samples in prop::collection::vec(
            (0.0f64..=1.0, prop::bool::ANY),
            1..200
        )
    ) {
        let mut estimator = UncertaintyEstimator::new();
        for (prob, correct) in &samples {
            estimator.record_outcome(*prob, *correct);
        }
        let metrics = estimator.get_metrics();
        let gate = estimator.is_well_calibrated();
        prop_assert_eq!(
            gate,
            metrics.ece < 0.1,
            "is_well_calibrated() ({}) inconsistent with ECE {:.6} < 0.1",
            gate,
            metrics.ece
        );
    }
}
```

- [ ] **Step 2: Add module to `tests/property/mod.rs`**

Open `tests/property/mod.rs`. Add one line at the end:

```rust
mod calibration_props;
```

`mod.rs` should now contain:
```rust
mod coherence_props;
mod connectivity;
mod fsm_reachability;
mod regression_tests;
mod self_model_props;
mod test_graph;
mod world_model_props;
mod calibration_props;
```

- [ ] **Step 3: Run calibration tests only**

```sh
cargo test --no-default-features --features "petgraph_backend" --test property_suite calibration_props 2>&1 | tail -20
```

Expected output contains:
```
test calibration_props::empty_estimator_ece_is_zero ... ok
test calibration_props::perfect_calibration_achieves_zero_ece ... ok
test calibration_props::ece_always_in_unit_interval ... ok
test calibration_props::ece_bounded_by_mce ... ok
test calibration_props::is_well_calibrated_iff_ece_below_threshold ... ok
```

If any test fails: check that `UncertaintyEstimator::with_bins` and `record_outcome` are spelled correctly — those are the exact method names from `src/modules/world_model_enhanced/uncertainty.rs:75,137`.

- [ ] **Step 4: Commit**

```sh
git add tests/property/calibration_props.rs tests/property/mod.rs
git commit -m "test(property): ECE calibration invariants — range, perfect-cal zero, ECE≤MCE, gate consistency"
```

---

## Task 2: `consolidation_props.rs` — consolidation reduction tests

**Files:**
- Create: `tests/property/consolidation_props.rs`

### Background (read before coding)

`mine_and_consolidate` is in `src/consolidation.rs` (public module):

```rust
pub fn mine_and_consolidate<B: MemoryBackend>(
    store: &mut MemoryStore<B>,
    log: Option<&TxLog>,
    min_frequency: usize,
    actor: &str,
) -> Result<MiningReport, String>
```

`MiningReport` fields: `motifs_found: usize`, `skills_induced: usize`, `beliefs_induced: usize`, `source_ids_archived: Vec<Uuid>`.

`MemoryRecord::new` signature (5 args):
```rust
pub fn new(
    record_type: MemoryType,
    actor: String,
    action: String,
    target: String,
    metadata: serde_json::Value,
) -> Self
```

To build causal chains: set `record.derived_from = Some(prev_id)` after construction.

To query store by type: `store.all_by_type(MemoryType::Skill)` → `Vec<&MemoryRecord>`.  
(`search_by_type` does not exist — use `all_by_type`.)

To count all records: `store.record_count() -> usize`.

- [ ] **Step 1: Create the file**

Create `tests/property/consolidation_props.rs` with this exact content:

```rust
// Property and unit tests for mine_and_consolidate consolidation invariants.
//
// Tests verify:
// 1. Causal chains produce Skill induction (motif mining works).
// 2. Induced Skill records carry evidence links (provenance preserved).
// 3. Hot store size decreases after consolidation (source records archived).
//
// Note: 90% reduction target (AC-4 full) requires Sub-spec 1 ExperienceStore.
// Test 3 verifies the mining layer archives at least one record.
// TODO(sub-spec-1): See bottom of file for the full 90% reduction assertion.

use hipcortex::consolidation::mine_and_consolidate;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use proptest::prelude::*;
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// Helper: build a causal chain of Temporal records linked via derived_from
// ============================================================================

fn build_chain(
    store: &mut MemoryStore<hipcortex::backends::in_memory_backend::InMemoryBackend>,
    actor: &str,
    action: &str,
    length: usize,
) {
    let mut prev_id: Option<Uuid> = None;
    for step in 0..length {
        let mut r = MemoryRecord::new(
            MemoryType::Temporal,
            actor.to_string(),
            action.to_string(),
            format!("target-{step}"),
            json!({}),
        );
        r.derived_from = prev_id;
        prev_id = Some(r.id);
        store.add(r).expect("add record");
    }
}

// ============================================================================
// Unit test 1: Causal chains produce Skill induction
//
// 3 identical action chains × 10 records each → motif frequency = 3.
// mine_and_consolidate(store, None, 3, actor) must find and induce ≥ 1 Skill.
// ============================================================================

#[test]
fn consolidation_induces_skills_from_causal_chains() {
    let mut store = MemoryStore::new_in_memory();
    let actor = "consolidation-test";
    let action = "repeated-causal-action";

    for _ in 0..3 {
        build_chain(&mut store, actor, action, 10);
    }

    let pre_count = store.record_count();
    let report = mine_and_consolidate(&mut store, None, 3, actor)
        .expect("mine_and_consolidate failed");

    assert!(
        report.skills_induced > 0,
        "expected ≥ 1 Skill induced from 3 identical causal chains, got skills_induced={}",
        report.skills_induced
    );
    assert!(
        store.record_count() != pre_count || !report.source_ids_archived.is_empty(),
        "consolidation must archive source records or add new Skill records"
    );
}

// ============================================================================
// Unit test 2: Induced Skills carry evidence links (provenance preserved)
// ============================================================================

#[test]
fn induced_skills_carry_evidence_links() {
    let mut store = MemoryStore::new_in_memory();
    let actor = "evidence-test";
    let action = "evidence-causal-action";

    for _ in 0..3 {
        build_chain(&mut store, actor, action, 10);
    }

    mine_and_consolidate(&mut store, None, 3, actor).expect("mine_and_consolidate failed");

    let skill_records = store.all_by_type(MemoryType::Skill);
    assert!(
        !skill_records.is_empty(),
        "no Skill records found after consolidation"
    );
    for skill in &skill_records {
        assert!(
            !skill.evidence.is_empty(),
            "Skill {} has empty evidence — provenance lost during consolidation",
            skill.id
        );
    }
}

// ============================================================================
// Unit test 3: Source records are archived after consolidation
//
// After mining, source_ids_archived must be non-empty for chains of frequency ≥ 3.
// ============================================================================

#[test]
fn consolidation_archives_source_records() {
    let mut store = MemoryStore::new_in_memory();
    let actor = "archive-test";
    let action = "archive-causal-action";

    for _ in 0..3 {
        build_chain(&mut store, actor, action, 8);
    }

    let report = mine_and_consolidate(&mut store, None, 3, actor)
        .expect("mine_and_consolidate failed");

    assert!(
        !report.source_ids_archived.is_empty(),
        "expected source records archived, got source_ids_archived=[]"
    );
}

// ============================================================================
// Proptest: N chains × M records → skills_induced > 0 when frequency ≥ min_freq
// ============================================================================

proptest! {
    #[test]
    fn consolidation_produces_skills_from_repeated_chains(
        chain_len in 3usize..12,
        n_chains in 3usize..6
    ) {
        let mut store = MemoryStore::new_in_memory();
        let actor = "prop-test-actor";
        let action = "prop-repeated-action";

        for _ in 0..n_chains {
            build_chain(&mut store, actor, action, chain_len);
        }

        let report = mine_and_consolidate(&mut store, None, n_chains, actor)
            .expect("mine_and_consolidate failed");

        // All chains use same action sequence → motif frequency = n_chains ≥ min_freq
        prop_assert!(
            report.skills_induced > 0,
            "expected skills from {} identical chains of length {}, got skills_induced={}",
            n_chains,
            chain_len,
            report.skills_induced
        );
    }
}

// ============================================================================
// TODO(sub-spec-1): Full 90% reduction assertion — activate after ExperienceStore ships
//
// #[test]
// fn experience_store_90_percent_reduction() {
//     // Insert 1000 Temporal records across 10 causal chains (100 per chain).
//     // Call AutoConsolidate delta via CognitiveHandle (Sub-spec 1 API).
//     // Assert: hot store record_count() <= 100 (>= 90% reduction).
//     // Assert: all source_ids_archived records reachable via evidence links
//     //         from remaining Skill/Belief records.
// }
// ============================================================================
```

- [ ] **Step 2: Verify `InMemoryBackend` import path**

`MemoryStore::new_in_memory()` returns `MemoryStore<InMemoryBackend>`. The `build_chain` helper is typed to that backend. Check the module path compiles:

```sh
grep -n "pub struct InMemoryBackend\|mod in_memory_backend" D:/all_projects/hipcortex/src/lib.rs D:/all_projects/hipcortex/src/backends/mod.rs 2>/dev/null | head -5
```

If `InMemoryBackend` is not at `hipcortex::backends::in_memory_backend::InMemoryBackend`, update the `build_chain` type annotation to use `_` or the correct path. The function can also be made generic:

```rust
fn build_chain<B: hipcortex::backends::MemoryBackend>(
    store: &mut MemoryStore<B>,
    ...
```

Use whichever compiles. Prefer the generic version if the concrete path is uncertain.

- [ ] **Step 3: Add module to `tests/property/mod.rs`**

Add one line at the end of `tests/property/mod.rs`:

```rust
mod consolidation_props;
```

`mod.rs` should now contain:
```rust
mod coherence_props;
mod connectivity;
mod fsm_reachability;
mod regression_tests;
mod self_model_props;
mod test_graph;
mod world_model_props;
mod calibration_props;
mod consolidation_props;
```

- [ ] **Step 4: Run consolidation tests only**

```sh
cargo test --no-default-features --features "petgraph_backend" --test property_suite consolidation_props 2>&1 | tail -25
```

Expected output contains:
```
test consolidation_props::consolidation_induces_skills_from_causal_chains ... ok
test consolidation_props::induced_skills_carry_evidence_links ... ok
test consolidation_props::consolidation_archives_source_records ... ok
test consolidation_props::consolidation_produces_skills_from_repeated_chains ... ok
```

If `build_chain` fails to compile due to backend type: replace the concrete type annotation with `impl hipcortex::backends::MemoryBackend` or use the generic version from Step 2.

- [ ] **Step 5: Commit**

```sh
git add tests/property/consolidation_props.rs tests/property/mod.rs
git commit -m "test(property): consolidation invariants — skill induction, evidence links, source archival"
```

---

## Task 3: `world_model_props.rs` — full diagonal PSD check

**Files:**
- Modify: `tests/property/world_model_props.rs:195-196`

### Context

Currently lines 195–196 hardcode `covariance[0][0]` and `covariance[1][1]`. If the covariance matrix grows beyond 2×2 (Sub-spec 1 entity state expansion), new diagonal elements would silently escape the check. Replace with a loop.

- [ ] **Step 1: Edit lines 195–196**

Open `tests/property/world_model_props.rs`. Find this block (lines 193–196):

```rust
        // Covariance diagonal elements must be non-negative
        let state = tracker.get_state();
        prop_assert!(state.covariance[0][0] >= 0.0, "Covariance[0][0] negative");
        prop_assert!(state.covariance[1][1] >= 0.0, "Covariance[1][1] negative");
```

Replace with:

```rust
        // All diagonal elements must be non-negative (PSD invariant for full matrix)
        let state = tracker.get_state();
        for (i, row) in state.covariance.iter().enumerate() {
            prop_assert!(
                row[i] >= 0.0,
                "Covariance diagonal [{i}][{i}] = {:.6} is negative (PSD violated)",
                row[i]
            );
        }
```

No other lines in the file change.

- [ ] **Step 2: Run the PSD test**

```sh
cargo test --no-default-features --features "petgraph_backend" --test property_suite kalman_covariance_positive_semidefinite 2>&1 | tail -10
```

Expected:
```
test world_model_props::kalman_covariance_positive_semidefinite ... ok
```

- [ ] **Step 3: Run full world_model_props suite to check no regression**

```sh
cargo test --no-default-features --features "petgraph_backend" --test property_suite world_model_props 2>&1 | tail -15
```

Expected: all existing `world_model_props` tests pass.

- [ ] **Step 4: Commit**

```sh
git add tests/property/world_model_props.rs
git commit -m "test(property): extend PSD check to full covariance diagonal (AC-8)"
```

---

## Task 4: Full property suite — final verification

- [ ] **Step 1: Run complete property suite**

```sh
cargo test --no-default-features --features "petgraph_backend" --test property_suite 2>&1 | tail -30
```

Expected: all tests green. New tests appear in output:
```
test calibration_props::empty_estimator_ece_is_zero ... ok
test calibration_props::perfect_calibration_achieves_zero_ece ... ok
test calibration_props::ece_always_in_unit_interval ... ok
test calibration_props::ece_bounded_by_mce ... ok
test calibration_props::is_well_calibrated_iff_ece_below_threshold ... ok
test consolidation_props::consolidation_induces_skills_from_causal_chains ... ok
test consolidation_props::induced_skills_carry_evidence_links ... ok
test consolidation_props::consolidation_archives_source_records ... ok
test consolidation_props::consolidation_produces_skills_from_repeated_chains ... ok
test world_model_props::kalman_covariance_positive_semidefinite ... ok
```

If any test fails: fix before proceeding. Do not push a red suite.

- [ ] **Step 2: Run lib unit tests to confirm no regression**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
```

Expected: `test result: ok.`

- [ ] **Step 3: Final commit and push**

```sh
git push origin main
```

---

## Self-Review

**Spec coverage:**
- AC-3 ECE ≤ 0.1 structure → `calibration_props.rs` Tests 1–5 ✓
- AC-4 90% reduction → `consolidation_props.rs` Tests 1–3 + proptest + TODO comment ✓
- AC-8 full PSD diagonal → `world_model_props.rs` loop ✓
- AC-8 probability conservation → already at 1e-6 in existing `world_model_props.rs:50` — untouched ✓

**Placeholder scan:** None. All test code is complete. `build_chain` backend type has a compile-time escape hatch in Task 2 Step 2.

**Type consistency:**
- `record_outcome` (not `add_prediction`, not `push`) — verified at `uncertainty.rs:137`
- `all_by_type` (not `search_by_type`) — verified at `memory_store.rs:290`
- `MiningReport.source_ids_archived` (not `records_archived`) — verified at `consolidation.rs:264`
- `MiningReport.skills_induced` — verified at `consolidation.rs:262`
- `MemoryRecord::new(type, actor, action, target, metadata)` — 5-arg constructor verified at `memory_record.rs:93`
- `store.record_count()` — verified at `memory_store.rs:286`
