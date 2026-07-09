## Context

After the Plans A/B/C SDD session (commits 78f130d¨C8f2bc05 on main), post-session review identified three categories of technical debt:

1. **Test correctness debt**: 6 integration tests fail permanently because they were written before `SymbolicStore::new()` began seeding `System:Self`. The tests assert absolute-count expectations that are now structurally wrong.
2. **API surface debt**: The TypeScript SDK `AddMemoryRequest` is 5¨C6 fields behind the live Rust server. SDK consumers cannot express `confidence`, `source`, `decay_factor`, `decay_half_life_secs`, `tags`, or `priority` ¡ª all of which the server accepts and the scoring pipeline uses.
3. **Documentation debt**: The decay scoring invariant and PPR/pinned separation are undocumented, creating a trap for future SDD implementers who may assume decay can amplify scores or that PPR inherits pinned priority.

All changes in this design are non-behavioral. No Rust production logic changes. No server API changes. No breaking changes anywhere.

## Goals / Non-Goals

**Goals:**
- Restore CI green: 0 failed integration tests on `--test integration_suite`
- SDK type parity with server `AddMemoryRequest` struct
- Explicit inline documentation of scoring invariants in source
- Zero ambiguity for the next SDD session implementer

**Non-Goals:**
- Changing the decay formula or scoring behavior
- Publishing to PyPI / npm / VS Marketplace (separate concern)
- Fixing unused-variable warnings in test files (minor, not blocking)
- Changing any REST API routes or response shapes

## Decisions

### D1: Fix test assertions using delta-relative counts, not backend bypass

**Choice:** Compute `initial_count = store.export_graph().0.len()` before adds, then assert `nodes.len() == initial_count + N`.

**Alternative rejected:** Use a raw `InMemoryGraph` backend directly to bypass `System:Self` seeding. Rejected because it would test a different code path (bare backend vs. production `SymbolicStore::new()`), reducing coverage of the identity anchor invariant.

**Alternative rejected:** Remove `System:Self` auto-seeding. Rejected because it is a documented canonical identity anchor needed by the coherence checker and world model integration. Removing it would be a production behavior change.

### D2: TypeScript SDK ¡ª extend interface, do not version-bump

**Choice:** Add 6 optional fields to `AddMemoryRequest` in `types.ts`. No new interfaces, no version bump for this patch.

**Rationale:** All fields are optional with server-side defaults. Existing SDK consumers pass only `actor/action/target` ¡ª they are unaffected. A version bump would be needed only if we broke backward compat, which we don't.

**Field types chosen to match server Rust types:**
```
confidence?: number          ¡û Option<f32> [0.0, 1.0]
source?: string              ¡û Option<String>
decay_factor?: number        ¡û Option<f32> [0.0, 2.0]
decay_half_life_secs?: number ¡û Option<u64> (seconds)
tags?: string[]              ¡û Vec<String>
priority?: "pinned" | "high" | "normal" | "low" ¡û String with enum validation
```

### D3: Extension type tightening ¡ª annotation change only

**Choice:** Change `record_type?: string` ¡ú union type literal, `metadata?: any` ¡ú `metadata?: Record<string, unknown>`. No change to axios call shape.

**Why:** TypeScript will catch callers passing invalid `record_type` strings at compile time. `Record<string, unknown>` prevents accidental `undefined` values appearing in JSON serialization.

### D4: Documentation placement

**Decay invariant** ¡ú inline doc comment above `compute_decay` in `memory_store.rs`. Engineers reading the scoring pipeline see it immediately.

**PPR/pinned separation** ¡ú two places:
1. Inline comment in `handle_search_related` in `web_server.rs`
2. The OpenAPI spec description string for `GET /memory/search/related`

This ensures it appears both in code review and in API consumers reading the spec.

## Risks / Trade-offs

- [Risk] Test delta-count approach requires the test to call `export_graph()` before any `add_node` calls. If a test calls `add_node` first, `initial_count` captures wrong baseline. ¡ú Mitigation: Review all 6 test bodies; place baseline read as first statement after store construction.

- [Risk] SDK type additions could conflict if a consumer has a local `AddMemoryRequest` that redefines the type. ¡ú Mitigation: All fields are optional; TypeScript structural typing absorbs this gracefully.

- [Trade-off] Documenting the confidence-as-ceiling invariant in source may prompt future PRs to "fix" it by splitting confidence from decay. This is an intentional design choice; the doc comment should explicitly note this is by design, not an oversight.

## Migration Plan

No migration required. All changes are:
- Test file edits (no deployment)
- TypeScript type additions (backward-compatible, no publish required for correctness)
- Rust doc comments (no recompile of production binary needed for fix)

Rollback: any individual change can be reverted independently ¡ª they are fully decoupled.
