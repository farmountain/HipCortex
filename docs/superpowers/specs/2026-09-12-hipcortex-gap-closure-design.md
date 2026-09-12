# HipCortex Gap Closure + Clarify-Coherence — Design

**Date:** 2026-09-12
**Status:** Draft — awaiting user review
**Scope:** H1–H6, H8, H9 (HipCortex defect closure), H10 (directional SCM), and the cross-cutting Clarify Protocol.
**Supersedes:** nothing. **Complements:** `2026-08-22-v100-pure-substrate-scm-design.md` (governs Phases 0–4, still in force).

---

## 0. Why this design exists

The authoritative work list is `Kakeya_Abstraction_Reasoning_Action_Model/karm_v0.1/openspec/specs/081-seam-segregation/HIPCORTEX_GAPS_HANDOVER.md`
(dated 2026-09-12, source- and live-verified). `HipCortex_Development_Gaps_Context_Roadmap.md` is declared **stale** by that
same spec and is treated here as historical input only.

The seam between the two repos is **already decided** by `081-seam-segregation/ownership.yaml` — the split is
**by mathematical object, not by role**:

| Object | Owner | Rationale (verbatim from `ownership.yaml`) |
|---|---|---|
| Causal reasoning over the **discrete entity transition graph** | HipCortex (`substrate.scm_graph`, `src/modules/world_model_enhanced/causal.rs`) | `counterpart: null` |
| Geometry over the **continuous latent sphere $S^{d-1}$** | Kakeya/KARM (`kakeya.cover`, `kakeya.flow`, `kakeya.sweep`, `geometry.causal_field`) | `counterpart: null` |
| Consolidation | HipCortex (`substrate.consolidation`, `src/consolidation.rs`) | `karm/memory/consolidator.py` is a **duplicate and must be removed (K5)** |

`SUBSTRATE_CONTRACT.md §10` states the boundary as non-negotiable:

> Multi-horizon goal decomposition, policy optimisation, full DTCF, and agent orchestration belong **outside** HipCortex — in KARAM. HipCortex only maintains and exposes high-quality $S$.

**Therefore:** this design closes HipCortex defects and the clarify mechanism only. It touches **no** KARM-owned capability and adds **no** Kakeya analogue (H7 = do not build).

### 0.1 Method note carried forward

`HIPCORTEX_GAPS_HANDOVER.md §2` records that its own first draft was wrong twice, and concludes:
**"probe the live engine, never the handover doc."** Every claim below was independently re-verified against v3.10.0
source *and* the live server on 2026-09-12. Where this design **disagrees** with the handover it says so explicitly.

`§2.1` also records that three `POST /memory/consolidate` calls believed to be dry runs **destroyed records**
(one call deleted 36 live records) because `dry_run` travelled in the body and was silently ignored. **No destructive
endpoint was called during this verification.** Handlers were read first.

---

## 1. Verified defect inventory

### 1.1 Confirmed as filed

| ID | Defect | Sev | Evidence (verified this session) |
|---|---|---|---|
| **H5** | `/memory/consolidate` destroys data | CRITICAL | `web_server.rs:3306-3395` read in full. All 8 filed defects present + 2 new (§1.3) |
| **H9** | `DELETE /memory/forget/:actor` un-auditable | MEDIUM | `web_server.rs:5238` read. `records_deleted = ids.len()` — **ids counted then discarded** |
| **H4** | `/v1/cognitive/report` ignores `actor` | HIGH | `cognitive_report.rs:112,166` — `store.all_by_type(MemoryType::Belief)` and `MemoryType::Decision` with **no actor filter** |
| **H2** | MCP bridge dead | HIGH | `sdk/mcp/server.py:1628,1639,1643,1652,1659,1668` call `_req(...)`; **no `def _req` exists**. *(Count corrected: this was a sample, not the population — **17** dispatched handlers reference `_req`, each with one call site. Measured against the copy that still carries the defect, `sdk/python/build/lib/hipcortex/install/mcp_server.py`, by `test_mcp_tool_surface.py`.)* |
| **H8** | Capability registry is a 4th hand-maintained list | LOW-MED | `src/bin/webserver.rs:59-73` — 15 names hardcoded at bootstrap |
| **H3** | Unknown `record_type` silently coerced | MEDIUM | `web_server.rs:4863-4880` — `_ => MemoryType::Temporal` |
| **H6** | `learned_beliefs` / `emergent_abstractions` are arrays | LOW | `cognitive_report.rs` returns `Vec<BeliefSummary>` |

### 1.2 Correction to the handover — H1 is larger, and its root cause is structural

The handover files H1 as "84 of 123 live routes absent from `/openapi.json`" (a documentation gap). **That is not the
main problem.** Verified:

- `src/web_server.rs` declares **175** `.route(...)` calls.
- It contains **three** independent `Router::new()` blocks: line **936** (inside `build_app`, line 562), line **1946**
  (inside `run_with_store`, line 1925), line **4192** (inside `run_with_both_stores`, line 3538).
- `src/bin/webserver.rs:157` serves `web_server::run_with_state(addr, state)` → `build_app` → the **936** router.
- Live `GET /openapi.json` documents **39** paths.
- `run_with_both_stores` is **referenced nowhere** — it is dead code — yet its router declares routes the live router
  does not.

**Consequence — these documented, advertised endpoints return 404 in production:**

| Route | Declared only at | Live status |
|---|---|---|
| `POST /goal/:id/clarify` | 4266 | **404 (probed)** |
| `/goal/:id/verify` | 4265 | not served |
| `POST /memory/diff` | 4269 | not served |
| `/v1/workspace/:id/renew` | 4267 | not served |
| `/v1/actions/authorized-wm` | 4268 | not served |
| `/v1/loop/omega`, `/v1/loop/subscribe`, `/v1/loop/status/:handle`, `/v1/loop/stop/:handle` | 4274-4277 | **404 (probed for `/v1/loop/omega`)** |

`POST /goal/:id/clarify` is the **user-facing surface of the clarify mechanism** and is dead. `/memory/diff` is
advertised in `CLAUDE.md` as a v0.6.0 endpoint and is dead. This makes H1 part data-loss-adjacent (features silently
absent) and part architectural (three divergent routers).

So H1 decomposes into three distinct defects:

- **H1a — dead router.** 57 route declarations in an unreachable `Router`. **Breakage.**
- **H1b — live-route drift.** ~75 of 114 live routes are absent from `/openapi.json`. **Documentation.**
- **H1c — no parity guard.** Nothing prevents the three lists (live router, OpenAPI spec, `bin/webserver.rs`
  capabilities) from drifting again. **Root cause of H1b and H8 both.**

### 1.3 H5 — all eight filed defects confirmed, plus two found here

`handle_consolidate` (`web_server.rs:3306`):

1. **Lexical only** — `split_whitespace()` token sets over `.target`; ignores `actor`, `action`, `metadata`, embeddings.
2. **Ignores `confidence`** — never referenced.
3. **Never merges** — only `ms.delete_by_id(uuid)`. No content, provenance, or evidence union.
4. **Ignores body `dry_run`** — the extractor is `Query(params): Query<ConsolidateParams>` with **no `Json` body
   extractor at all**, yet `ConsolidateParams` (`web_server.rs:365`) declares `dry_run: Option<bool>` as a
   body-shaped field. A caller passing `{"dry_run": true}` is silently ignored. `dry_run` also **defaults to `false`**
   → **destructive by default**.
5. **Cross-actor destruction** — `params.actor.as_ref().map_or(true, |a| &r.actor == a)`. Omitting `actor` scans and
   deletes **across all actors**.
6. **Pair set is not a partition** — `drop_set` is consulted only for `candidates[j]`; `keep` is never checked, so a
   record already marked for dropping can later become a `keep`, and one record can appear in many pairs.
7. **Audit entry absent** — no Merkle / `audit.log` write in the handler.
8. **Loss unrecoverable from archive** — hard `delete_by_id`, **not** `ArchiveStore::archive()`, violating the repo's
   Hot/Cold rule; `derived_from` / `evidence` provenance is not repaired, orphaning children.

Additionally found:

9. **Similarity is not Jaccard** — `intersection / max(|i|,|j|)` is a containment ratio, not `intersection/union`.
   Short targets trivially clear `0.80`.
10. **`expires_at` unit coupling** — compared against `Utc::now().timestamp()`; correct today, but any writer storing
    ms would silently exclude records. Flagged as a guard to add, not a present defect.

Backed by `HIPCORTEX_GAPS_HANDOVER.md §2.2`: `Consolidate` records erased ids (592/653 transactions, 9429 ids) so it is
**attributable**; `ForgetActor` records **zero** ids (19/19 transactions) so it is **not reconstructable**. Only **623**
of the 9429 ids appear in `memory-archive.jsonl`. *"The archive is an overlapping history, not a journal."*

### 1.4 H2 — root cause found, and it is trivial

`sdk/mcp/server.py` uses **two** HTTP helper conventions:

- `_post(path, body)` — **exists**; used by the older handlers (`handle_consolidate_memory`, `handle_twin_create`, …).
- `_req(method, path, payload)` — **does not exist**; referenced by **17** dispatched handlers (the SCM/MGV/report group and more). The "6" this section originally reported was the six sites visible in one reading, not the population.

So the fix is to define one small `_req`, not to rewrite the seventeen call sites — a **surgical** change (Karpathy §3).

### 1.5 H10 — directional `StructuralEquation` (listed by no document)

Roadmap §5 requires intervening on **directional** variables. The v1.0 spec §7 **explicitly deferred** it. Verified in
`src/modules/world_model_enhanced/causal.rs`:

```rust
pub trait StructuralEquation: Send + Sync + std::fmt::Debug {
    fn evaluate(&self, parents: &[f64], u: f64) -> f64;      // scalar parents, scalar noise
    fn invert_for_u(&self, parents: &[f64], observed: f64) -> f64;
}
pub struct LinearSE { pub weights: Vec<f64> }                 // scalar weights

pub struct CausalNode {
    pub id: String,
    pub properties: HashMap<String, String>,
    pub embedding: Option<[f32; 128]>,                        // scaffolding present, unused
    pub equation: Option<Arc<dyn StructuralEquation>>,
    pub noise_var: f64,
}

pub struct InterventionQuery {
    pub outcome: String,
    pub intervention_var: String,
    pub intervention_value: f64,                              // <-- scalar; no vector path
    pub conditioned_on: HashMap<String, f64>,
    pub intervention_label: Option<String>,
}
```

**Nuance, stated precisely:** `u` is a separate `f64` parameter, not part of `parents`. So "no holdable $U_i$" is only
*partially* true — $u$ is recoverable by abduction via `invert_for_u`, but it cannot be a **vector** and cannot be
persisted as a first-class node. `CausalNode.embedding: Option<[f32;128]>` and `noise_var: f64` already exist as
partial scaffolding. **H10 is the one gap that actually blocks the Kakeya seam**, and it is HipCortex-owned per
`ownership.yaml::substrate.scm_graph`.

### 1.6 The Clarify Protocol is already built and wired — but its budget is not lifetime-bounded

`src/clarify_engine.rs` already implements the user's requirement: self-prompt first, ask second, bounded
(`MAX_CLARIFY_ROUNDS = 3`), with the CoT header stating *"Exit is guaranteed (no indefinite loop)."* It is wired into
`loop_engine.rs:584`, `substrate_daemon.rs:224`, and a `GoalNotClarified` hard gate at `cognitive_state.rs:37`.

Residual, agent-found gaps:

| # | Gap | Verified evidence |
|---|---|---|
| C1 | **The 3 rounds are decorative** — each round re-runs the *identical* belief search; `round` appears only in a log note. No escalating strategy, so the bound buys nothing. | `clarify_engine.rs` `run()` loop |
| C2 | **Two parallel clarify systems.** `src/agent_guidance.rs` (15,705 bytes: `clarify_goal`, `plan_validation`, `check_progress`, `should_exit`) is referenced **only** by `lib.rs:8` and 4 orphan routes `/agent/{clarify-goal,plan-validation,check-progress,should-exit}` (`web_server.rs:1016-1019`; handlers 6944/6950/6959/6974). A duplication smell by 081's own one-owner law. | grep |
| C3 | **Budget is per-invocation, not per-goal.** A ReAct loop can clarify 3× per iteration × N iterations ⇒ unbounded in aggregate. **This is the real unbounded-loop hazard**, not the round counter. | `clarify_engine.rs` |
| C4 | **Not applied to AC definition / validation & test planning** as a first-class flow — `plan_validation` exists but is orphaned. | `agent_guidance.rs` |
| C5 | **Invisible in OpenAPI**, and its REST trigger (`/goal/:id/clarify`) is **404** (§1.2). | live probe |

---

## 2. Decisions

| # | Decision | Rationale |
|---|---|---|
| **D1** | Delete the dead router (`run_with_both_stores`, line 3538) **after** porting its 9 dead-only routes into `build_app`. | H1a. Restores `/goal/:id/clarify`, `/memory/diff`, `/goal/:id/verify`, `/v1/loop/*`. |
| **D2** | Extract **one** `register_routes(router, state) -> Router`; `build_app` calls it; the other routers are removed. | Single source of truth for the routing surface. |
| **D3** | Add a **route-parity test** asserting the live router's path set == `/openapi.json` path set == the handler dispatch set. | H1c. Drift becomes a test failure, not a silent regression. |
| **D4** | Generate the OpenAPI path list from the router rather than hand-maintaining it. | H1b. Removes the third hand-maintained list. |
| **D5** | `/memory/consolidate`: **require `actor`**, accept `dry_run` from **body or query**, **default to preview**, and require an explicit `confirm=true` to mutate. | H5 defects 4, 5. Safe by default; the destructive default is the single highest-value fix. |
| **D6** | `/memory/consolidate`: partition candidates with **union-find into disjoint clusters**; elect one survivor per cluster by `(confidence desc, timestamp desc, provenance count desc)`; **merge** `evidence`/`derived_from`/metadata into the survivor; **`ArchiveStore::archive()`** the rest. | H5 defects 3, 6, 8. A partition is the only correct grouping; archiving preserves the Hot/Cold contract. |
| **D7** | `/memory/consolidate`: true **Jaccard**, over `target + action + sorted metadata keys`, gated to **same `MemoryType`** and never crossing `pinned` with non-pinned. Emit a `Reflexion{consolidated}` audit record carrying the full id list. | H5 defects 1, 2, 7, 9. |
| **D8** | `/memory/forget/:actor`: return **`deleted_ids`** and write an audit record **before** deleting. Keep the hard delete. | H9. GDPR erasure *must* erase content; the audit may lawfully record **ids only** (provenance metadata, not content). |
| **D9** | `/v1/cognitive/report`: scope beliefs, decisions, and `predicted_only_ids` by `actor`. Omitting `actor` returns `actor_scoped: false` rather than silently returning everyone's data. | H4. The handover's note — *"Worse than empty: it looks like it works"* — is the reason this must be explicit. |
| **D10** | Define `_req(method, path, payload=None)` in `sdk/mcp/server.py` in terms of the existing transport, failing **silent to `{"error": …}`**. Do not touch the **17** call sites. | H2. Minimal diff. |
| **D11** | `parse_record_type_alias` returns `Result`; unknown values produce **HTTP 400** listing the accepted aliases. | H3. Never silently coerce. |
| **D12** | H10: keep `evaluate(&self, parents: &[f64], u: f64)` **unchanged** and additive. | Roadmap §5 verbatim: *"Design the StructuralEquation interface so that parents can be vectors without breaking existing scalar code paths."* |

### 2.1 H10 interface design (D12 detail)

Additive only — every existing impl keeps compiling:

```rust
/// How many flat slots this equation consumes, and whether they include a direction block.
pub enum SeShape { Scalar, Directional { dim: usize } }

pub trait StructuralEquation {
    fn evaluate(&self, parents: &[f64], u: f64) -> f64;              // UNCHANGED
    fn invert_for_u(&self, parents: &[f64], observed: f64) -> f64;   // UNCHANGED
    fn shape(&self) -> SeShape { SeShape::Scalar }                   // NEW, defaulted
    fn evaluate_directional(&self, parents: &[f64], dir: &[f64], u: f64) -> f64 {
        self.evaluate(parents, u)                                    // NEW, defaulted = scalar fallback
    }
}

/// Directional linear SE: outcome = <w_parents, parents> + <w_dir, dir> + u
pub struct DirectionalSE { pub weights: Vec<f64>, pub dir_weights: Vec<f64>, pub dim: usize }

/// Vector intervention. `InterventionValue::Scalar` preserves the current wire format.
pub enum InterventionValue { Scalar(f64), Direction(f64 /*norm*/, Vec<f64>) }
```

`InterventionQuery` gains `intervention_vector: Option<Vec<f64>>` (defaulting to `None`) so the JSON contract is
backward compatible. `CausalNode.embedding: Option<[f32;128]>` becomes the natural user of the direction block, and
`noise_var` is promoted to a per-component vector for directional nodes only.

**This is the enabling change for the Kakeya seam.** KARM sweeps directions on $S^{d-1}$; without a directional
`StructuralEquation` and a vector intervention, a swept direction cannot be *held fixed* while intervening, so
`SUBSTRATE_CONTRACT.md §0.3`'s requirement cannot be met even in principle.

---

## 3. The Clarify Protocol (cross-cutting requirement)

The user's requirement, restated as a testable contract:

> Resolve ambiguity by **self-prompting first**. Ask the user **only** when self-prompting cannot resolve it, and only
> for **critical** questions. Apply this to goal definition, acceptance criteria, validation/test planning,
> unknowns/uncertainty, planning, and the ReAct loop. **Guarantee exit** — no indefinite self-prompting or asking.

### 3.1 Design: a strictly-descending information ladder

`MAX_CLARIFY_ROUNDS = 3` is replaced by **three tiers, each attempted at most once per goal**, each strictly more
informative (and more expensive) than the last. Because the ladder is consumed monotonically, termination is provable
**without** relying on a counter.

| Tier | Strategy | Cost | On success |
|---|---|---|---|
| **T0 — Environment restatement** *(exists)* | `restate_if_env_changed`: scan last 20 `Temporal` records for failure-keyword × factor-keyword overlap; rename blocked factors to `{name}_when_available`; emit one `Temporal{probe_required}` per blocked factor. | 0 external | `ClarifiedBySubstrate{source: "environment"}` |
| **T1 — Prior-art restatement** *(new)* | Search `Reflexion` + `Skill` for a **Succeeded** goal whose `success_factors` overlap ≥ 0.5 with this goal. Adopt the delta as a *proposed* AC. | 0 external | `ClarifiedBySubstrate{source: "prior_art", evidence: [goal_ids]}` |
| **T2 — Causal attribution restatement** *(new; needs H10)* | Run `wm.credit_assign_trajectory(&trajectory)`. If `broken_equation.is_some()` ∧ `single_intervention_sufficient` ∧ `confidence >= 0.85` (the **existing** gate), self-resolve by emitting the existing `CognitiveDelta::RewriteStructuralEquation`. | 0 external | `ClarifiedBySubstrate{source: "causal", broken_equation, confidence}` |
| **T3 — Ask the user** *(last resort)* | Emit exactly **one** deduped `Belief{action:"clarify_needed", derived_from: goal_id}`. Its `clarifying_questions` are authored by `agent_guidance::clarify_goal`. | user attention | `NeedsUserClarification` |

### 3.2 Exit guarantees (explicit, exhaustive)

The protocol terminates because **every** path is bounded by construction:

1. **Tier monotonicity.** A tier is recorded as a `Reflexion{clarify_tier, tier}` keyed by `(goal_id, tier)` and is
   never re-entered for the same goal.
2. **Ladder exhaustion.** `MAX_CLARIFY_TIERS = 3`; after T2 fails, T3 is terminal for that goal.
3. **Lifetime budget.** `MAX_CLARIFY_CYCLES_PER_GOAL = 3` counts **invocations per goal**, not rounds per invocation.
   This closes C3, the actual unbounded hazard. On exhaustion the goal is forced to `NeedsUserClarification` and its
   status is set terminal — the ReAct loop does **not** spin.
4. **No-progress exit.** If a tier's `Reflexion` content is byte-identical to the previous attempt at that tier, the
   ladder exits immediately to T3. Progress must be monotone.
5. **Budget tie-in.** Each tier debits `SubstrateBudget`; when `clarify_spend ≥ B_0`, T3 is forced — consistent with
   `SUBSTRATE_CONTRACT.md §4`'s $K_{max} = \lfloor B_0 / c_{min} \rfloor$ almost-sure-termination argument.
6. **Ask-cost gate.** T3 is reached only if the ladder is exhausted **and** `DecisionEngine` expected utility favours
   asking: ask iff `P(unresolvable alone) × cost_of_wrong_execution > cost_of_asking`. This makes *"only critical
   questions should clarify with users"* **computable** rather than a matter of judgement. `cost_of_wrong_execution`
   derives from the goal's risk class.

`ReactEngine`'s `max_react_iterations` remains the outer bound; the clarify ladder is strictly inside it.

### 3.3 One-owner resolution (closes C2)

Per `ownership.yaml`'s law — *one owner per capability; non-owner holds no parallel truth* — the two clarify systems are
given **distinct, non-overlapping** responsibilities rather than merged:

- **`clarify_engine` = the single stateful authority.** Owns triggering, the ladder, the budget, and the terminal
  decision. Nothing else may write `Belief{clarify_needed}`.
- **`agent_guidance` = pure advisory text generation, stateless.** Owns the *phrasing* of questions, validation plans,
  and exit rationales. It is **called by** `clarify_engine`; it decides nothing.

This converts the 4 orphan `/agent/*` routes from "dead duplication" into the *content generators* of a single
authority — deduplication without deletion, and without a second truth.

### 3.4 Application to acceptance criteria, validation, and test planning

- `ClarifyTrigger` gains **`UntestableAC`**: AC present but lacking a machine-checkable metric/threshold. (`EmptyAC`
  already covers the absent case.)
- On `UntestableAC`, T1 is the primary resolver (prior art is the best source of a proven metric); if it fails and T3 is
  reached, the question asked is *"what threshold makes this AC decidable?"*
- The resolved AC is stored as a proposal on the goal, and a **validation plan** produced by
  `agent_guidance::plan_validation` is emitted alongside it.
- **`agent_guidance::check_progress` + `should_exit` become the single exit authority** consulted by `ReactEngine`,
  replacing the ad-hoc all-`success_factors`-satisfied check currently reimplemented in `loop_engine.rs`. This removes C4
  *and* the second half of C2 in one move.

### 3.5 Observability

Every ladder step writes a `Reflexion`, so the existing live `GET /goal/:id/trace` and
`cognitive_report.clarify_pending` (Q10) already show the ladder. Add `clarify_ladder: [{tier, outcome, evidence}]` to
the report so a human can see exactly why the substrate did or did not ask.

---

## 4. Work packages

Ordering follows the handover's recommendation (**stop data loss first**) with H10 and the clarify work placed to
unblock the seam and the user's cross-cutting requirement.

| WP | Content | Depends on | Acceptance criteria |
|---|---|---|---|
| **WP1 (H5)** | Consolidate: require `actor`; body+query `dry_run`; **preview default** + `confirm=true`; union-find partition; merge provenance; archive survivors' duplicates; true Jaccard; type/pinned guards; audit `Reflexion`. | — | `dry_run` in body is honoured; omitted `actor` → 400; a 3-way near-duplicate cluster yields **1 survivor + 2 archived**, never 3 deletions; every archived id appears in the audit record; no `delete_by_id` remains in the handler. |
| **WP2 (H9)** | Forget: return `deleted_ids`; audit before delete. | — | Response contains every victim id; the audit record precedes deletion; a wrong-actor call records 0 ids and destroys 0 records. |
| **WP3 (H4)** | Report actor scoping. | — | A fabricated actor returns **0** beliefs and **0** decisions; omitted `actor` sets `actor_scoped: false`. |
| **WP4 (H2)** | Define `_req`. | — | All **17** call sites work against the live server; a transport failure returns `{"error":…}` and never raises. |
| **WP5 (H1a/H1b/H1c)** | Port the 9 dead-only routes; delete the dead router; extract `register_routes`; derive OpenAPI from the router; add the parity test. | WP1–WP4 (they touch the same file) | Live router path set == OpenAPI path set == dispatch set, enforced by a test; `/goal/:id/clarify` returns non-404; `/memory/diff` returns non-404; zero unreferenced `Router::new()` blocks remain. |
| **WP6 (H3)** | `parse_record_type_alias` → `Result`; unknown ⇒ 400. | WP5 | `{"record_type":"Fact"}` → 400 naming the valid aliases; `"Semantic"` still → `Symbolic`; `"Temporal"` → 200. |
| **WP7 (H8)** | Derive the capability registry from the router + OpenAPI instead of the hardcoded list. | WP5 | Removing a route removes its capability; adding a route registers it; the 15-name literal is gone. |
| **WP8 (H6)** | `learned_beliefs` / `emergent_abstractions` → counts, with the arrays retained under explicit `*_detail` keys. | WP3 | Both counts present; existing consumers unaffected. |
| **WP9 (H10)** | Directional `StructuralEquation` + `InterventionValue` per §2.1. | — | `LinearSE` unchanged and compiling; `DirectionalSE` evaluates a direction block; a vector intervention is *held fixed* across a rollout; existing scalar SCM tests pass untouched. |
| **WP10 (Clarify)** | Ladder tiers T1/T2; lifetime budget; `UntestableAC`; `agent_guidance` folded in as content generator; `should_exit` as single exit authority; `clarify_ladder` in the report. | WP5 (route), WP9 (T2) | Each tier fires at most once; budget exhaustion forces T3 and a terminal status; a goal with testable AC never reaches T3; a goal with prior art never reaches T3; the report shows the ladder. |

---

## 5. Test plan

- **Unit** (`tests/unit_suite.rs`): union-find partition correctness; true-Jaccard boundaries (0.0/1.0/short-string);
  survivor election ordering; `parse_record_type_alias` rejection; ladder tier monotonicity and no-progress exit;
  `DirectionalSE` evaluation and abduction round-trip.
- **Integration** (`tests/integration_suite.rs`): consolidate dry-run-then-confirm leaves the store byte-identical on
  preview; forget audit precedes deletion; report scoping with two actors.
- **Property** (`tests/property_suite.rs`): **consolidate never increases the record count and never loses an id** — the
  invariant whose absence caused the original data loss. Extend `world_model_props.rs` for directional SCM.
- **Route parity**: a new test in the live-router set asserting the three path sets are equal (D3). This is the guard
  that keeps H1c closed.
- **E2E** (`tests/e2e_user_harness/suites/`): `/goal/:id/clarify` reachable; the ladder visible in
  `/goal/:id/trace`; `/memory/diff` reachable.
- **Python** (`sdk/python/tests/`): `_req` defined and fail-silent; MCP tool call succeeds against a live server.
- **Regression data**: `memory-tx.jsonl` / `memory-archive.jsonl` snapshots were taken before any probing and are used as
  restore points. No destructive endpoint is called during testing without a snapshot first.

---

## 6. Non-goals (explicit, to respect the L0 boundary)

Per `2026-08-13-hipcortex-gap-remediation-design.md`'s **L0 constraint** and `SUBSTRATE_CONTRACT.md §10`:

- ✗ Kakeya directional coverage, geodesic trajectory optimisation, action sweeping (KARM-owned).
- ✗ Multi-horizon goal decomposition, policy optimisation, full DTCF, agent orchestration (KARAM-owned).
- ✗ Distillation of planning into a continuous operator (KARM-owned).
- ✗ Removing `karm/memory/consolidator.py` — that is **K5, in the sibling repo**, outside this workspace's write scope.
- ✗ Any new parallel truth for a capability listed with a non-null `counterpart` in `ownership.yaml`.
- ✗ H7 — no Kakeya-set analogue. By design.

## 7. Assumptions (stated, per Karpathy §1)

1. `bin/webserver.rs` remains the production entry point, so `build_app`'s router is the one to keep. **If a deployment
   serves `run_with_both_stores` instead, D1/D2 invert.** Verified for the local binary only.
2. `ArchiveStore` is wired into `AppState` for the consolidate path. *(To confirm at implementation; `handle_add_memory`
   already takes an `archive_store`, so this is expected.)*
3. KARM's `HipCortexClient` retry + circuit-breaker means none of these fixes are release-blocking for KARM — this is
   quality work, per the handover's own conclusion.
4. `VERSION` (3.10.0) is the single version source of truth; new endpoints do not change it.

## 8. Risks

| Risk | Mitigation |
|---|---|
| WP5 touches the hottest file (`web_server.rs`, 7000+ lines) | Sequence last among the H-gaps; parity test lands **with** the change, not after. |
| Changing `dry_run`'s default is a breaking API change | Correct default for a destructive endpoint. Documented in `CHANGELOG.md`; `confirm=true` is explicit opt-in. |
| `RunWithBothStores` may be used by an out-of-tree caller | Grep confirms zero in-repo references; `mcp_server.rs` uses `run_with_store`, which is preserved. |
| Deleting the dead router could orphan handlers | Compiler-enforced: unused `async fn` + `dead_code` warnings; ported routes keep their handlers. |
| `MAX_CLARIFY_CYCLES_PER_GOAL` could stall a legitimate multi-goal run | Budget is **per goal**, not global; exhaustion sets a terminal status rather than blocking. |
