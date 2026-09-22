# Reconstruction Engine Foundation — Design Spec

**Date:** 2026-09-22  
**Target version:** v3.16.0  
**Status:** Approved for implementation

---

## Goal

Close 4 gaps turning HipCortex from "cognitive state substrate" into "dynamic reconstruction engine":

1. **Gap 1** — Anomaly → axiomatic write-back: surprising experiences must elevate to durable reusable Laws, not just revise Beliefs
2. **Gap 2** — Policies as first-class citizens: `S_{t+1} = f(S_t, Policy)` inside DigitalTwin, not just goal/react cycles
3. **Gap 3** — MDL sparsity pressure: Laws GC'd by information gain / complexity, not just reference counting
4. **Gap 4** — Laws as dominant reasoning primitive: structural equations, not probabilistic rules, drive long-horizon rollouts

---

## Architecture

### New MemoryType variants

Add two variants to `MemoryType` in `src/memory_record.rs`:

```rust
Law,     // structural equation elevated from anomaly evidence
Policy,  // reactive rule attached to entity driving state evolution
```

Impact on existing code:
- `parse_record_type_alias` in `web_server.rs` — add "law"/"policy" string arms
- `grpc_server.rs` string match — add "Law"/"Policy" arms
- `ValueEnum` derive on `MemoryType` — clap auto-derives CLI aliases (no change needed)
- No exhaustive match arms exist in the codebase — addition is backward-safe

### New payload types (in `src/payloads.rs`)

**LawPayload:**
```rust
pub struct LawPayload {
    pub equation: String,                   // "effect_var ~ f(cause_vars)"
    pub causal_variables: Vec<String>,      // input variable names
    pub effect_variable: String,            // output variable name
    pub evidence_ids: Vec<Uuid>,            // surprising Intent IDs that triggered extraction
    pub mdl_score: f64,                     // evidence_count / (eq_len/20 + 1)
    pub domain: Option<String>,             // None = domain-independent
    pub holds_under_intervention: bool,     // do-calculus applicable
}
```

**PolicyPayload:**
```rust
pub struct PolicyPayload {
    pub entity_id: Uuid,           // entity this policy governs
    pub trigger_condition: String, // e.g. "state.velocity > 2.0"
    pub action_fn: String,         // e.g. "apply_brake(0.5)"
    pub priority: u32,             // higher wins on conflict
    pub active: bool,
    pub law_id: Option<Uuid>,      // Law this policy was derived from (provenance)
}
```

### New modules

**`src/law_extractor.rs`** — `LawExtractor`

Triggered from `loop_engine.rs` after `score_success_factors_from_intents()`. Clusters surprising Intent records by shared `target_entity`. If cluster ≥ MIN_SURPRISING_FOR_LAW (3), attempts pattern extraction:

1. Collect `target_entity`, `action`, `content_excerpt` from surprising Intents
2. Identify causal variables = distinct `action` tokens across cluster
3. Form equation = `"{effect} ~ {most_common_action}({top_causal_vars})"`
4. Compute `mdl_score = evidence_count / (equation.len() as f64 / 20.0 + 1.0)`
5. Check idempotency: abort if Law with same equation already exists
6. Call `route_uncertainty()` if < 2 causal variables identified (ambiguous pattern) — T0 self-prompt first, T3 ask user only if irresolvable
7. Write `MemoryType::Law` via store.add

**`src/policy_registry.rs`** — `PolicyRegistry`

In-memory store of active Policies. Methods:
- `register(record: MemoryRecord) -> Result<(), String>` — validates PayloadPayload, calls `route_uncertainty()` if trigger_condition is empty
- `active_for_entity(entity_id: Uuid) -> Vec<PolicyRecord>` — returns matching Policies sorted by priority desc
- `deactivate(policy_id: Uuid) -> bool`

### Modified modules

**`src/digital_twin.rs`** — add `step_with_policies()`

```rust
pub fn step_with_policies(
    &mut self,
    action: &str,
    registry: &PolicyRegistry,
    entity_id: Uuid,
) -> Result<Vec<f64>, CognitiveError>
```

Before applying RK4 dynamics: checks `registry.active_for_entity(entity_id)`, evaluates trigger conditions against current state, applies highest-priority matching Policy's `action_fn` (overrides or modulates the `action` arg).

**`src/emergence.rs`** — add Law extraction alongside belief clustering

After the existing `detect()` logic synthesises Beliefs, call:
```rust
let laws = LawExtractor::attempt_extract(store, actor);
// returns Vec<Uuid> of newly written Law IDs
```

**`src/cognitive_gc.rs`** — add MDL-aware GC for Laws

```rust
pub const MDL_KEEP_THRESHOLD: f64 = 0.5;

pub fn gc_action_for_law(&self, record_id: Uuid, mdl_score: f64) -> GcAction {
    if mdl_score >= MDL_KEEP_THRESHOLD {
        return GcAction::Keep;
    }
    // Below threshold: fall through to reference check
    match self.references.get(&record_id) {
        Some(refs) if !refs.is_empty() => GcAction::Archive,
        _ => GcAction::Delete,
    }
}
```

This adds sparsity pressure: Laws below MDL threshold are removed even if referenced by low-value goals.

---

## Self-Prompting Integration (route_uncertainty throughout)

Per the lifecycle mandate: `route_uncertainty()` called at each decision gate. Self-prompt resolves first (T0–T2); only escalates to T3 (ask user) if irresolvable.

| Gate | Uncertainty condition | Self-prompt (T0) | Escalate (T3) |
|------|----------------------|------------------|---------------|
| LawExtractor | < 2 causal variables identified | Re-scan entity evidence for action tokens | Ask user: "What causal variable drives {effect_variable}?" |
| PolicyRegistry.activate | trigger_condition is empty | Default condition: `"state_changed"` | Ask: "What condition should trigger this policy?" |
| GC_for_law | mdl_score between 0.3 and 0.7 (borderline) | Re-compute from fresh evidence scan | No user escalation — keep borderline Laws |

The `route_uncertainty()` call signature (from `agent_guidance.rs`):
```rust
route_uncertainty(
    uncertain: bool,          // true = ambiguous
    tiers_spent: u32,         // how many T0–T2 rungs already tried
    incomplete: bool,         // true = ladder not yet exhausted
    cost_of_wrong_execution: f64,
) -> ClarifyRoute
```

---

## Acceptance Criteria (ReAct loop exit conditions)

| ID | Criterion | Verification command |
|----|-----------|---------------------|
| AC-L1 | `MemoryType::Law` serializes/deserializes correctly (round-trip) | `cargo test --lib law_payload` |
| AC-L2 | `LawExtractor::attempt_extract()` returns `Some` with ≥3 surprising Intents for same entity | `cargo test --lib law_extractor` |
| AC-L3 | Law extraction idempotent — same equation not written twice | `cargo test --lib law_extractor_idempotency` |
| AC-L4 | `route_uncertainty` called when < 2 causal variables | `cargo test --lib law_extractor_route_uncertainty` |
| AC-P1 | `PolicyPayload` round-trips through serde | `cargo test --lib policy_payload` |
| AC-P2 | `PolicyRegistry::active_for_entity()` returns policies sorted by priority | `cargo test --lib policy_registry` |
| AC-P3 | `DigitalTwin::step_with_policies()` applies highest-priority matching policy | `cargo test --lib digital_twin_policies` |
| AC-S1 | `gc_action_for_law(id, 0.1)` returns Delete for orphaned law | `cargo test --lib gc_law_mdl` |
| AC-S2 | `gc_action_for_law(id, 0.9)` returns Keep regardless of references | `cargo test --lib gc_law_mdl` |
| AC-BUILD | Full lib suite green | `cargo test --no-default-features --features petgraph_backend --lib` |

ReAct exit: all 10 ACs green OR max 3 iterations × task and root cause documented in Reflexion record.

---

## Files Touched

| File | Change |
|------|--------|
| `src/memory_record.rs` | Add `Law`, `Policy` to `MemoryType` enum |
| `src/payloads.rs` | Add `LawPayload`, `PolicyPayload` structs |
| `src/law_extractor.rs` | **Create** — LawExtractor with route_uncertainty hooks |
| `src/policy_registry.rs` | **Create** — PolicyRegistry |
| `src/emergence.rs` | Call LawExtractor::attempt_extract after belief detect |
| `src/digital_twin.rs` | Add step_with_policies() |
| `src/cognitive_gc.rs` | Add gc_action_for_law() + MDL_KEEP_THRESHOLD |
| `src/web_server.rs` | parse_record_type_alias + REST endpoints |
| `src/lib.rs` | Register law_extractor, policy_registry modules |
| `tests/unit/law_extractor_tests.rs` | **Create** — unit tests AC-L1..L4 |
| `tests/unit/policy_registry_tests.rs` | **Create** — unit tests AC-P1..P3 |
| `tests/unit/cognitive_gc_law_tests.rs` | **Create** — unit tests AC-S1..S2 |
| `tests/unit_suite.rs` | Register 3 new test modules |

---

## Constraints

- No LLM calls in Law extraction (structural/syntactic only) — keeps extraction O(n) and testable
- Laws write via `store.add()` (not raw write) — SafetyGuardrail respected
- `MemoryType::Law` and `Policy` backward-safe — existing JSONL stores parse as-is
- `step_with_policies()` is a NEW method — `step()` and `step_with_wm()` unchanged
