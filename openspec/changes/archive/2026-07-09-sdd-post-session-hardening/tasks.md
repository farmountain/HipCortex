## 1. Fix Intelligence Hook Tests (intelligence_hooks_sit.rs)

- [x] 1.1 Read lines 160-173 of `tests/integration/intelligence_hooks_sit.rs`. In `test_symbolic_store_constructs_with_intelligence`: capture `initial_nodes` before any adds, then assert `nodes.len() == initial_nodes.len()`. Current assertion `assert!(nodes.is_empty())` must be replaced.
- [x] 1.2 In `test_symbolic_store_add_node_with_self_model` (line ~176): assert `nodes.len() == initial_count + 1` (1 added node) rather than any absolute count. Capture baseline after store construction.
- [x] 1.3 In `test_symbolic_store_health_reporter_initial_state`: identify the assertion that uses absolute node count; apply delta-relative pattern.
- [x] 1.4 In `test_symbolic_store_nodes_and_edges_with_intelligence`: capture baseline before adding test nodes; assert `nodes.len() == baseline + N` for however many nodes the test adds.
- [x] 1.5 In `test_symbolic_store_full_intelligence_pipeline`: same delta-relative fix for any `nodes.len()` or `edges.len()` assertions.
- [x] 1.6 Run `cargo test --no-default-features --features "petgraph_backend" --test integration_suite intelligence_hooks_sit -- --nocapture` and confirm 5/5 pass.

## 2. Fix CLI World Model Test (world_model_cli_sit.rs)

- [x] 2.1 Read `tests/integration/world_model_cli_sit.rs` (26 lines). In `cli_exports_graph`: after `SymbolicStore::from_backend(backend)`, insert `let baseline = { let (n, _) = store.export_graph(); n.len() };` before `add_node` calls. Change `assert_eq!(parsed.0.len(), 2)` to `assert_eq!(parsed.0.len(), baseline + 2)`.
- [x] 2.2 Run `cargo test --no-default-features --features "petgraph_backend" --test integration_suite cli_exports_graph -- --nocapture` and confirm it passes.
- [x] 2.3 Run the full integration suite and confirm `test result: ok. N passed; 0 failed`. Record the passing count.

## 3. Add Decay Invariant Doc Comment (memory_store.rs)

- [x] 3.1 Open `src/memory_store.rs`. Locate the `compute_decay` function (currently at ~line 394). Replace or extend the existing doc comment block to include the invariant: `/// INVARIANT: return value is always in [0.0, rec.confidence].` Add: `/// confidence at ingestion is a permanent score ceiling ！ a record with confidence=0.5` and `/// can contribute at most 0.5x of its relevance to the final weighted score.` Add: `/// This is intentional: low-confidence memories should not dominate even if query-relevant.`
- [x] 3.2 Confirm `cargo check --no-default-features --features "petgraph_backend"` passes with no new errors.

## 4. Document PPR/Pinned Separation (web_server.rs + openapi_spec.rs)

- [x] 4.1 In `src/web_server.rs`, locate the `handle_search_related` function (or the route handler for `GET /memory/search/related`). Add a block comment directly above the PPR invocation: `// NOTE: PPR scores reflect graph centrality from the seed node (topological proximity).` and `// The pinned=2.0 score override used by search_semantic does NOT apply here.` and `// If a pinned record is unreachable from the seed via CausalTopoGraph edges, it will not appear.`
- [x] 4.2 In `src/openapi_spec.rs`, locate the description string for `GET /memory/search/related`. Append to it: `" Scores reflect graph centrality (PPR), not record priority. Pinned priority does not apply; pinned records appear only if topologically reachable from the seed."` Keep it in the same string literal format already used.
- [x] 4.3 Confirm `cargo check --no-default-features --features "petgraph_backend"` passes.

## 5. Align TypeScript SDK Types (sdk/typescript/src/types.ts)

- [x] 5.1 Open `sdk/typescript/src/types.ts`. Locate `export interface AddMemoryRequest` (currently ends at line 8). Add the following optional fields after `ttl_seconds?: number;`:
  ```typescript
  /** Reliability signal [0.0, 1.0]. Affects decay scoring ceiling. Default 1.0. */
  confidence?: number;
  /** Source identifier for trust weighting (e.g. "user-input", "claude-3-7"). */
  source?: string;
  /** Per-record decay rate multiplier [0.0, 2.0]. 1.0=normal, 0=no decay. Default 1.0. */
  decay_factor?: number;
  /** Per-record decay half-life in seconds. Default 2592000 (30 days). */
  decay_half_life_secs?: number;
  /** Categorization tags for filtering and RAG retrieval (e.g. ["bug", "decision"]). */
  tags?: string[];
  /** Search ranking priority. "pinned" records always surface first at score 2.0. */
  priority?: "pinned" | "high" | "normal" | "low";
  ```
- [x] 5.2 Run `cd sdk/typescript && npm run build` and confirm 0 TypeScript errors.

## 6. Tighten Extension Type Annotations (vscode-extension/src/extension.ts)

- [x] 6.1 Open `vscode-extension/src/extension.ts`. Locate `interface AddMemoryRequest` (line ~74). Change `record_type?: string;` to `record_type?: "Temporal" | "Symbolic" | "Procedural" | "Reflexion" | "Perception";`
- [x] 6.2 In the same interface, change `metadata?: any;` to `metadata?: Record<string, unknown>;`
- [x] 6.3 Run `cd vscode-extension && npm run compile` and confirm 0 TypeScript errors.
- [x] 6.4 If any call sites pass `metadata: someAnyTyped` and now fail type check, cast them: `metadata: someAnyTyped as Record<string, unknown>`. Fix all compile errors before proceeding.

## 7. Add System:Self Spec to openspec/specs

- [x] 7.1 Create directory `openspec/specs/search-scoring-invariants/` if it does not exist.
- [x] 7.2 Copy `openspec/changes/sdd-post-session-hardening/specs/search-scoring-invariants/spec.md` to `openspec/specs/search-scoring-invariants/spec.md`. This promotes the delta spec to the main spec directory.

## 8. Final Verification

- [x] 8.1 Run `cargo test --no-default-features --features "petgraph_backend" --test integration_suite 2>&1 | Select-String "test result"` ！ expect `0 failed`.
- [x] 8.2 Run `cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | Select-String "test result"` ！ expect `307 passed; 0 failed` (no regressions).
- [x] 8.3 Run `cd sdk/typescript && npm run build` ！ expect 0 errors.
- [x] 8.4 Run `cd vscode-extension && npm run compile` ！ expect 0 errors.
- [x] 8.5 Run `git diff --stat HEAD` to confirm only the expected files were touched (no accidental production Rust logic changes).
- [x] 8.6 Commit with message: `fix(tests): resolve 6 integration failures + SDK/ext type alignment + scoring invariant docs`
