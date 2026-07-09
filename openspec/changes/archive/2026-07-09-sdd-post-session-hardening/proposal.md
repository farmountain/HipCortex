## Why

The Plans A/B/C SDD session shipped 9 tasks cleanly, but the post-session review uncovered 6 permanently-failing integration tests (masked test bugs from `System:Self` seeding), a 5-field gap between the TypeScript SDK and the live server API, loose types in the VS Code extension, and missing invariant documentation for the decay scoring formula and PPR/pinned separation. These are hardening tasks that de-risk the next SDD session and eliminate CI red from main.

## What Changes

- **Fix 6 integration test failures** ！ `intelligence_hooks_sit` (5 tests) and `cli_exports_graph` (1 test) fail because `SymbolicStore::new()` seeds `System:Self` but tests assert absolute empty counts. Change to delta-relative assertions.
- **Align TypeScript SDK types** ！ Add `confidence`, `source`, `decay_factor`, `decay_half_life_secs`, `tags`, `priority` to `AddMemoryRequest` in `sdk/typescript/src/types.ts` (all optional, fully backward-compatible).
- **Tighten extension types** ！ Change `record_type?: string` to union type; change `metadata?: any` to `metadata?: Record<string, unknown>` in `vscode-extension/src/extension.ts`.
- **Document decay invariant** ！ Add inline doc comment in `src/memory_store.rs` stating compute_decay always returns in [0, confidence] and confidence acts as a permanent ceiling on achievable relevance score.
- **Document PPR/pinned separation** ！ Add API-level comments in `src/web_server.rs` and `src/openapi_spec.rs` explaining that `search_related` (PPR) scores graph centrality not record priority; pinned override only applies to `search_semantic`.

## Capabilities

### New Capabilities
- `search-scoring-invariants`: Formal documentation of the decay and priority scoring invariants in memory_store and the REST API spec.

### Modified Capabilities
(none ！ no spec-level behavior changes)

## Impact

- `tests/integration/intelligence_hooks_sit.rs` ！ 5 tests fixed (test code only)
- `tests/integration/world_model_cli_sit.rs` ！ 1 test fixed (test code only)
- `sdk/typescript/src/types.ts` ！ interface extended (additive, no breaking change)
- `vscode-extension/src/extension.ts` ！ type annotations tightened (no runtime change)
- `src/memory_store.rs` ！ doc comment added (no runtime change)
- `src/web_server.rs` / `src/openapi_spec.rs` ！ doc comment added (no runtime change)
- No Rust production code changes; no server behavior changes
- CI will go from 6 FAILED to 0 FAILED on integration suite
