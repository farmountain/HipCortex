# Pipeline Enforcement — closing the G1–G8 residuals

- **Status:** accepted (design), implementation pending
- **Date:** 2026-09-12
- **Predecessor:** `docs/superpowers/specs/2026-09-12-hipcortex-gap-closure-design.md` (WP1–WP10 / H1–H10)
- **Feature set for all verification:** `--no-default-features --features "web-server,petgraph_backend"`

---

## 1. Context

The predecessor workstream closed H1–H10 and was independently verified. Four commits landed
(`21c487b`, `f564abc`, `5c4586b`, `fb2cc4d`). The verification report closed with one HIGH
recommendation and four code-level residuals, and this document closes them.

Investigating *why* those residuals survived produced the actual root cause, and it is one
finding rather than eight.

### 1.1 The measured finding

| Measurement | Value |
| --- | --- |
| Integration tests CI builds and runs | **182** |
| Integration tests under `--features web-server,petgraph_backend` | **306** |
| REST contract tests CI **never** executes | **124** |
| Python tests CI executes from `sdk/python/tests/` | **~2 of 237** |
| Steps in `publish-pypi.yml` that check the shipped MCP mirror | **0** |

`ci.yml`'s `build-core` job runs `integration_suite` with `--features petgraph_backend` only.
Every `#[cfg(feature = "web-server")]` module in `tests/integration/mod.rs` is therefore
compiled out of the CI graph — including `rest_contract_safety_sit`, `route_parity_sit`,
`capability_catalog_sit`, `cognitive_report_sit` and `consolidate_safety_sit`. The only job
that carries the `web-server` feature is `build-web`, and it runs `cargo build` alone: it
compiles a binary and asserts nothing.

### 1.2 Why this is the whole story

WP5 made `build_app` declare its own routes in `ROUTE_TABLE`. WP7 made the capability catalog
derive its names from that declaration instead of holding a literal. The design principle was
**one declared authority, no hand-maintained parallel list.** That principle was applied at
runtime and never applied to delivery:

- route parity is proven by a test CI does not run;
- the MCP `_req` drift was unguarded at the one step that ships it;
- 237 green Python tests sit outside every job.

All three are the same defect class: **a claimed invariant with no enforcing gate.** So the
fix is also one thing: extend declaration-based truth from the router to the pipeline that
builds, tests and publishes it. That is what makes this cohesive rather than eight patches.

---

## 2. Goals and acceptance criteria

Every goal below states a **testable** acceptance criterion and an explicit **exit** — no goal
may be "complete" by assertion. Where a criterion cannot be tested as written it must be
renegotiated through the clarify ladder rather than silently weakened (see §7).

| ID | Goal | Acceptance criterion (testable) | Exit |
| --- | --- | --- | --- |
| **G2** | CI executes the web-server surface | `cargo test --no-default-features --features "web-server,petgraph_backend"` runs `integration_suite`, `unit_suite` and `property_suite` in one job; the count is 306 / 502 / ≥59, not 182 | job green |
| **G3** | CI executes the Python suite | `pytest sdk/python/tests/ -q` is a CI step and reports 237 passed | step green |
| **G1** | The PyPI mirror cannot drift | `publish-pypi.yml` runs `python scripts/stamp_versions.py --mcp --check` **before** `python -m build`; a divergence fails the release | check exits 0 |
| **G4** | No `delete_by_id` remains in `handle_consolidate` | the text between `async fn handle_consolidate` and the next top-level `fn` contains `upsert(` and `delete_by_ids(` and contains no `delete_by_id(` | test green |
| **G4b** | Consolidation is durable | a store built over a temp backend, where duplicates are added and consolidated, does not resurrect any dropped id after `flush()` + reload | test green |
| **G5** | The REST consolidate invariant is enforced | a sweep over ≥64 seeded shapes asserts active count never increases, no input id is lost, archived ids are exactly the Cold Store contents, and survivors ∩ archived = ∅ | test green |
| **G6** | `_req` is defined, fail-silent, and the mirror is identical | a Python test asserts all three | test green |
| **G7** | SQLite sidecars are ignored | `git status --porcelain` reports no `*.db-wal` / `*.db-shm` | clean status |
| **G8** | The clarify exit is informative | the 409 body names the supported retry route, and a test asserts the retry actually succeeds | test green |

---

## 3. Design

### 3.1 G2 — a CI job that carries the web-server feature

Add to `.github/workflows/ci.yml`:

```yaml
  web-tests:
    name: Test (web-server,petgraph_backend)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-web-${{ hashFiles('Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-web-
      - name: Build
        run: cargo build --no-default-features --features "web-server,petgraph_backend"
      - name: Unit tests
        run: cargo test --no-default-features --features "web-server,petgraph_backend" --lib
      - name: Integration tests
        run: cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite
      - name: Property tests
        run: cargo test --no-default-features --features "web-server,petgraph_backend" --test property_suite
```

The cache key is shared with `build-web` (`cargo-web`), so the new job reuses that job's
compiled artifacts instead of paying a full cold build. `build-web` keeps its release binary
build; the two jobs answer different questions ("does it ship" vs "is it correct").

Deliberately **not** added: `--all-features`. That requires PostgreSQL, Neo4j, RocksDB and a
Tauri toolchain, and `CLAUDE.md` is explicit that it is not the default expectation.

### 3.2 G3 — run the Python suite in CI

Add to the existing `python-sdk` job in `ci.yml`, after the langchain step:

```yaml
      - name: Run SDK test suite
        run: pytest sdk/python/tests/ -q
```

The job already installs `sdk/python[test]` plus `requests`/`langchain-core`/`langchain`, which
is the same environment in which all 237 tests were measured green (3.77 s). No new
dependency. `sdk/python/langchain_contrib/...` stays as its own step because it is a different
suite with a different filter.

### 3.3 G1 — guard the PyPI mirror at the shipping step

Add to `publish-pypi.yml`, immediately before `python -m build`:

```yaml
      - name: Verify bundled MCP server matches sdk/mcp/server.py
        run: python scripts/stamp_versions.py --mcp --check
```

`stamp_versions.py` resolves `REPO_ROOT` from `__file__`, so it is correct from any working
directory — no `working-directory:` override is needed, and none is added.

`sync_mcp_bundle()` is the single authority: it whole-file copies `sdk/mcp/server.py` to
`sdk/python/hipcortex/install/mcp_server.py`, and under `--check` it prints `MISMATCH` and
returns 1 without writing. Note that `stamp_mcp()` already calls it unconditionally, so the
guard is one line and reuses the existing contract rather than inventing a check.

The `_req` drift was able to ship because of how the file is installed. `install_hosts.py`
`_install_mcp_server()` (`:484–499`) tries `sdk/mcp/server.py` relative to the repo first, and
**falls back to `importlib.resources.path("hipcortex.install", "mcp_server.py")`** when that
path does not exist — which is exactly the case for a PyPI install. It then
`shutil.copy2`s the result to `~/.hipcortex-mcp/server.py`, the file MCP hosts actually
execute. So for every user who installs from PyPI, the **bundled mirror is the shipped
artifact**, and nothing between the mirror's content and the PyPI upload verifies it. That is
what makes this release-critical and what step §3.3 makes impossible.

### 3.4 G4 / G4b — two `MemoryStore` primitives, and the non-durable delete defect

#### 3.4.1 The defect this unblocks

There are two independent ways a removal through `delete_by_id` fails to stick, and together
they mean **`handle_consolidate`'s removals do not survive a restart.**

**(i) The pending write buffer is not purged.** `delete_by_id` does only:

```rust
self.records.retain(|r| r.id != id);
if removed > 0 { self.rebuild_indices(); true } else { false }
```

`self.buffer` is untouched, and `add` flushes only when `buffer.len() >= batch_size`
(`:268`). `batch_size` defaults to **8** for file-backed stores (`:72`, `:97`) — note that
`new_in_memory()` deliberately uses **1** (`:128`), so an in-memory test store flushes on every
`add` and cannot observe this. A duplicate added and not yet flushed is removed from `records`,
stays in `buffer`, and is **re-appended by the next `flush()`**.

**(ii) The append-only backend is never rewritten.** `MemoryBackend` (`persistence.rs:19–23`)
exposes exactly `load`, `append`, `flush`, `clear` — **there is no `delete`.** So a record that
has already been flushed to `memory.jsonl` is still on disk after `delete_by_id`, and
`MemoryStore::load` (`:163`) reads it straight back on the next start. The same is true of
`purge_expired`. `delete_by_actor` is the only removal in this file that persists, and it
persists precisely because it does `backend.clear()` followed by re-appending every surviving
record.

Consequence: the sibling method `delete_by_actor` shows the file's own established contract for
a durable removal, and `delete_by_id` does not meet it. So a consolidate that drops three
duplicates looks correct in memory, and after one restart the Hot Store holds all four again
**while the Cold Store still holds the three archived copies** — the store grew, the archive
grew, and every restart-plus-consolidate cycle repeats both.

This is not cosmetic. WP1's safety contract says non-survivors are archived "before removal, so
consolidation is recoverable" — but if removal is undone, the tiering invariant the archive
exists to protect is broken. It is precisely the invariant §5 of the predecessor spec asked to
be property-tested, and it is why the missing test mattered: the criterion was labelled a
Property and then never written, so nothing ever falsified it.

#### 3.4.2 Additions to `src/memory_store.rs`

Both are **additive**; no existing caller changes.

```rust
/// Replace the record with the same id in place, or insert it when absent.
///
/// `add` stores verbatim, so a caller that mutates a record before re-inserting it
/// (consolidation absorbing `evidence`, for example) otherwise has to
/// `delete_by_id` + `add`, which rebuilds every index twice and leaves the
/// integrity hash to the caller. This is that pair in one step, mirroring
/// `add`'s namespace tagging, buffering, audit entry and flush threshold.
pub fn upsert(&mut self, record: MemoryRecord) -> Result<()>;

/// Remove every record whose id is in `ids`. Purges `buffer` as well as `records`,
/// rewrites the append-only backend without them (the same contract as
/// `delete_by_actor`, because `MemoryBackend` has no `delete`), and rebuilds the
/// indices once. Returns the number of records actually removed.
pub fn delete_by_ids(&mut self, ids: &[uuid::Uuid]) -> Result<usize>;
```

The buffer purge and the backend rewrite are the correctness fix (§3.4.1); rebuilding indices
**once** instead of once per record is the performance fix, since `delete_by_id` calls
`rebuild_indices()` per record and so the current 3-way cluster rebuilds the indices four
times. The backend rewrite is guarded to run only when something was actually removed, and it
is `clear()` + re-append, which is O(n) — the same cost `/memory/forget` already pays, on an
endpoint that requires explicit `confirm: true`.

`upsert` keeps the caller's integrity contract: the handler still sets
`survivor.integrity = Some(survivor.compute_hash())` before calling it, which is the
behaviour the existing comment already documents.

`delete_by_id` is **left untouched.** It is public, it has a caller outside this handler
(`web_server.rs:5281`), and changing its buffering behaviour is a separate decision with a
separate blast radius. `delete_by_ids` documents the divergence explicitly so the next reader
does not assume they are interchangeable.

#### 3.4.3 Handler rewrite

In `handle_consolidate` only the two mutation sites change; ordering is preserved exactly:

```rust
// (2) Survivor absorbs the cluster's provenance, then duplicates go.
let mut survivor = keep_rec.clone();
survivor.evidence.extend(drop_recs.iter().map(|r| r.id));
/* … metadata: consolidated_from / consolidated_at, unchanged … */
survivor.integrity = Some(survivor.compute_hash());
if let Err(e) = ms.upsert(survivor) {
    errors.push(format!("reinsert survivor {}: {}", keep_rec.id, e));
}
survivors.push(keep_rec.id.to_string());

let dropped: Vec<uuid::Uuid> = drop_recs.iter().map(|r| r.id).collect();
if let Err(e) = ms.delete_by_ids(&dropped) {
    errors.push(format!("drop duplicates: {}", e));
}
```

Archive-before-remove is untouched: the Cold Store append still happens before either
mutation, and the journal is still written last. This closes the literal WP1 criterion ("no
`delete_by_id` remains in the handler") as a consequence of two better primitives rather than
by relocating the calls.

### 3.5 G5 — the invariant test

**Home:** `tests/integration/consolidate_safety_sit.rs`.

Rationale, and a deliberate deviation from the predecessor spec's "Property" label:
`property_suite` is a separate test binary and CI builds it **without** `web-server`, so a
REST-level property cannot live there without extracting the planning logic out of a
feature-gated handler. `consolidate_safety_sit.rs` is already `#[cfg(feature = "web-server")]`
and already owns `make_test_state`, `start_test_server` and the consolidate assertions.
Extracting production code purely to relocate a test would be a larger change for a weaker
result. The property lives where its subject lives, and G2 is what makes it enforced.

**Shape:** a deterministic seeded sweep (a small xorshift, no new dependency) over ≥64 shapes,
each varying cluster size (2–6), distractor count, confidence ordering and pinned-ness. Per
iteration, against a fresh actor on the live test server:

1. record `before` = ids returned by `GET /memory/query`.
2. `POST /memory/consolidate` with `dry_run: true` to read the plan.
3. `POST /memory/consolidate` with `dry_run: false, confirm: true`.
4. assert:
   - active count **after ≤ before** (never grows);
   - every id in `before` is either a survivor, in `archived_ids`, or is the journal;
   - `archived_ids` are exactly the ids in the Cold Store (no silent loss, no phantom);
   - `survivors ∩ archived_ids = ∅`;
   - `errors` is empty.

The dry-run preview is also asserted to be byte-identical to the plan the mutation reports,
which pins WP1's "preview then confirm" contract at the same time.

**Durability is asserted separately, at the store level, and deliberately not over REST.**
There is no `/memory/flush` route (verified), and resurrection-by-restart cannot be observed
from inside a live process, so a REST test would either be vacuous or require lifecycle
control the harness does not have. The assertion therefore lives beside the primitive it
tests, in `tests/unit/`:

- build a `MemoryStore` over a temp backend directory with the default `batch_size` of 8;
- add several records that cluster as duplicates (so they sit in `buffer`), plus enough
  additional records that one of them has already been flushed;
- `delete_by_ids(&dropped)`;
- `flush()`;
- assert the backend file does not contain the dropped ids, **and** that a fresh
  `MemoryStore` loading the same path yields none of them.

Note the trap this avoids: `new_in_memory()` sets `batch_size: 1` (`:128`), so it flushes on
every `add` and cannot exercise the buffer path. A test written against `new_in_memory` would
pass before and after the fix. The test must be file-backed.

### 3.6 G6 — the Python `_req` test

**New file:** `sdk/python/tests/test_mcp_server_req.py`. Imports
`hipcortex.install.mcp_server` (the package `__init__.py` exists, and `main()` is guarded by
`if __name__ == "__main__"` at `:1987`, so importing is safe and side-effect free).

- `_req` is defined and callable.
- **fail-silent:** patch `requests.request` to raise `ConnectionError`; `_req("GET", "/nope")`
  returns a `dict` containing `"error"` and does **not** raise. This is the assertion that
  would have failed before `21c487b`.
- a non-2xx response returns `{"error", "status", "detail"}` rather than raising.
- **mirror identity:** `sdk/mcp/server.py` and
  `sdk/python/hipcortex/install/mcp_server.py` are byte-identical. This is the same invariant
  as G1's CI step, asserted at the unit level so it fails in a developer's own test run and not
  only at release time.

### 3.7 G7 — `.gitignore`

Add to the `# SQLite files` block:

```
*.db-wal
*.db-shm
```

`*.db` is already listed, so this only completes an intent that was already declared. This is
what makes `.tokensave/` (which holds `tokensave.db` plus a `-wal` and `-shm` sidecar) show as
untracked today. `.tokensave/` itself is deliberately **not** added: ignoring a whole directory
would hide future content that may deserve tracking, whereas the three file patterns mirror
`*.db`'s existing scope exactly.

### 3.8 G8 — an informative exit, not a behaviour change

`web_server.rs:2021` currently returns:

```rust
{"error": "cannot clarify completed goal"}
```

For a **Failed** goal that is misleading — it is not completed, and the caller is not stuck.
`POST /goal/:id/react` only returns 422 when `success_factors` is empty, so a Failed goal that
already has factors from a resolved ladder is retryable through `/react`. That is the supported
path; `/clarify` is correctly refused because there is nothing left to clarify.

Change the message to name that exit:

```rust
{"error": "cannot clarify a settled goal: success_factors already exist — retry with POST /goal/{goal_id}/react"}
```

No test asserts the current string (verified: the only occurrence is the handler itself), so
this is a message change plus a new test asserting the retry path succeeds end to end.

**As built (refinement found during implementation).** The gate is
`terminal && (succeeded || !success_factors.is_empty())`, which covers two states that need
different advice: a `Succeeded` goal is genuinely completed and has nothing to retry, whereas a
`Failed` goal with factors is retryable. Emitting the "success_factors already exist" wording for
the first case would assert something false — a `Succeeded` goal may have no factors at all. The
body is therefore chosen per state:

| State | `error` | `fix` |
| --- | --- | --- |
| `Succeeded` | `cannot clarify a completed goal` (unchanged) | `null` |
| `Failed` with factors | `cannot clarify a settled goal: success_factors already exist` | `retry with POST /goal/{goal_id}/react` |

`status` is echoed for observability. The `Succeeded` string is deliberately left byte-identical
so the change cannot alter behaviour on a state that was never misdescribed.
Test: `a_settled_failed_goal_answers_409_naming_the_retry_that_works` in
`tests/integration/clarify_ladder_sit.rs` — asserts the 409 body names `/goal/{id}/react`, that
`error` does **not** call a `Failed` goal "completed", and that following the advice returns 200
(`ReactEngine::run` has no terminal-status guard; `/react` 422s only on an empty factor list).

---

### 3.9 G9 — the abort that hid every step after it (found by running the sweep)

**Found, not planned.** The validation sweep for this change ran the exact command
`build-core` runs. It failed with exit 101 — so the finding was investigated rather than assumed
to be a regression.

*Evidence, in order:*

1. `cargo test --no-default-features --features "petgraph_backend,tokio" --lib` → **101**,
   two consecutive runs (deterministic). 369 passed, 1 failed.
2. The panic is `src/actors/world_model_actor.rs:378`:
   `assertion failed: intervention_res.contains_key("wet_floor") || intervention_res.is_empty()`.
3. `git show 17055a0 -- src/actors/` — this change's only edit to that file is **+1 line**
   (`intervention_vector: None,` in a struct literal, a compile fix for the new field). Not the
   cause.
4. `git show origin/main:src/.../causal.rs` — `origin/main` carries the **same** value-keyed tail
   in `CausalGraph::compute_intervention`. So this is not introduced here.
5. `gh run list` / `gh run view 34604611892` — `origin/main`'s own CI run matching `395776d` is
   **`failure`**. The failing job is `Build + Test (petgraph_backend)`; the failing step is
   `X Unit tests (with tokio actors)` with `Process completed with exit code 101`, and
   **every later step is `-` (skipped)**: `Integration tests`, `Integration tests (with tokio
   actors)`, `Property tests`, `Property tests (with tokio actors)`, `Clippy`, `Clippy (with tokio
   actors)`, `Rustfmt`.

Step 5 is the important one, and it is worse than a single red test: **`origin/main` has never run
its integration, property, clippy or rustfmt gates.** They are declared in the workflow and have
never executed. This is a second, independent instance of the defect this whole change exists to
fix (§1.1) — a gate that is declared and never reached is not a gate. G2's rationale is therefore
understated: not "the web-server feature was untested" but "any step below a step that fails is
untested, and step order is a silent dependency."

**Root cause.** `WorldModelEnhanced::causal_intervention` has two branches. The empirical branch
(when distributions have been recorded) inserts `<outcome>=<value>` keys and then repeats the MAP
estimate under the bare outcome name; `mod.rs`'s own test pins that contract
(`res.get("Y")`). The fallback branch — taken when no empirical distributions exist — delegated
straight to `CausalGraph::compute_intervention`, which keys by outcome **value** alone
(`{"1": 0.5}`). One public method therefore answered in one of two shapes depending on hidden
state, and the actor test reads the bare outcome name. Under the empirical branch it works; under
the fallback it gets a non-empty map with no addressable outcome and panics. The test is
correct — it is asserting the contract the method's own documentation and sibling test assert.

**Decision — fix where the contract is stated, not where the shape is produced.** The alternative
was to relax the assertion. Rejected: the assertion states the public contract, and the empirical
branch already honours it. The fix goes in the wrapper
`WorldModelEnhanced::causal_intervention` (the `mod.rs` fallback call), **not** in
`CausalGraph::compute_intervention`, because `tests/integration/intelligence_sit.rs` calls the
graph-level function directly and a change there would alter a second, independent public API to
fix a defect in the first. `CausalGraph` keeps its value-keyed shape.

The fallback is re-keyed into the empirical shape — additive, nothing dropped:

```rust
let heuristic = graph.compute_intervention(&query)?;
if heuristic.is_empty() {
    return Ok(heuristic);
}
let mut result: HashMap<String, f64> = heuristic
    .into_iter()
    .map(|(value, p)| (format!("{}={}", query.outcome, value), p))
    .collect();
let map_estimate = result
    .iter()
    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
    .map(|(_, &p)| p);
if let Some(p) = map_estimate {
    result.insert(query.outcome.clone(), p);
}
Ok(result)
```

The empty case is returned untouched: an empty map stays empty, so the "no answer" case keeps
meaning no answer and no key is invented.

*Falsification:* the new test was written first and run against unfixed code. It failed at
`tests/unit/scm_foundations_tests.rs:564` with the actual map in the message —
`the outcome variable is not addressable: {"1": 0.5}`. After the fix: 1 passed, and the previously
red CI step is `370 passed; 0 failed`.

Regression test: `causal_intervention_fallback_is_keyed_by_the_outcome_variable` in
`tests/unit/scm_foundations_tests.rs`. It is deliberately **not** tokio-gated — the actor test
that found this only runs under `--features tokio`, so the contract would be unpinned in the
minimal build, which is the build the default CI job runs first. It asserts the outcome variable is
addressable, that value-qualified keys survive, and that the bare key is the MAP over them.

Consumers audited before choosing the site (19 call sites, 9 files): the REST handler
`handle_wm_causal_intervention` forwards the map as `outcome_probabilities` and gains the
consistent shape; `examples/causal_agent_demo.rs`, `benches/world_model_bench.rs` and
`tests/integration/intelligence_wiring_sit.rs` do not assert on keys; only
`tests/integration/intelligence_sit.rs` is sensitive to the graph-level shape, and it is untouched.

---

### 3.10 G10 — the suite no CI step named (found by the same sweep)

The census that closed §3.9 was run against the **registrar**: for each of `tests/unit/`,
`tests/integration/` and `tests/property/`, list the `.rs` files and check each one against the `mod`
declaration that turns a file into a module. 160 files, one orphan:
`tests/integration/v040_contract_sit.rs` — a file with a real assertion history, most recently
`dfad2ea` (link aliases), `f2d76b9` (`loops_run`) and `2dde95d` (search/related enrichment), none of
which had ever executed.

The registrar reading of that orphan turned out to be wrong, and the correction is part of the
finding. The file was **not** forgotten: `77b418b` ("move v040_contract_sit to standalone binary,
avoid intelligence_sit compile errors") deliberately removed it from the registrar and declared it as
its own `[[test]]` target with `required-features = ["web-server"]` — a legitimate way to keep one
web-gated suite off another suite's compile path, and an ancestor of `origin/main`. But a standalone
target is compiled and run only by a job that names it, and every job in `ci.yml` names `unit_suite`,
`integration_suite` and `property_suite`. So the six tests were declared, compiled on demand, and
never executed by the pipeline: the same outward symptom as G1–G3, reached from a different gate.
Re-registering the module would have reverted a deliberate decision to close a hole that decision did
not open; the fix is a `web-tests` step that names the target.

*Evidence:*

1. Six tests, three of them the v0.4.0 contract fixes the file is named for — G-LINK
   `POST /memory/link` field aliases, G-BELIEFS `GET /memory/live_beliefs` top-level `loops_run`,
   G-RELATED `GET /memory/search/related` record enrichment. A repo-wide search for
   `v040_contract` returned nothing before the fix.
2. Running it for the first time — first as a temporary registrant, then as the standalone target it
   already is — went **red**: `test_worldmodel_rollout_endpoint` at `v040_contract_sit.rs:235`,
   expected `actions must be non-empty`, got
   `No actions available for MCTS (observe transitions first)`. A file that has never run is not a
   file that would have passed. The temporary registration was reverted afterwards, so the target
   ships exactly as `77b418b` declared it, plus a step that runs it.
3. Root cause: `handle_wm_rollout` resolved its default mode as
   `unwrap_or(if req.actions.is_empty() { "mcts" } else { "dirichlet" })`. With empty `actions` and
   no `mode`, the MCTS branch answered first, so the `actions must be non-empty` guard beneath it was
   **unreachable for the request it was written for**. `git log -S` puts both that inference and the
   `(or set mode=mcts)` suffix on the guard's message in one commit, `bc5d6f7` (2026-07-20).
4. The endpoint's archived spec is the authority and disagrees:
   `openspec/changes/archive/2026-07-09-worldmodel-rollout-endpoint/design.md` fixes the guard as an
   input check returning exactly `{"error": "actions must be non-empty"}` — in D2, in the component
   design, and again in verification item 4 — and predates `bc5d6f7` by nine days.
   `WmRolloutRequest`'s own doc comment still reads `"dirichlet" (default)` with `actions` "Optional
   when mode=mcts". The code had drifted from its spec *and* from its documentation, and nothing
   executed the assertion that would have said so.
5. The defect is not cosmetic. A caller posting an empty `actions` list without a mode was answered
   about MCTS they never requested, by a `mode` the handler had inferred for them.

*Decision:* the mode default is plainly `"dirichlet"`, and the guard's message is restored to the
specified string. MCTS is opted into through `mode` and is not advertised by a validation error;
MCTS with no actions still works when `mode` asks for it, which
`tests/integration/worldmodel_self_http_sit.rs` pinned already and still passes. Editing the
assertion instead was rejected: the archived design pins the exact string in three places, and the
primitive the guard fronts — `WorldModelEnhanced::rollout_dirichlet` — returns that same wording for
the same input.

*Verification:* the target is green at **6 passed / 0 failed** as its own `web-tests` step, and the
`web-server` integration suite is **309 → 310, 0 failed** (the +1 is the regression test recorded in
§3.12); the web unit suite is 374/0. Both assertions the test went red on are the endpoint's two
specified error contracts: the empty-actions guard, and `No trained predictors available` when
nothing is trained.

*Found and left open:* the same audit turned up one contract that nothing adjudicates.
`docs/superpowers/specs/2026-08-13-hipcortex-gap-remediation-design.md` §2.1 and `CLAUDE.md` both
state the server-side caps as `iterations <= 200, max_depth <= 10`, while `check_rollout_depth`
rejects `depth > 5` and `handle_wm_rollout` clamps `max_depth` to 5. The OpenAPI entry at
`src/openapi_spec.rs:556` declares no maximum for either, so it does not break the tie, and every
test that pins a 5 is about `k` (`actions.len()`, "7 actions → only 5 steps"), which is a different
quantity from MCTS tree depth — `v040_contract_sit.rs` exercises `max_depth` only at values ≤ 3.
With no test and no declaration on either side, choosing 5 or 10 would be preference rather than
evidence, so behaviour is left unchanged and the discrepancy is recorded here instead of being
resolved silently. This is the clarify ladder's exit applied to a spec question: the ambiguity is
real and bounded, so it gets named rather than blocking.

*Generalisation, corrected by the above:* a file being committed is not evidence that it runs, and a
census against the registrar is necessary but not sufficient — the registrar gates only `mod`-included
files, whereas a `[[test]]` target is gated by the CI command line that names it and by nothing else.
Two censuses are needed: the directory against the registrar, and the declared targets against the
steps that name them. A declared target nobody names is as invisible as a file nobody declares — and
this change drew its evidence from the first census while the defect lived entirely in the second.

### 3.12 G11 — the guardrail that blocked the operation for its own identifiers

Running `v040_contract_sit` also surfaced an intermittent second failure, and that one is a defect in
the product rather than in the suite. In one of the twelve runs it was given:

```
assertion `left == right` failed: link failed:
  {"error":"precondition blocked: PII risk=0.90 patterns=[\"PII:369623-7774\"]","success":false}
  left: 403
 right: 200
```

The request was `POST /memory/link` carrying two UUIDs and `"relation": "supports"`. There is no
content in it to be personal data. The guard's context was
`format!("link {} --[{}]--> {}", req.from_id, req.relation, req.to_id)`, and the PII set's US-phone
pattern is `\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}` — three digits, an optional separator, three
digits, a separator, four digits — which **six digits, a hyphen and four more digits** satisfies
exactly. That is what a UUID presents at one of its hyphens: `…369623-7774…`. A v4 UUID carries such
a straddle roughly once in twenty, and with two ids per request the endpoint refused well-formed
requests at random, on identifiers the caller does not choose and cannot re-roll — retrying the same
ids always fails, and the only recourse is to store the records again and hope. A false positive at
that rate is a reliability defect, not a strictness setting, which is why it is removed rather than
tolerated.

The classifier is not wrong; it is answering the question it was asked. The defect is the question.
`/memory/add` classifies `actor action target`, its free text, and `/topo/apply_hyp` classifies `text`:
both content. `/memory/link` classified identifiers, which cannot be content, and was the only site in
the tree that did. Its context is now `format!("link memory records --[{}]-->", req.relation)` — the
check still runs, the one caller-supplied free-text field is still in scope, and the record ids are no
longer asked to prove they are not phone numbers. They leave the audit context with this change: they
are not what the check examines, and what the check refuses is what gets logged.

Every other classification site was audited for the same mistake and is clean: `temporal_backend.rs`
classifies the FSM condition, `semantic_cache.rs` the cache key length, and
`check_precondition_with_threshold` and `check_postcondition` have no callers at all. Widening the
phone pattern with a word boundary was rejected: it still has to match `(555) 123-4567`, and the same
boundary that rejects `x123456-1234` would suppress that form too. The detector is not where the
mistake is.

*Verification:* `link_does_not_classify_identifiers_as_pii` in
`tests/integration/rest_contract_safety_sit.rs` builds the false positive deterministically —
`ab123456-1234-4abc-8def-0123456789ab` is a valid UUID whose first hyphen carries the phone shape —
and asserts **404** (the store was consulted and the synthetic records were not found) where the
pre-fix handler answered **403** (the classifier refused), reporting `patterns=["PII:123456-1234"]`.
The `web-server` integration suite is 309 → 310, 0 failed, and the standalone target ran 6/0 on three
consecutive runs.

### 3.11 A2 — recorded, not reconciled

The G9 and G10 audits read `docs/superpowers/specs/2026-08-13-hipcortex-gap-remediation-design.md`,
whose header says `**Status:** IMPLEMENTED`, and found one of its directives apparently at odds with
the code: §2.2 asks to *"Remove the linear heuristic from `compute_intervention`. Return explicit
`Err` if empirical distributions are missing."* Read literally, that second clause would delete the
code path G9 had just re-keyed, so both halves were checked rather than assumed.

- **Clause 1 is satisfied — the directive is stale, not disobeyed.** No linear heuristic remains in
  `CausalGraph::compute_intervention` (`causal.rs:851`); it computes a genuine backdoor adjustment,
  `P(Y|do(X=x)) = Σ_z P(Y|X=x,Z=z) · P(Z=z)`. The only occurrences of "linear" in that file are
  `LinearSE`, the directional structural equation type contributed by H10, which is unrelated to
  intervention querying. The spec's `IMPLEMENTED` status is defensible on this reading, and §2.1 and
  §2.3 of the same spec match the code exactly (mode enum plus the `iterations ≤ 200` cap; the
  Joseph-form covariance update with symmetrization at `world_model_enhanced/entity.rs:178`).
- **Clause 2 is not implemented, and cannot be implemented literally without overturning a live
  assertion.** `compute_intervention` answers with `unwrap_or(0.5)` placeholders and a uniform `P(Z)`
  when `prior_Z` is absent (`causal.rs:903`, `:942`, `:970`) — the silent-failure shape Sprint 1 set
  out to remove. But `tests/integration/intelligence_sit.rs:414` asserts `intervention.is_ok()` on a
  fresh graph with three nodes and no empirical distributions at all, which is precisely the input
  clause 2 says must produce `Err`. The suite pins the current contract; the two cannot both hold.

No behaviour changes. Clause 2 is a real, user-visible question — should an absent intervention
estimate be a placeholder or an error — it is outside the G1–G10 mandate, and the repository holds
evidence on both sides rather than a rule. It is recorded here so the next reader does not
rediscover it as a defect, and so the decision is taken deliberately. This is the clarify ladder's
exit again: bounded, named, and not silently resolved in either direction.

### 3.13 G12 — the packaging gate that checked size, and the path that never called it

This one was found by using the product rather than by auditing it: the extension directory
`farmountain.hipcortex-memory-3.10.0` answered `/health` with `{"version":"3.5.0"}`.

The payload behind that answer was an 8,030,208-byte executable containing the literal `3.5.0`, in a
directory whose manifest said 3.10.0. Every staged binary under `vscode-extension/server/` was 3.5.0
too — linux-amd64 6,818,056, linux-arm64 6,129,672, darwin-amd64 6,474,656, darwin-arm64 2,478,520
(**truncated**; the published asset is 6,036,832) and win32 8,030,208 — and the VSIX built from that
tree (15,386,848 B) embedded the same stale executable. The published `v3.10.0` assets were correct
throughout, the windows asset at 8,102,400 B and the published VSIX at 17,069,132 B embedding it.
So the build was right and the packaging was wrong; a correct release had been assembled from an
incorrect tree.

Two independent causes, each survivable alone:

1. **The gate could not fail for the reason that mattered.** `isValidBinary()` tested
   `size >= 1_000_000` and "the first 32 bytes are not `PLACEHOLDER` and not `<!`". A stale 8 MB
   `MZ` executable satisfies both. The guard's answer to "is this the binary we want?" was "it is
   large and not HTML". This is G10's lesson in another costume: the check existed, ran, and was
   blind — *declaring* a check is not *having* one that can see the defect.
2. **The packaging path never reached the fetcher.** `vsce package` runs the `vscode:prepublish`
   script, which is `npm run package` = `webpack --mode production`; `fetch-bins.js` was reachable
   only through the separate `package:vsix` script or by hand. The gate and the path that needed it
   never met, so even a perfect `isValidBinary()` would not have run during packaging.

What let the two drift apart without a symptom is that the fetch target was a hardcoded
`RELEASE_TAG = 'v3.10.0'`: the staged binaries and the crate version had no shared source of truth to
disagree about. `EXPECTED_VERSION` now reads `VERSION` (with a `HIPCORTEX_SERVER_VERSION` override),
`RELEASE_TAG` derives from it, `binaryHasVersion()` scans the file as latin1 for that literal, and
`isValidBinary()` requires it — which makes "this binary is the right version" the same statement as
"this binary matches this checkout", the thing the release actually needs to be true.

`--check` is the pull-able form of the assertion: per platform it prints `ok`, `STALE` or `absent` and
exits non-zero if any staged binary is not `v${EXPECTED_VERSION}`. `main()` is guarded by
`require.main === module` and the module exports `EXPECTED_VERSION`, `RELEASE_TAG`,
`readExpectedVersion`, `binaryHasVersion` and `isValidBinary`, so the gate is testable rather than
self-certifying. And `release.yml`'s `package-vsix` job now runs it, because that job copies the
`build-release` artifacts into `vscode-extension/server/` and seals them *without* re-fetching — the
precise window in which a stale matrix ships under a new label.

*Verification:* RED with the change stashed — `src/test/fetch-bins.test.ts` reports 8 failed / 8
(`isValidBinary is not a function`); GREEN restored — 8 passed. Against the real tree
`node scripts/fetch-bins.js --check` printed `STALE` for all five platforms and exited 1, and after
`npm run fetch-bins` (6,858,056 / 6,166,536 / 6,511,664 / 6,036,832 / 8,102,400) printed `ok` for all
five and exited 0. The extension suite is unchanged at 87 passed / 2 suites. The installed extension
was repaired through the supported path — `code --install-extension <published vsix> --force`, since a
hand copy of the same file had not stuck — and now measures 8,102,400 B, contains `3.10.0`, does not
contain `3.5.0`, and serves `/health` version 3.10.0.

**Correction, recorded rather than amended away.** The first draft of this section attributed a
second symptom to the stale binary: that `POST /memory/add` with `record_type=belief` stored
`Temporal`. It does not. `origin/main`'s `parse_record_type_alias` is exact-match and case-sensitive —
`Some("Belief") => MemoryType::Belief`, `_ => MemoryType::Temporal` — while the local commits ahead of
`origin/main` replace it with a case-insensitive parser that returns `Err` instead of coercing. A
lowercase `"belief"` therefore stores `Temporal` on *any* released build, a correct one included, and
the two binaries that disagreed about it were HEAD and the release, not 3.5.0 and 3.10.0. The
stale-binary finding stands on the version literal and the `/health` output alone.

*Residual, recorded as open when this section was written:* no CI job ran the extension's jest suite,
so these 8 tests were exercised by hand only and the `release.yml` gate was the fix's sole automated
consumer. The next section closes it.

### 3.14 G12b — the suite that had no owner

That residual was left open on purpose — it is the G10 shape, and naming it is the point — and then
closed in the same window for the same reason G10 was: a test suite nothing runs is documentation.
`ci.yml` now carries a `vscode-extension` job (`actions/setup-node@v4` caching npm against
`vscode-extension/package-lock.json`, `npm ci`, `npm test`). It needs neither a Rust toolchain nor a
server, so it belongs in the always-on pipeline rather than in the release path.

The gate itself is deliberately *not* run there. `vscode-extension/server/` is untracked, so on a
fresh checkout `--check` reports `absent` for all five platforms and exits 1 — correct for
`release.yml`, where `build-release` has just populated that tree and absence means the packaging job
has nothing to seal, and wrong for `ci.yml`, where absence is the normal state. The same binary that
is a gate in one job is a false alarm in the other; the placement is the design.

*Verification:* `npm ci` exit 0 and `npm test` **87 passed / 2 suites** on a clean install, run
locally against the committed lockfile.

---

## 4. Test plan — acceptance criterion to named test

| AC | Test / command |
| --- | --- |
| G1 | `python scripts/stamp_versions.py --mcp --check` exits 0; a deliberately diverged copy exits 1 |
| G2 | CI job count: 306 integration / 502 unit / ≥59 property (measured locally at the same feature set) |
| G3 | `pytest sdk/python/tests/ -q` → 237 passed |
| G4 | `consolidation_uses_bulk_primitives_not_delete_by_id` — slices `src/web_server.rs` via `include_str!` between `async fn handle_consolidate` and the next top-level `fn`, asserts `upsert(` and `delete_by_ids(` are present and `delete_by_id(` is absent (the trailing paren matters: `delete_by_ids(` does not match `delete_by_id(`) |
| G4b | `delete_by_ids_purges_buffer_and_rewrites_backend` — file-backed store, default batch size 8, `flush()`, then reload and assert the dropped ids are gone |
| G5 | `consolidation_never_grows_the_store_and_never_loses_an_id` in `consolidate_safety_sit.rs` |
| G6 | `test_mcp_server_req.py::test_req_is_defined_and_fail_silent`, `::test_bundled_mirror_matches_canonical` |
| G7 | `git status --porcelain` free of `*.db-wal` / `*.db-shm` |
| G8 | `a_failed_goal_with_factors_is_retried_by_react_not_clarify` in `clarify_ladder_sit.rs` |
| G9 | `causal_intervention_fallback_is_keyed_by_the_outcome_variable` in `tests/unit/scm_foundations_tests.rs` (not tokio-gated); the tokio actor step that found it is `370 passed; 0 failed` |
| G10 | `test_worldmodel_rollout_endpoint` in `v040_contract_sit.rs`, reachable only because the `web-tests` job now names the standalone target (`cargo test --test v040_contract_sit`); target 6 passed / 0 failed, `web-server` integration 310, 0 failed |
| G11 | `link_does_not_classify_identifiers_as_pii` in `tests/integration/rest_contract_safety_sit.rs` — asserts 404 rather than 403 for a link whose UUIDs carry the phone shape; `web-server` integration 310, 0 failed |
| G12 | `vscode-extension/src/test/fetch-bins.test.ts` (8 tests: version literal required, stale 3.5.0 body rejected for 3.10.0, oversized HTML rejected, undersized rejected, missing invalid, `EXPECTED_VERSION` equals the repo `VERSION`, `RELEASE_TAG` derived from it); the gate itself is `node scripts/fetch-bins.js --check`, exit 1 → 0 across `npm run fetch-bins`, and it is now called by `release.yml`'s `package-vsix` before `vsce package` |
| G12b | the `vscode-extension` job in `ci.yml` runs `npm ci` then `npm test` — the first CI step that executes this suite; locally `npm ci` exit 0, `npm test` 87 passed / 2 suites |

Plus the existing regression set, which must stay green and unmodified:
`unit_suite` 502 → 510, `integration_suite` 306 → 310 (`web-server`, the +4 being G5's consolidation
invariant, G8's failed-goal retry, the `/memory/query` vocabulary test and G11's link regression) /
182 (minimal), `property_suite` 59, the standalone `v040_contract_sit` target (6 tests, previously
unnamed by any step), the 24 `acceptance_suite*` targets (181 tests), and the Python suite
(237 → 242). Counts after this change were measured locally at the same feature sets, and every one
is green (`unit` 510/0, `integration` 182/0 minimal and 310/0 web, `property` 59/0, Python 242
passed, `v040_contract_sit` 6/0).

New `MemoryStore` unit tests live in the existing `tests/unit/` aggregate (registered in
`tests/unit/mod.rs`):
`upsert_replaces_in_place_and_inserts_when_absent`,
`delete_by_ids_purges_buffer_and_rewrites_backend`,
`delete_by_ids_rebuilds_indices_once`.

The new REST test in `consolidate_safety_sit.rs` needs no registration (`tests/integration/mod.rs`
already carries that module under `#[cfg(feature = "web-server")]`), and the new Python file
is collected by `pytest sdk/python/tests/` automatically.

---

## 5. Non-goals

- No Kakeya / geodesic / action-sweeping work (KARM), and no multi-horizon decomposition or
  policy optimisation (KARAM).
- No CI-triage subsystem that routes build failures through the clarify ladder. That is a new
  production capability, not a gap closure, and `karpathy-guidelines` "Simplicity First"
  forbids building it unasked. The clarify/exit mechanism is instead **exercised** by this
  change (§7) rather than extended.
- No change to `delete_by_id` semantics (see §3.4.2).
- No `--all-features` job.
- No removal of `karm/memory/consolidator.py` (sibling repo, K5).
- No new capability names that would create a second authority for anything already listed in
  `ownership.yaml`.

---

## 6. Assumptions

- **A1 (verified by reading, to be falsified by test in step 1 of implementation):**
  `delete_by_id` purges neither `buffer` nor the append-only backend; file-backed stores
  default to `batch_size` 8 while `new_in_memory()` uses 1; `MemoryBackend` has no `delete`;
  and `delete_by_actor` is the only row in this file that shows the durable-removal contract
  (`clear()` + re-append). Confirmed at `memory_store.rs:229–241`, `:268`, `:72`, `:97`,
  `:128`, `:667–700`, `persistence.rs:19–23`. Per `systematic-debugging`, the first
  implementation step is a failing test for this, not a fix.
- **A2:** the user granted autonomous execution ("work autonomously and make good decisions")
  after the scoping question was posed, and every option offered closed the code gaps. Option A
  is taken: code gaps **plus** pipeline rails, without the new CI-triage subsystem. This
  substitutes for an explicit design approval; the decision is recorded here so it can be
  reversed cheaply, and each gap is a separate commit for exactly that reason.
- **A3:** GitNexus MCP tools are not available in this session, so the impact analysis required
  by `AGENTS.md` is performed with the local equivalent — `grep` for every caller of each
  touched symbol (`upsert` and `delete_by_ids` are new, so they have none; `handle_consolidate`
  is reached only from its router entry in `build_app`; the 409 string has no readers).
- **A4:** the 124 currently-unrun integration tests and 237 Python tests are green **today** at
  the CI feature sets, so adding the jobs does not import a backlog. Measured: 306/502/59 and
  237/0.
- **A5:** `VERSION` 3.10.0 remains the single source of truth; no version bump is part of this
  change.

---

## 7. The clarify/exit discipline applied to this change

This section is the cross-cutting requirement, applied to itself.

Every acceptance criterion in §2 is a **goal with testable AC**, not a to-do. The discipline
that the WP10 ladder enforces at runtime is applied here at design time:

1. **Self-resolution before escalation.** Of the three genuine unknowns found, two were
   resolved from the repository rather than by asking: G8 turned out not to be a defect (the
   `/react` retry path exists), and G5's home was determined by the fact that CI builds
   `property_suite` without `web-server`. Only the scope of "fully complete" was escalated, and
   the escalation carried the self-resolved items so they could be vetoed instead of
   re-litigated.
2. **Exit for every loop.** The design has a bounded stopping rule: eight goals, each with a
   named test and a named exit. No goal is "done" by assertion; `verification-before-completion`
   requires the command output, not the claim.
3. **No untestable AC survives.** Where the predecessor spec's "Property" label could not be
   satisfied in `property_suite`, that criterion is **renegotiated explicitly** (§3.5) rather
   than quietly dropped or quietly loosened — the same behaviour the ladder demands of a goal.
4. **The workstream's own goal is registered in the substrate** with its success factors, and
   driven to completion through `POST /goal/:id/react` → `GET /goal/:id/verify`, so the
   mechanism is exercised by the change that claims to respect it.

---

## 8. Risks and rollback

| Risk | Mitigation |
| --- | --- |
| The new CI job makes CI slower | shared `cargo-web` cache key with `build-web`; unit + integration + property in one job to compile once |
| `upsert` diverges from `add` semantics (namespace tag, buffering, audit) | it is a literal mirror of `add`'s body with the push replaced by a replace-or-push; the parity test asserts the audit entry and flush threshold both fire |
| `delete_by_ids` buffer purge + backend rewrite is behaviourally new | both match `delete_by_actor`, the file's own precedent; the rewrite is guarded to the "something was removed" case and is covered by a dedicated unit test |
| The backend rewrite makes consolidate O(n) instead of O(drops) | consolidate already requires `confirm: true`; the cost is the same as `/memory/forget`; correctness of the tiering invariant outweighs it, and the alternative is silent resurrection on restart |
| The 409 message is a wire string | verified no test or client reads it; the new test asserts the replacement |
| New CI jobs surface pre-existing failures | A4: both feature sets measured green today |
| Anything here turns out wrong | one commit per gap; any single gap reverts without touching the others |

---

## 9. Changelog note

`CHANGELOG.md` gains one `### Fixed` entry covering the durability defect (§3.4.1: deletions
by id survived a restart, so consolidation could re-grow the Hot Store while the Cold Store
kept its copies) and one `### Changed` entry for the 409 message (§3.8). CI and `.gitignore`
changes are not user-visible and are not listed.
