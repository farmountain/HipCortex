# v0.4.0 Contract Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three server-extension contract mismatches that cause `POST /memory/link` to 422 on every call, the status bar to always show "loops 0", and `hipcortex_graph_search` to always return "No related memories found".

**Architecture:** All three fixes are in `src/web_server.rs`. Fix 1 adds serde field aliases so the server accepts both the internal naming (`from_id`/`to_id`) and the extension naming (`source_id`/`target_id`). Fix 2 promotes a nested field to the top-level response JSON. Fix 3 injects `memory_store` into the `search/related` handler and enriches each PPR result with the full `MemoryRecord` payload.

**Tech Stack:** Rust, Axum 0.6, Serde, reqwest 0.11 (integration tests), tokio (via `web-server` feature)

## Global Constraints

- Feature flag for compilation: `--no-default-features --features "petgraph_backend"` for lib tests; add `,web-server` when compiling integration tests that exercise HTTP endpoints
- All new test functions in `tests/integration/v040_contract_sit.rs` are gated by registering the module under `#[cfg(feature = "web-server")]` in `tests/integration/mod.rs`
- Never bypass `SafetyGuardrail::check_precondition` on mutations — none of these fixes touch mutation paths, so no safety changes needed
- `MemoryRecord` integrity hashes are not touched by these fixes
- Follow existing module CoT comment convention; no new modules are created

---

## File Map

| Action | File | What changes |
|--------|------|-------------|
| Modify | `src/web_server.rs:277-285` | Add `#[serde(alias)]` to `MemoryLinkRequest.from_id` and `.to_id` |
| Modify | `src/web_server.rs:666-671` | `memory_search_related_route` closure — inject `memory_store` clone |
| Modify | `src/web_server.rs:2092` | `handle_memory_search_related` — add generic `B`, `memory_store` param, `results` enrichment, update response JSON |
| Modify | `src/web_server.rs:3545-3559` | `handle_memory_live_beliefs` JSON — add `"loops_run"` key |
| Modify | `tests/integration/intelligence_wiring_sit.rs:13-21` | `make_app_state()` — add missing `topo_graph` field (compile error with `web-server` feature) |
| Create | `tests/integration/v040_contract_sit.rs` | Three test groups: serde alias, loops_run presence, results enrichment |
| Modify | `tests/integration/mod.rs` | Register `mod v040_contract_sit` under `#[cfg(feature = "web-server")]` |

---

### Task 1: Fix `POST /memory/link` 422 error (serde field aliases)

**Files:**
- Modify: `src/web_server.rs:277-285`
- Modify: `tests/integration/intelligence_wiring_sit.rs:13-21`
- Create: `tests/integration/v040_contract_sit.rs` (initial skeleton + Task 1 tests)
- Modify: `tests/integration/mod.rs`

**Interfaces:**
- Produces: `MemoryLinkRequest` accepts both `{"source_id","target_id"}` (extension) and `{"from_id","to_id"}` (SDK/CLI) — downstream callers unchanged

- [ ] **Step 1: Write two failing serde tests**

Create `tests/integration/v040_contract_sit.rs`:

```rust
//! SIT tests for v0.4.0 server-extension contract fixes.
//! G-LINK: POST /memory/link field name aliases
//! G-BELIEFS: GET /memory/live_beliefs loops_run key
//! G-RELATED: GET /memory/search/related results enrichment

use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::web_server::AppState;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::self_model::SelfModel;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use hipcortex::CausalTopoGraph;
use std::sync::{Arc, Mutex, RwLock};

fn make_state() -> AppState<InMemoryBackend> {
    AppState {
        memory_store:   Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model:    Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus:         Arc::new(Mutex::new(AureusBridge::new())),
        self_model:     Arc::new(SelfModel::new()),
        coherence:      Arc::new(CoherenceChecker::new()),
        topo_graph:     Arc::new(Mutex::new(CausalTopoGraph::new())),
    }
}

// ── G-LINK: serde alias tests ─────────────────────────────────────────────────

#[test]
fn test_memory_link_request_accepts_source_id_target_id() {
    use hipcortex::web_server::MemoryLinkRequest;
    let json = r#"{"source_id":"00000000-0000-0000-0000-000000000001","target_id":"00000000-0000-0000-0000-000000000002","relation":"supports"}"#;
    let req: MemoryLinkRequest = serde_json::from_str(json)
        .expect("extension-style source_id/target_id must deserialize");
    assert_eq!(req.from_id, "00000000-0000-0000-0000-000000000001");
    assert_eq!(req.to_id,   "00000000-0000-0000-0000-000000000002");
    assert_eq!(req.relation, "supports");
}

#[test]
fn test_memory_link_request_still_accepts_from_id_to_id() {
    use hipcortex::web_server::MemoryLinkRequest;
    let json = r#"{"from_id":"00000000-0000-0000-0000-000000000003","to_id":"00000000-0000-0000-0000-000000000004","relation":"caused_by"}"#;
    let req: MemoryLinkRequest = serde_json::from_str(json)
        .expect("SDK-style from_id/to_id must still deserialize");
    assert_eq!(req.from_id, "00000000-0000-0000-0000-000000000003");
    assert_eq!(req.to_id,   "00000000-0000-0000-0000-000000000004");
}
```

- [ ] **Step 2: Register the new test module in mod.rs**

Open `tests/integration/mod.rs`. Add after the last `#[cfg(feature = "web-server")]` block:

```rust
#[cfg(feature = "web-server")]
mod v040_contract_sit;
```

- [ ] **Step 3: Run tests to confirm they fail**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite test_memory_link_request_accepts_source_id_target_id -- --nocapture
```

Expected: FAIL — `"missing field from_id"` or similar serde error.

- [ ] **Step 4: Fix the intelligence_wiring_sit compile error**

Open `tests/integration/intelligence_wiring_sit.rs`. The `make_app_state()` function is missing the `topo_graph` field that was added to `AppState` in the TMF change. Without this fix, the `web-server` feature test run won't compile.

Replace:
```rust
pub fn make_app_state() -> AppState<InMemoryBackend> {
    AppState {
        memory_store: Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model: Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model: Arc::new(SelfModel::new()),
        coherence: Arc::new(CoherenceChecker::new()),
    }
}
```

With:
```rust
pub fn make_app_state() -> AppState<InMemoryBackend> {
    AppState {
        memory_store:   Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model:    Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus:         Arc::new(Mutex::new(AureusBridge::new())),
        self_model:     Arc::new(SelfModel::new()),
        coherence:      Arc::new(CoherenceChecker::new()),
        topo_graph:     Arc::new(Mutex::new(hipcortex::CausalTopoGraph::new())),
    }
}
```

Note: also add the import at the top of the file if not already present:
```rust
use hipcortex::CausalTopoGraph;
```
Then update the function to use the imported name:
```rust
        topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
```

- [ ] **Step 5: Add serde aliases to MemoryLinkRequest in web_server.rs**

Open `src/web_server.rs`. Find `MemoryLinkRequest` at approximately line 277. The full context is:

```rust
/// POST /memory/link — create a directed relational edge between two MemoryRecords.
/// relation: "supports" | "caused_by" | "follows" | "contradicts"
#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct MemoryLinkRequest {
    pub from_id:  String,  // MemoryRecord UUID
    pub to_id:    String,  // MemoryRecord UUID
    pub relation: String,
}
```

Replace with:

```rust
/// POST /memory/link — create a directed relational edge between two MemoryRecords.
/// relation: "supports" | "caused_by" | "follows" | "contradicts"
#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct MemoryLinkRequest {
    #[serde(alias = "source_id")]
    pub from_id:  String,  // MemoryRecord UUID
    #[serde(alias = "target_id")]
    pub to_id:    String,  // MemoryRecord UUID
    pub relation: String,
}
```

- [ ] **Step 6: Run tests to confirm they pass**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite test_memory_link_request -- --nocapture
```

Expected: 2 PASS — both alias and original field names deserialize correctly.

- [ ] **Step 7: Verify lib tests still pass**

```
cargo test --no-default-features --features "petgraph_backend" --lib
```

Expected: all pass (no web-server code compiled, no impact).

- [ ] **Step 8: Commit**

```
git add src/web_server.rs tests/integration/v040_contract_sit.rs tests/integration/mod.rs tests/integration/intelligence_wiring_sit.rs
git commit -m "fix(link): accept source_id/target_id aliases in MemoryLinkRequest

Extension sends source_id/target_id; server struct used from_id/to_id.
Serde aliases make both field name sets valid — breaks no existing callers.
Also fix intelligence_wiring_sit compile error (missing topo_graph field)."
```

---

### Task 2: Fix `GET /memory/live_beliefs` missing `loops_run` key

**Files:**
- Modify: `src/web_server.rs:3545-3559` (inside `handle_memory_live_beliefs`)
- Modify: `tests/integration/v040_contract_sit.rs` (add HTTP test)

**Interfaces:**
- Consumes: `make_state()` from Task 1 test file
- Produces: `GET /memory/live_beliefs` JSON response gains top-level `"loops_run": u64` key; all existing keys remain

- [ ] **Step 1: Write the failing HTTP test**

Append to `tests/integration/v040_contract_sit.rs`:

```rust
// ── G-BELIEFS: loops_run present at top level ─────────────────────────────────

#[tokio::test]
async fn test_live_beliefs_has_loops_run_at_top_level() {
    let state = make_state();
    let addr: std::net::SocketAddr = "127.0.0.1:3050".parse().unwrap();
    let srv = tokio::spawn(async move {
        hipcortex::web_server::run_with_state(addr, state).await;
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let resp = reqwest::get("http://127.0.0.1:3050/memory/live_beliefs")
        .await
        .expect("request failed");
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(
        body.get("loops_run").is_some(),
        "loops_run key missing — extension status bar will always show 0"
    );
    assert!(
        body["loops_run"].is_number(),
        "loops_run must be a number, got: {:?}",
        body["loops_run"]
    );

    srv.abort();
}
```

- [ ] **Step 2: Run test to confirm it fails**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite test_live_beliefs_has_loops_run -- --nocapture
```

Expected: FAIL — assertion `loops_run key missing` fires.

- [ ] **Step 3: Add loops_run to the live_beliefs JSON response**

Open `src/web_server.rs`. Find `handle_memory_live_beliefs` final return (around line 3545). The current JSON block ends with:

```rust
    Json(serde_json::json!({
        "symbolic_facts": symbolic_facts,
        "code_facts": code_facts,
        "current_hypotheses": current_hypotheses,
        "world_state": world_state,
        "intel": {
            "self": intel_self,
            "coherence": intel_coherence,
            "pinned_memories": pinned,
        },
        "summary": summary,
        "actor": actor,
        "limit": limit,
        "source": "unified /memory/live_beliefs (existing stores, no new persistence)",
    }))
```

Replace with:

```rust
    Json(serde_json::json!({
        "symbolic_facts": symbolic_facts,
        "code_facts": code_facts,
        "current_hypotheses": current_hypotheses,
        "world_state": world_state,
        "intel": {
            "self": intel_self,
            "coherence": intel_coherence,
            "pinned_memories": pinned,
        },
        "summary": summary,
        "loops_run": current_hypotheses.get("loops").and_then(|v| v.as_u64()).unwrap_or(0),
        "actor": actor,
        "limit": limit,
        "source": "unified /memory/live_beliefs (existing stores, no new persistence)",
    }))
```

- [ ] **Step 4: Run test to confirm it passes**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite test_live_beliefs_has_loops_run -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run all integration tests to check nothing broke**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite -- --nocapture 2>&1 | tail -20
```

Expected: no regressions.

- [ ] **Step 6: Commit**

```
git add src/web_server.rs tests/integration/v040_contract_sit.rs
git commit -m "fix(beliefs): add loops_run to /memory/live_beliefs top-level response

Extension reads response.loops_run for status bar display.
Value was nested under current_hypotheses.loops — now promoted to top level.
Existing keys unchanged."
```

---

### Task 3: Enrich `GET /memory/search/related` with full record data

**Files:**
- Modify: `src/web_server.rs:666-671` (`memory_search_related_route` closure)
- Modify: `src/web_server.rs:2092` (`handle_memory_search_related` signature + body + response JSON)
- Modify: `tests/integration/v040_contract_sit.rs` (add HTTP integration test)

**Interfaces:**
- Consumes: `MemoryStore<B>::find_by_id(uuid::Uuid) -> Option<MemoryRecord>` (already exists, used in `handle_memory_link`)
- Produces: `GET /memory/search/related` gains top-level `"results": [{score: f64, record: {id, actor, action, target, confidence}}]` key; existing `"related"` key kept for backward compat

- [ ] **Step 1: Write the failing HTTP integration test**

Append to `tests/integration/v040_contract_sit.rs`:

```rust
// ── G-RELATED: results key present with full record data ─────────────────────

#[tokio::test]
async fn test_search_related_returns_results_with_record_data() {
    let state = make_state();
    let addr: std::net::SocketAddr = "127.0.0.1:3051".parse().unwrap();
    let srv = tokio::spawn(async move {
        hipcortex::web_server::run_with_state(addr, state).await;
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let base = "http://127.0.0.1:3051";

    // Add memory A
    let body_a: serde_json::Value = client
        .post(&format!("{}/memory/add", base))
        .json(&serde_json::json!({"actor":"test","action":"decided","target":"use postgres"}))
        .send().await.unwrap()
        .json().await.unwrap();
    let id_a = body_a["record_id"].as_str().expect("record_id missing from add response").to_string();

    // Add memory B
    let body_b: serde_json::Value = client
        .post(&format!("{}/memory/add", base))
        .json(&serde_json::json!({"actor":"test","action":"confirmed","target":"postgres scales well"}))
        .send().await.unwrap()
        .json().await.unwrap();
    let id_b = body_b["record_id"].as_str().expect("record_id missing").to_string();

    // Link A → B (using from_id/to_id so this doesn't depend on Task 1 alias)
    let link = client
        .post(&format!("{}/memory/link", base))
        .json(&serde_json::json!({"from_id": id_a, "to_id": id_b, "relation": "supports"}))
        .send().await.unwrap();
    assert_eq!(link.status().as_u16(), 200, "link failed: {}", link.text().await.unwrap_or_default());

    // Search related from seed A
    let rel: serde_json::Value = client
        .get(&format!("{}/memory/search/related?seed_id={}&limit=10", base, id_a))
        .send().await.unwrap()
        .json().await.unwrap();

    // Must have results key (extension reads this)
    assert!(rel.get("results").is_some(), "results key missing — hipcortex_graph_search LM tool will always return empty");
    let results = rel["results"].as_array().expect("results must be array");
    assert!(!results.is_empty(), "expected at least one PPR result for linked seed");

    let first = &results[0];
    assert!(first.get("score").is_some(), "score missing from results[0]");
    assert!(first.get("record").is_some(), "record missing from results[0]");
    let record = &first["record"];
    assert!(record.get("id").is_some(),     "record.id missing");
    assert!(record.get("actor").is_some(),  "record.actor missing");
    assert!(record.get("action").is_some(), "record.action missing");
    assert!(record.get("target").is_some(), "record.target missing");
    // Verify it's actually the linked record B
    assert_eq!(record["action"], "confirmed",        "expected record B action");
    assert_eq!(record["actor"],  "test",             "expected record B actor");

    // Backward compat: related key still present
    assert!(rel.get("related").is_some(), "related key must still exist for backward compat");

    srv.abort();
}
```

- [ ] **Step 2: Run test to confirm it fails**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite test_search_related_returns_results_with_record_data -- --nocapture
```

Expected: FAIL — `results key missing`.

- [ ] **Step 3: Update memory_search_related_route to inject memory_store**

Open `src/web_server.rs`. Find `memory_search_related_route` (around line 666):

```rust
    let memory_search_related_route: axum::routing::MethodRouter = {
        let tg = topo_arc.clone();
        get(move |Query(p): Query<MemoryRelatedParams>| async move {
            handle_memory_search_related(tg, Query(p)).await
        })
    };
```

Replace with:

```rust
    let memory_search_related_route: axum::routing::MethodRouter = {
        let tg = topo_arc.clone();
        let ms = memory_store.clone();
        get(move |Query(p): Query<MemoryRelatedParams>| async move {
            handle_memory_search_related(tg, ms, Query(p)).await
        })
    };
```

- [ ] **Step 4: Update handle_memory_search_related with generic B and record enrichment**

Open `src/web_server.rs`. Find `handle_memory_search_related` (around line 2081). Replace the entire function with:

```rust
/// GET /memory/search/related handler.
///
/// Runs PPR (α=0.85 fixed, 20 iterations) over the CausalTopoGraph rooted at
/// "mem-{seed_id}". Strips the "mem-" prefix from results before returning.
/// Returns `results` (extension-compatible: [{score, record}]) and `related`
/// (backward-compat: [{id, score}]). If a record UUID is not found in the store,
/// the result entry still has `score` but `record` contains only `{id}`.
///
/// α=0.85 is standard PageRank damping (15% restart probability). This value
/// works well for graphs up to ~10K nodes. Expose as a query param in a future
/// change if empirical evidence calls for a different default.
#[cfg(feature = "web-server")]
async fn handle_memory_search_related<B: MemoryBackend + Send + Sync + 'static>(
    topo: Arc<Mutex<crate::topological_memory::CausalTopoGraph>>,
    memory_store: Arc<Mutex<MemoryStore<B>>>,
    Query(params): Query<MemoryRelatedParams>,
) -> Json<serde_json::Value> {
    let limit    = params.limit.unwrap_or(10).min(50);
    let seed_sym = format!("mem-{}", params.seed_id);

    match topo.lock() {
        Err(e) => Json(serde_json::json!({
            "seed_id": params.seed_id,
            "results": [],
            "related": [],
            "error":   format!("lock: {}", e),
        })),
        Ok(tg) => {
            let raw = tg.ppr(&seed_sym, limit, 0.85, 20);

            if raw.is_empty() {
                // Distinguish "seed not in graph" from "seed has no reachable nodes"
                // by checking whether the node exists at all (via neighbors probe).
                let has_any_edge = !tg.get_neighbors(&seed_sym).is_empty()
                    || !tg.get_incoming(&seed_sym).is_empty();
                if !has_any_edge {
                    return Json(serde_json::json!({
                        "seed_id": params.seed_id,
                        "results": [],
                        "related": [],
                        "note":    "seed_id has no graph edges — link it first via POST /memory/link",
                    }));
                }
            }

            // Build lightweight related vec (backward compat)
            let related: Vec<serde_json::Value> = raw
                .iter()
                .map(|(sym_id, score)| {
                    let id = sym_id.trim_start_matches("mem-").to_string();
                    serde_json::json!({
                        "id":    id,
                        "score": (score * 1000.0).round() / 1000.0,
                    })
                })
                .collect();

            // Build enriched results vec — look up full MemoryRecord for each result
            let results: Vec<serde_json::Value> = match memory_store.lock() {
                Ok(ms) => raw
                    .iter()
                    .map(|(sym_id, score)| {
                        let id_str = sym_id.trim_start_matches("mem-");
                        let score_rounded = (score * 1000.0).round() / 1000.0;
                        let record = id_str
                            .parse::<uuid::Uuid>()
                            .ok()
                            .and_then(|uid| ms.find_by_id(uid))
                            .map(|rec| serde_json::json!({
                                "id":         rec.id.to_string(),
                                "actor":      rec.actor,
                                "action":     rec.action,
                                "target":     rec.target,
                                "confidence": rec.confidence,
                            }))
                            .unwrap_or_else(|| serde_json::json!({"id": id_str}));
                        serde_json::json!({ "score": score_rounded, "record": record })
                    })
                    .collect(),
                Err(_) => vec![],
            };

            Json(serde_json::json!({
                "seed_id":   params.seed_id,
                "results":   results,
                "related":   related,
                "limit":     limit,
                "algorithm": "ppr",
                "alpha":     0.85,
            }))
        }
    }
}
```

- [ ] **Step 5: Run the new test to confirm it passes**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite test_search_related_returns_results_with_record_data -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run all v040 contract tests together**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite v040_contract -- --nocapture
```

Expected: 4 tests PASS (2 serde alias, 1 loops_run, 1 results enrichment).

- [ ] **Step 7: Run lib tests to confirm nothing broke**

```
cargo test --no-default-features --features "petgraph_backend" --lib
```

Expected: all pass.

- [ ] **Step 8: Run full integration suite with web-server feature**

```
cargo test --no-default-features --features "petgraph_backend,web-server" --test integration_suite 2>&1 | tail -30
```

Expected: no regressions. If port conflicts appear (`address already in use`), re-run — integration tests with fixed ports occasionally conflict under parallel execution.

- [ ] **Step 9: Commit**

```
git add src/web_server.rs tests/integration/v040_contract_sit.rs
git commit -m "fix(search): enrich /memory/search/related with full record data

Extension's hipcortex_graph_search LM tool reads res.results[].record.action
and res.results[].record.target for Copilot context. Server was returning
{related:[{id,score}]} — added memory_store lookup to build
{results:[{score,record:{id,actor,action,target,confidence}}]}.
Kept related key for backward compat (SDK, CLI)."
```

---

## Self-Review

### Spec coverage

| Fix | Task covering it |
|-----|-----------------|
| `POST /memory/link` 422 on `source_id`/`target_id` | Task 1 |
| `GET /memory/live_beliefs` missing `loops_run` | Task 2 |
| `GET /memory/search/related` empty `results` + no record data | Task 3 |
| `intelligence_wiring_sit` compile error with web-server feature | Task 1 Step 4 |

### Placeholder scan

No TBDs, no "implement later", no "add appropriate error handling". Every step has exact code.

### Type consistency

- `MemoryLinkRequest.from_id` / `.to_id` — used in both test assertions and in `handle_memory_link` at line ~2002 (`req.from_id`, `req.to_id`) — consistent.
- `handle_memory_search_related<B: MemoryBackend + Send + Sync + 'static>` — same bound as `handle_memory_link` and other generic handlers — consistent.
- `ms.find_by_id(uid)` — same call pattern as `handle_memory_link` at line ~2022 — consistent.
- `rec.actor`, `rec.action`, `rec.target`, `rec.confidence` — all public fields on `MemoryRecord` — consistent with usage throughout web_server.rs.
