# Loop-Ready Contract (v3.18.0) — Design

**Date:** 2026-09-27
**Status:** Implemented (Rust + REST + MCP + Python SDK)
**Builds on:** v3.17.0 contract surface (`CognitiveSnapshot.tx_cursor`, `TransitionView`,
`CognitiveStateProvider` / `CognitiveStateSink`, `GET /snapshot/:actor/transitions`).

## 1. Problem

An external control loop consumes HipCortex as "snapshot once, then follow the changes".
The v3.17.0 surface cannot support that loop:

| # | Defect | Evidence |
|---|--------|----------|
| D1 | `transitions_since(actor, since_tx)` ignores `since_tx` — every call returns every settled intent | `cognitive_state.rs` (`_since_tx`) |
| D2 | Settled intents are never removed from `open_intents` — unbounded growth | only `push`, no removal anywhere |
| D3 | No read model for the structural causal model (equations, weights, interventions) | — |
| D4 | `TransitionView.ok` is `status == Received`; a failed receipt (`ok: false`) reports `ok: true` | `transitions_since` |
| D5 | `snapshot()` reads `tx_cursor` *after* its content — a settle landing mid-read is in neither the snapshot nor the feed after it | `snapshot()` |
| D6 | `get_parents` / `parents_of` iterate a `HashMap`; the directional counterfactual and credit assignment bind `weights[i]` to the i-th parent positionally, so the binding is random per graph | `causal.rs` |
| D7 | `InFlight` intents are reported as settled transitions | filter is `!Open` |

## 2. Design

### 2.1 Settle stamps (fixes D1, D4, D7)

`ActionIntent` gains two server-owned fields, both `#[serde(default, skip_serializing_if = "Option::is_none")]`:

- `settled_tx: Option<u64>` — the transaction that settled the intent.
- `receipt_ok: Option<bool>` — the `ok` of the receipt that settled it.

An intent is **settled** iff `settled_tx.is_some()`. Only settles produce stamps:

| Event | Stamp source | Status | `receipt_ok` |
|-------|--------------|--------|--------------|
| Receipt (first) | the receipt's `TxKind::MemoryAdd` entry | `Received` | `Some(receipt.ok)` |
| Receipt for an already-`Received` intent | none — first receipt wins; records still written | unchanged | unchanged |
| Receipt after `Expired` (late) | new stamp (restamp) | `Received` | `Some(receipt.ok)` |
| Deadline passed (`expire_stale_intents`) | new `TxKind::IntentExpire` entry, actor = the intent's actor | `Expired` | unchanged (`None`) |

Without a TxLog, stamps come from a per-handle `settle_seq` counter starting at 1, and receipt
cursors stay 0 as before.

`OpenIntent` clears any client-supplied `settled_tx` / `receipt_ok`. A forged stamp could otherwise
raise the compaction horizon to `u64::MAX` and flag every later page as truncated.

`TransitionView` gains `#[serde(default)] settled_tx: u64`, and
`ok = (status == Received) && receipt_ok.unwrap_or(true)`.

### 2.2 Ordering (consistent cut)

- A settle applies every effect first: world-model contact, calibration, beliefs, the intent-record
  sync, and the observation and receipt records. Only then does it allocate its stamp, and it does
  so **while holding the `open_intents` lock**. The status mutation happens under that same lock.
- Stamps therefore become visible in allocation order. A reader holding the lock sees a prefix of
  the stamp sequence, so paging by `settled_tx > since` is gap-free.
- If adding the records fails, the receipt returns `Err` and nothing is settled. Before this change
  the status was flipped first.
- `snapshot()` reads `tx_cursor` **before** reading any content (fixes D5).
- Guarantee: `snapshot()` → `N`, followed by `transitions(since = N)`, delivers every settle at least
  once. A settle whose effects are visible in the snapshot but whose stamp is greater than `N` is
  delivered again, so consumers upsert by `intent_id`.
- The feed is a compacted changelog, carrying the latest state per intent.

### 2.3 Retention and horizon (fixes D2)

- `transition_retention` defaults to 4096 and is set via the builder `with_transition_retention(n)`,
  which enforces a minimum of 1.
- After each settle, the pure function `evict_settled(&mut Vec<ActionIntent>, cap) -> Option<u64>`
  drops the oldest-stamped settled intents beyond `cap`. Unstamped (Open/InFlight) intents are
  never evicted. It returns the largest evicted stamp, and `horizon_tx` is raised to it with
  `fetch_max`.
- `horizon_tx` starts at `TxLog::current_tx()` when the handle is built, or at 0 with no TxLog.
  Intents are not rehydrated on restart, so every pre-restart cursor is correctly reported as behind
  the horizon.
- `TransitionPage { transitions, next_cursor, horizon_tx, truncated }`:
  - `transitions` are sorted ascending by `settled_tx`;
  - `next_cursor = max(since, last.settled_tx)`;
  - `truncated = since < horizon_tx`. When `truncated` is true the consumer must re-bootstrap from
    `snapshot()`.

### 2.4 Canonical parent order (fixes D6)

- `get_parents` and `parents_of` return parent ids in ascending `String` order.
- Contract: `weights[i]` of a structural equation binds to the i-th parent in that order. This is
  documented on `StructuralEquation`.

### 2.5 SCM read model (fixes D3) — `src/scm_view.rs`

```text
ScmView        { as_of_tx, acyclic, nodes: [ScmNodeView], edges: [ScmEdgeView], interventions: [InterventionView] }
ScmNodeView    { id, parents (canonical order), equation: Option<EquationView>, noise_var, noise_var_vector }
EquationView   { shape: "scalar" | "directional", dim: Option<usize>, weights: Option<[f64]>, dir_weights: Option<[f64]> }
ScmEdgeView    { from, to, strength }
InterventionView { var, value, direction: Option<[f64]> }
```

- Every list in the view is sorted, so the JSON is byte-identical for identical graphs.
- Weights are exposed through new **defaulted** trait methods, `StructuralEquation::weights()` and
  `dir_weights()`, which return `None` by default. `LinearSE` and `DirectionalSE` override them. A
  custom equation that doesn't override them is shown with `weights: null`.
- Builders:
  - `ScmView::from_graph(&CausalGraph, as_of_tx)`;
  - `WorldModelEnhanced::scm_view(as_of_tx)`;
  - `CognitiveHandle::scm_view()`, which reads the TxLog cursor before the graph, so the view
    reflects at least every transaction ≤ `as_of_tx`.
- New `CausalGraph` accessors: `node(id)` and `pinned_vars()` (sorted).

### 2.6 Contract traits (additive)

`CognitiveStateProvider` gains two defaulted methods:

- `provider_transitions_page(actor, since)`, built from `provider_transitions_since` with horizon 0;
- `provider_scm() -> Option<ScmView>`, which returns `None`.

`CognitiveHandle` overrides both.

### 2.7 REST (feature `web-server`)

- `GET /snapshot/:actor/transitions?since=N` adds `next_cursor`, `horizon_tx` and `truncated`. The
  existing `transitions` and `count` keys are kept.
- New `GET /snapshot/:actor/scm` returns 200 with the `ScmView` JSON, or 500 with `{"error"}`. The
  SCM is global, so `actor` is accepted only for path symmetry, as `prediction_error` already does.
  The route is auth-protected like the other snapshot routes.
- `ROUTE_TABLE` gains the four routes v3.17.0 registered without declaring (`/laws`,
  `/policies/:entity_id`, `/snapshot/:actor/transitions`, `/snapshot/:actor/prediction_error`) plus
  `/snapshot/:actor/scm`.

### 2.8 MCP and Python SDK

- MCP tools `snapshot_transitions` (`actor` default `mcp`, as `open_intent`; `since` default 0) and
  `snapshot_scm` (`actor` default `_global`) return the REST JSON unchanged. Both go through
  `_req`, so a transport failure or non-2xx status comes back as `{"error"}` instead of raising.
  An empty `actor` falls back to the default, because an empty path segment never routes.
- `HipCortexClient.snapshot_transitions(actor, since=0)` and `snapshot_scm(actor="_global")`.
- Both surfaces percent-encode `actor` as one path segment (`/` → `%2F`).
- The VSIX has no MCP code of its own: `vscode:prepublish` copies `sdk/mcp/server.py` into the
  package, so a rebuilt VSIX carries both tools.

### 2.9 Out of scope

- VSIX `languageModelTools` entries for the new routes.
- A producer for `Denied`.
- Side effects of duplicate receipts.
- Snapshot performance.
- Rehydrating intents on restart.
- Downgrade to ≤ 3.17 after `IntentExpire` entries exist: older binaries skip unknown kinds when
  restoring the counter, so they can reuse tail tx ids. This combination is not supported.

## 3. Acceptance criteria

| AC | Criterion (measurable) |
|----|------------------------|
| AC-1 | `transitions_since(a, N)` returns exactly actor `a`'s retained intents with `settled_tx > N`, ascending |
| AC-2 | Unstamped intents (Open, InFlight) never appear |
| AC-3 | `ok` is false for a receipt with `ok: false` and for `Expired`; true for a successful receipt |
| AC-4 | With a TxLog, the receipt's returned `tx_cursor` == `intent.settled_tx` == `view.settled_tx` |
| AC-5 | A duplicate receipt changes neither `settled_tx` nor `receipt_ok` |
| AC-6 | A late receipt after `Expired` restamps higher, shows `Received`, and appears once in the feed |
| AC-7 | `expire_stale_intents()` stamps each expired intent from a `TxKind::IntentExpire` entry whose actor is the intent's actor |
| AC-8 | With retention R and M > R settles: exactly R settled intents are retained, open intents are untouched, and `horizon_tx` == the newest evicted stamp |
| AC-9 | `truncated` is true iff `since < horizon_tx`; `next_cursor == max(since, last stamp)` |
| AC-10 | Restart: the new handle's `horizon_tx` == `current_tx()` at reopen; the old cursor is truncated; the fresh snapshot cursor is not; new stamps > old stamps |
| AC-11 | `snapshot()` reads `tx_cursor` before any content lock (verified by code review) |
| AC-12 | 4 threads × 50 concurrent settles while a consumer pages: every id is delivered exactly once, stamps strictly increase across pages |
| AC-13 | Parents are returned sorted; the directional counterfactual gives `y_cf = 34` for `do(a=4)` in each of 32 freshly built graphs |
| AC-14 | `ScmView` JSON is identical across 32 fresh builds; parents align with weights; scalar and directional pins appear; `acyclic` is correct |
| AC-15 | Old JSON without the new fields deserialises; open intents serialise without them; v3.17-shaped provider/equation impls compile and get the defaults |
| AC-16 | Client-supplied `settled_tx` / `receipt_ok` on OpenIntent are cleared |
| AC-17 | REST exposes the new page fields and `/snapshot/:actor/scm`; `route_table_matches_build_app` passes |
| AC-18 | A caught-up page (≤ 16 new) over 4096 retained takes < 2 ms mean; a full 4096 page takes < 50 ms (debug build) |
| AC-19 | lib/unit/integration/property suites ≥ baseline with no new failures; clippy `-D warnings` introduces no new findings |
| AC-20 | MCP `snapshot_transitions` / `snapshot_scm` and the client methods request the right path (escaped actor, defaults, cursor), return the page fields intact, and the MCP tools are fail-silent (`sdk/python/tests/test_loop_ready_surface.py`) |

**Baseline (HEAD `6f0c744`):**

- petgraph build: lib 379, unit 555, integration 190, property 59.
- web-server integration: 336 pass, 2 fail — `route_table_matches_build_app`, fixed by AC-17, and
  `clarify_ladder_sit::react_on_a_goal_that_is_still_unclarifiable_is_rejected`, which is unrelated
  and left as a follow-up.
- Other failures already present: the acceptance v290 case `ac_g1_schema_mismatch_payload_returns_422_not_panic`
  and the `clarify_engine.rs` doctest.

**Result (verified 2026-09-27):**

- petgraph build: lib 379, unit 597, integration 194, property 59, with no failures.
- web-server, all targets: `route_table_matches_build_app` now passes. The three other failures
  listed above remain, and nothing else fails.
- AC-18 (debug build): a caught-up page with 16 new over 4096 retained takes 30 µs mean; a full
  4096 page takes 1.4 ms. The feed filters on the stamp before building views, so a caught-up
  poll does not pay for the retained window.
- clippy `-D warnings` still fails on findings that predate this change. None of its 342 findings
  is on a line this change adds.

**Result, MCP and SDK (verified 2026-09-28):**

- Python: 269 passed (258 before, plus the 11 AC-20 tests), including the bundled-mirror
  byte-identity and tool-surface checks.
- Live, against a v3.18 `webserver` with an API key set:
  - Paging from the snapshot's `tx_cursor` returned exactly the two settles made after it, in
    stamp order, with `ok` true and false and the open intent absent. A page from `next_cursor`
    was empty.
  - The actor `team/a b` round-trips through `%2F` and does not collapse into actor `team`.
  - `snapshot_scm` returned the view. MCP and SDK results were identical.
  - Without a key both routes return 401, which the MCP tools report as `{"error"}`.

## 4. Implementation order

1. Canonical parent order (AC-13).
2. Intent/view fields, `TransitionPage`, `evict_settled` (AC-1…3, 8, 9 unit level).
3. Handle stamping, retention, horizon, `transitions_page`, OpenIntent hygiene (AC-4…6, 16).
4. `TxKind::IntentExpire`, the `state_diff` arm, `expire_stale_intents`, daemon wiring (AC-7).
5. Snapshot cut ordering (AC-11).
6. `ScmView` with its accessors (AC-14).
7. Provider defaults (AC-15).
8. REST and `ROUTE_TABLE` (AC-17).
9. SIT: consumer loop, concurrency, restart, performance (AC-10, 12, 18).
10. Full suites and clippy (AC-19).
11. MCP tools and client methods (AC-20).
