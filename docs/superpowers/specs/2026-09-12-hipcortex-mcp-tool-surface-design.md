# HipCortex MCP Tool Surface — Design

**Date:** 2026-09-12
**Status:** Draft — self-reviewed; open questions resolved by self-prompt, see §3
**Scope:** the MCP tool contract in `sdk/mcp/server.py` and the test that keeps it honest.
**Complements:** `2026-09-12-hipcortex-gap-closure-design.md` (governs H1–H10; still in force).
**Source of facts:** `docs/hosts/mcp-copy-chain-drift-audit.md` (§4), which audited a handover document from
another repository against HipCortex's own artifacts.

---

## 0. Why this design exists

A handover document claimed a set of MCP copy-chain defects (G1–G9). Auditing it produced a different picture:
most gaps were **stale rather than real**, three had already closed, and **three of the live ones are
local-machine state, not repository defects**. But going one level deeper than the handover did turned up
something it never mentioned: **a defect in the tool contract itself**, which is a repository fact.

The audit's §3 separated fact from narrative for the handover. This design does the same for the *findings*:
only items that are (a) verified against artifacts, (b) in this repository, and (c) fixable without changing the
user's running environment are in scope. Everything else is recorded and deferred.

**The subject is not drift.** Drift is already gated (`pytest sdk/python/tests/` holds
`test_bundled_mirror_is_byte_identical_to_the_canonical_server`; `publish-pypi.yml` runs
`stamp_versions.py --mcp --check`). The subject is that **the tool list and the tool handlers disagree**, and
nothing checks that they agree.

---

## 1. Verified defect inventory

All four confirmed by `ast.parse` over `sdk/mcp/server.py` plus one execution proof. "Reads" below means the
handler reads that key from its `args` dict.

### D1 — `forget_actor` is advertised twice, and the first shape raises (HIGH)

| # | line | description | `required` |
|---|---|---|---|
| 1 | `:212` | "Delete all memories for an actor (GDPR right-to-forget / fresh start)." | `["actor"]` |
| 2 | `:630` | "GDPR hard-delete all records for an actor via ForgetActor delta." | `["actor_id"]` |

`handle_forget_actor` is defined **twice**:

* `:1073` reads `actor` → `_delete("/memory/forget/{actor}")`
* `:1424` reads `actor_id` → `POST /v1/cognitive/transact` with a `ForgetActor` delta

Python keeps the second. So the handler requires `actor_id` while **entry #1 — the one a host shows first —
advertises `actor`**, and the `DELETE /memory/forget/{actor}` body is unreachable dead code.

Proven by execution, not by reading. Importing the canonical module with `HIPCORTEX_URL` on a dead port and
calling `dispatch_tool("forget_actor", {"actor": "probe"})` raises `KeyError: 'actor_id'`; `main()`'s
`except Exception as e: respond(id_, error=str(e))` turns that into JSON-RPC `-32000`. The `{"actor_id": …}`
form passes the handler and reaches `POST /v1/cognitive/transact`.

This is the same failure mode as the `_req` incident — a handler dying on `-32000` — but caused by an argument,
not a missing global, so no existing test could have caught it.

### D2 — `add_memory` reads `intent_id` without declaring it (LOW)

`handle_add_memory:1025` routes a Temporal add through the Accept-Receipt seam when `intent_id` is present. The
schema declares `actor`, `action`, `target`, `record_type`, `ttl_seconds` and not `intent_id`, so a client cannot
discover the field. Nothing raises — the read is `.get`. Severity is low; the fix is one schema entry.

### D3 — `consolidate_memory` reads `actor` without declaring it (LOW)

`handle_p5_consolidate:1327` reads `args.get("actor", "mcp")` and attributes the resulting transaction to it.
The schema declares only `min_frequency`, so the attribution actor is always the literal `"mcp"` and a client
cannot set it. Nothing raises.

### D4 — `CLAUDE.md` states the tool surface wrongly (LOW)

`CLAUDE.md` says the MCP server exposes **"18 tools + 3 resources"**. Measured: `TOOLS` holds **62 entries
(61 unique)** and `RESOURCES` holds **7**. The "18" matches the *stale deployed copy*, so the figure predates the
canonical list growing and was never corrected. Same class as the spec's stale "6 call sites" (gap G7).

### Not fixed here, recorded instead

`handle_p5_consolidate:1349` calls `handle_add_memory({"memory_type": "Reflexion", "content": …, "metadata": …})`
intending to leave "a durable Reflexion{consolidation_ratio} for audit trail across restarts". `handle_add_memory`
ignores all three keys (it forwards only `actor`, `action`, `target`, `record_type`, `ttl_seconds`), so the call
writes a **Temporal** record and the ratio text is dropped. The comment describes behaviour that does not happen.
Deferred: fixing it means teaching `handle_add_memory` to forward `metadata`, which changes what reaches the
store, and this design is about the contract, not about substrate write volume.

---

## 2. Defects explicitly ruled out as non-facts

Not fixed, and not to be re-opened without new measurement:

* **`twin_create` declaring `dim`/`dt`/`max_covariance` that the handler "never reads"** — a probe artefact. The
  handler reads them via `body = {k: args[k] for k in ("dim", "dt", "max_covariance") if k in args}`, which a
  collector looking only for `args["literal"]` cannot see. Its `required` is empty, so it breaks no rule. This
  false positive is the reason the test in §4 asserts `required ⊆ reads` but **not** `declared ⊆ reads`.
* **The handover's G3–G5** (stale `~/.hipcortex-mcp/server.py`, a dead VS Code registry path, four pre-fix
  processes) — real, but local-machine state. Repairing them alters an environment in active use. Deferred to the
  user, in the order G4 → G5 → G3.
* **G8's bare `python`** — a real trap, but the repo-side "fix" (committing one machine's absolute interpreter
  path) makes the file non-portable. `install_hosts.py::_desired_mcp_entry()` already pins `sys.executable` for
  machine-local installs, which is the correct place for that decision.

---

## 3. Open questions, resolved by self-prompt

The standing constraint is that the user is unavailable, and that a clarify-clarity mechanism should self-resolve
where it can and escalate only what it cannot. Each question below is stated, answered from evidence, and given a
falsifier so the answer can be revisited rather than merely asserted.

**Q1: Should `forget_actor` keep the `DELETE /memory/forget/{actor}` behaviour (the dead handler) or the
`ForgetActor` delta behaviour (the live one)?**
*Answered: keep the delta.* The dead handler is unreachable, so keeping the DELETE path means *restoring*
behaviour that has not run since the duplicate was introduced — a bigger semantic change than deleting it. The
delta path is also the **auditable** one: `forget_actor` in `cognitive_state.rs:599` appends a `TxKind::ForgetActor`
to the tx log, whereas the design doc's H9 filed `DELETE /memory/forget/:actor` as *un-auditable*
(`records_deleted = ids.len()`, ids discarded). So the live path is the better of the two on the project's own
record. *Falsifier:* if a host is later found calling the DELETE endpoint through the MCP tool, restoring it is a
new decision, not a revert.

**Q2: Under which key should the surviving entry declare its argument?**
*Answered: `actor` required, `actor_id` optional.* `actor` is the project-wide convention — `dispatch_tool:1771`
itself does `args.get("actor", "_global")`, and every other tool spells it `actor`. But `actor_id` may already be
in use, because entry #2 advertised it and a host reading entry #2 would send it. Declaring `actor` and accepting
`actor_id` as a documented alias fixes the raised `KeyError` **and** does not break a caller of either shape.
*Falsifier:* if the repo ever drops the alias, it should be a deliberate deprecation, not a cleanup.

**Q3: Is a hard `delete_by_actor` acceptable without `CognitiveGC::gc_action` first?**
*Answered: out of scope, and no evidence of a violation to fix.* The project rule is to consult `gc_action` before
hard-deleting Temporal or Belief records; `forget_actor` calls `delete_by_actor` directly. But a GDPR
right-to-forget is the one operation whose *point* is deletion, and the behaviour predates this design. Changing
it changes privacy semantics. *Recorded here so the next reader does not mistake it for an oversight.*

**Q4: Should the new test assert the inverse direction (`declared ⊆ reads`)?**
*Answered: no.* Measured, that direction has exactly two hits: one is the D1 defect seen from the other side, the
other is the `twin_create` false positive. `declared ⊆ reads` would therefore make the suite red on a handler that
is correct, and would keep firing on any future handler that forwards `**args`. A rule that punishes a correct
handler is worse than no rule. *The asserted rules are in §4.*

---

## 4. The contract rules the test will enforce

| ID | Rule | Rationale | Currently |
|---|---|---|---|
| R1 | tool names in `TOOLS` are unique | a host showing two entries with one name shows one broken contract | **violated by D1** |
| R2 | every `TOOLS` name maps to a handler in `dispatch_tool`'s `handlers` dict, and every target names a module-level function | `dispatch_tool` is a dict lookup, so a name without an entry fails on first call; a target that is not a def fails at import | holds (measured) |
| R3 | every key in a tool's `required` is read by its handler | a required key nobody reads is a promise the server does not keep | **violated by D1** |
| R4 | every key a handler reads is declared in its schema | a read key the client cannot supply is unreachable behaviour | **violated by D1, D2, D3** |
| R5 | every global name a dispatched handler reads resolves at module scope (or is a local, parameter, lambda parameter, or builtin) | this is the general form of the `_req` defect | holds (measured) |

R5 is the load-bearing one, because it is the only rule with a **known-bad input to validate against**:
`sdk/python/build/lib/hipcortex/install/mcp_server.py` still lacks `_req`, and the check returns exactly
**17 unresolved sites** there — one per handler that calls it — while returning none for the canonical file, the
bundle, and the deployed copy. A check that reproduces a historical defect on the copy that had it, and clears the
copies that were fixed, is a check worth having. It is asserted in the test, not left as a probe.

R4's collector must handle `args["k"]`, `args.get("k")`, and `args[k]` over a constant collection — otherwise it
reports the `twin_create` false positive. The test will pin that case as a fixture so the collector cannot regress
into noise.

---

## 5. The fix

**D1.** Delete the dead `handle_forget_actor` at `:1073`. Delete schema entry #2 at `:630`. Keep one entry that
declares `actor` (required) plus `actor_id` (optional alias) and reads both in the live handler, raising a clear
`ValueError` when neither is present. Net effect on behaviour: the previously-broken argument shape starts working;
the previously-working one keeps working; the unreachable DELETE path is removed as dead code, not as a behaviour.

**D2, D3.** Add `intent_id` to `add_memory`'s properties; add `actor` to `consolidate_memory`'s properties. Both
optional. Both handlers already read them; only the declaration is missing.

**Bundle parity.** `sdk/mcp/server.py` and `sdk/python/hipcortex/install/mcp_server.py` must stay byte-identical.
They are synced by `scripts/stamp_versions.py`, and a test in CI already fails on drift, so the fix is not
complete until both are.

**D4 and the stale spec.** Correct `CLAUDE.md`'s counts. Restate the H2 criterion in
`2026-09-12-hipcortex-gap-closure-design.md` as "every `_req` call site resolves" instead of a count, so it cannot
go stale the way "6" did.

---

## 6. Acceptance criteria

1. `dispatch_tool("forget_actor", {"actor": "probe"})` no longer raises `KeyError`; with a dead `HIPCORTEX_URL` it
   fails at the **transport** (`ConnectionError`), which is the proof it got past the argument binding.
2. `dispatch_tool("forget_actor", {"actor_id": "probe"})` behaves identically.
3. `dispatch_tool("forget_actor", {})` raises `ValueError` whose message names the missing key.
4. `TOOLS` contains exactly one entry named `forget_actor`, with `required == ["actor"]`.
5. A grep for `def handle_forget_actor` finds exactly one definition.
6. The new test passes against the canonical file and is **RED when pointed at the `build/lib` copy** (the R5
   fixture). A test that cannot fail on a known-bad input is not evidence.
7. `sha256` of `sdk/mcp/server.py` equals that of the bundled copy, and
   `python scripts/stamp_versions.py --mcp --check` exits 0.
8. The full `sdk/python/tests/` suite passes, and so does `sdk/mcp/test_server.py`.

## 7. Non-goals

* No change to the four live MCP processes, no config edits, no interpreter pinning (user's environment).
* No change to `build/lib` (gitignored; regenerated by setuptools).
* No `metadata` forwarding in `handle_add_memory` (see §1 "Not fixed here").
* No version bump. The tool-list change is a defect fix within 3.10.0; a version move is a release decision.
* No change to any Kakeya/KARM-owned surface.

## 8. Exit conditions

This design stops when the acceptance criteria hold. It does not expand into "and while we were here": every
deferred item is named in §1 or §2 with its reason, so deferral is a recorded decision rather than an oversight.
If the R5 check ever needed to be loosened to make the suite pass, that would falsify §4's premise and must be
escalated rather than patched.
