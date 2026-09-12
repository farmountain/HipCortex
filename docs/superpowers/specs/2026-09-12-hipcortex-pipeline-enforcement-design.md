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

Plus the existing regression set, which must stay green and unmodified:
`unit_suite` 502, `integration_suite` 306, `property_suite` 59, the 24 `acceptance_suite*`
targets (181 tests), and the Python suite (237).

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
