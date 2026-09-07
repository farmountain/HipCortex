# HipCortex Substrate Scorecard

10 verifiable questions that differentiate a **cognitive substrate** from an agent memory layer (Mem0 / Zep / Letta / basic vector store).

Each answer includes a code or test reference that can be independently verified.

> **Live truth (v3.0.0+):** `GET /substrate/scorecard?actor=<actor>` returns live cognitive state: `uncertain_count`, `invalidated_count`, `recommended_op`, `goal_target`. The static answers below are reference documentation; the JSON endpoint is authoritative. The static answers reflect *capability claims* — the live endpoint reflects *current state* for a specific actor.

---

## Q1 — Does it survive process restart without data loss?

**Yes.** Memory store persists to JSONL (default) or RocksDB. On reload, SHA-256 integrity hashes are verified per record. Merkle-chained `audit.log` detects tampering.

- Test: `tests/integration/restart_survivability_sit.rs::g1_snapshot_coherent_after_restart`
- Code: `src/memory_store.rs::new_from_file`, `src/memory_store.rs::load_from_disk`

---

## Q2 — Does it prevent hallucinated beliefs from propagating?

**Yes.** The JTMS labels (`In` / `Out` / `Unknown`) track justification chains. `BeliefExecutive::retract` propagates removals through `derived_from` links. Beliefs without grounded `evidence` stay `Unknown` and are excluded from `get_live_beliefs`.

- Code: `src/belief_executive.rs::retract`, `src/payloads.rs::JtmsLabel`
- Test: `tests/unit/cognitive_state_tests.rs`

---

## Q3 — Does it gate actions before execution?

**Yes.** `DecisionEngine` implements `ExecutionGate` and is installed by default in every `subscribe_with_config` call (G7c). Stage 5 of the cognitive loop calls `gate.evaluate("react_step")` before running ReactEngine. Rejection writes a `gate_veto` Temporal record.

- Code: `src/substrate_daemon.rs::subscribe_with_config` (line ~115), `src/execution_gate.rs`
- Test: `tests/acceptance_suite_v260.rs::ac_e2_gate_rejected_writes_veto_record`

---

## Q4 — Does it learn P(s′|s,a) from real observations?

**Yes.** `update_from_receipt` writes two Dirichlet-Multinomial transitions per receipt: a meta-probe (`entity→probe→entity_ok|failed`) and a domain observation (`entity→observe→entity:<state>`), feeding `WorldModelEnhanced.observe_transition`.

- Code: `src/wm_updater.rs::update_from_receipt`
- Test: `tests/acceptance_suite_v270.rs::ac_a1_update_from_receipt_writes_dual_transitions`

---

## Q5 — Does it assign credit to the right beliefs after a correct action?

**Yes.** `accept_receipt_impl` traverses `derived_from` and `evidence` provenance links to find beliefs causally connected to the matched temporal record, then calls `BeliefExecutive::reinforce` with a confidence boost. No substring matching.

- Code: `src/cognitive_state.rs::accept_receipt_impl` (provenance block)
- Test: `tests/acceptance_suite_v270.rs::ac_b1_belief_reinforce_traverses_provenance_links`

---

## Q6 — Does it age out stale records without manual cleanup?

**Yes.** `MemoryStore::purge_expired` removes all records whose `expires_at` Unix timestamp has passed. `CognitiveGC::gc_action` routes records to `ArchiveStore` (Cold Store) when in-degree > 0, or deletes when orphaned.

- Code: `src/memory_store.rs::purge_expired`, `src/cognitive_gc.rs::gc_action`
- Test: `tests/integration/soak_sit.rs::ac_s1_temporal_decay_purge_expired_cleans_hot_store`

---

## Q7 — Does it know when it is uncertain?

**Yes.** `GroundingGate` blocks the ReAct loop when entity coverage < τ_c=0.6 or max epistemic uncertainty > τ_e=0.5. `EntityContactRecord.grounding_status` tracks Virgin/Sketch/Mapped/Stale/Anomalous per entity. IG probe ranking targets the highest-uncertainty entity first.

- Code: `src/grounding_gate.rs`, `src/action_intent.rs::EntityContactRecord`
- Test: `tests/acceptance_suite_v250.rs`

---

## Q8 — Does it support multiple concurrent agents / actors?

**Yes.** Every `MemoryRecord` carries an `actor` field. Passive observers use `{prefix}-{role}` namespacing (e.g. `langchain-agent`, `crew-agent`, `vscode-terminal`). `MemoryStore::search_by_actor` isolates per-agent views. `SubstrateDaemon` manages independent 8-stage loops per actor.

- Code: `src/substrate_daemon.rs::subscribe`, `sdk/python/hipcortex/adapters/`
- Test: `tests/integration/conversation_memory_sit.rs`

---

## Q9 — Does it provide a tamper-evident audit trail?

**Yes.** Every write appends a SHA-256 record hash to a Merkle-chained `audit.log`. AES-GCM encrypted JSONL option available via `MemoryStore::new_encrypted`. `safety-audit` CLI command replays the chain.

- Code: `src/memory_store.rs` (audit block), `src/bin/cli.rs::safety_audit`
- CLI: `cargo run -- safety-audit`

---

## Q10 — Does it run without any external service?

**Yes.** Default feature `petgraph_backend` is a pure in-memory graph. No database, no LLM, no network required. All cognitive loop stages, JTMS, WM, and GC work offline. External LLM connectors and database backends are optional feature flags.

- Code: `Cargo.toml` `[features]` block, `src/backends/petgraph/`
- Build: `cargo build --no-default-features --features petgraph_backend`

---

## Machine-Readable Summary

```json
{
  "restart_survivability": true,
  "hallucination_prevention": true,
  "action_gating": true,
  "world_model_learning": true,
  "causal_credit_assignment": true,
  "temporal_decay": true,
  "epistemic_uncertainty": true,
  "multi_actor": true,
  "audit_trail": true,
  "zero_external_deps": true
}
```

> Full test suite: `cargo test --no-default-features --features petgraph_backend`
