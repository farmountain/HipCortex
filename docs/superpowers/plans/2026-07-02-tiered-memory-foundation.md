# Tiered Memory Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the three highest-impact tiered-memory behavioral gaps identified in the capability audit: working-memory TTL eviction (records accumulate forever), consolidate endpoint that finds-but-never-deletes duplicates, and the complete absence of memory↔memory relational edges.

**Architecture:** Three independent, additive Rust changes that each follow TDD: (1) add `purge_expired` + `delete_by_id` to `MemoryStore` and wire a background eviction `tokio::spawn`; (2) fix `handle_consolidate` to execute the deletes it already identifies; (3) bridge `MemoryRecord` UUIDs into the existing `CausalTopoGraph` infrastructure and expose two REST endpoints (`POST /memory/link`, `GET /memory/neighbors/:id`). No new dependencies. No changes to existing API contracts.

**Tech Stack:** Rust 2021 edition, Axum 0.6, Tokio, petgraph (existing), `IndexMap` (existing), `chrono` (existing). Build with `--no-default-features --features "petgraph_backend"` for unit tests, add `web-server` for integration tests.

## Global Constraints

- Build command: `cargo build --no-default-features --features "petgraph_backend"`
- Unit test command: `cargo test --no-default-features --features "petgraph_backend" --lib`
- Test suite command: `cargo test --no-default-features --features "petgraph_backend" --test unit_suite`
- Integration test command: `cargo test --no-default-features --features "petgraph_backend,web-server" --test unit_suite`
- New test files go in `tests/unit/` and MUST be registered in `tests/unit/mod.rs`
- New modules in `src/` MUST be registered in `src/lib.rs`
- Never use `cargo build --all-features` — it requires external databases
- All `tokio::spawn` loops use `tokio::time::interval`, not `sleep`
- `MemoryStore::new_in_memory()` returns `Self` (not `Result<Self>`) — no `.unwrap()` needed
- FileBackend tests create and delete a temp `.jsonl` file per test (follow pattern in `tests/unit/memory_store_tests.rs`)
- Keep changes surgical: no pre-existing cleanup, no unrelated refactors

---

### Task 1: Working Memory TTL Eviction

Closes the gap where expired records accumulate forever in the `records: Vec<MemoryRecord>` and `.jsonl` file. After this task, a background task purges every 5 minutes and `ms.all()` is consistent with query results.

**Files:**
- Modify: `src/memory_store.rs` — add `rebuild_indices()` (private) and `purge_expired() -> usize` (public)
- Modify: `src/web_server.rs` — add eviction `tokio::spawn` block in `run_with_state`
- Create: `tests/unit/memory_store_eviction_tests.rs`
- Modify: `tests/unit/mod.rs` — register new test file

**Interfaces:**
- Produces: `pub fn purge_expired(&mut self) -> usize` on `MemoryStore<B: MemoryBackend>`
- Produces: `fn rebuild_indices(&mut self)` (private, used by `purge_expired` and `delete_by_id` in Task 2)

---

- [ ] **Step 1: Write the failing tests**

Create `tests/unit/memory_store_eviction_tests.rs`:

```rust
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;

fn expired_record(actor: &str, target: &str) -> MemoryRecord {
    let mut r = MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    );
    r.expires_at = Some(chrono::Utc::now().timestamp() - 100); // 100s in the past
    r
}

fn live_record(actor: &str, target: &str) -> MemoryRecord {
    let mut r = MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    );
    r.expires_at = Some(chrono::Utc::now().timestamp() + 3600); // 1h in the future
    r
}

fn eternal_record(actor: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    )
    // expires_at = None → never expires
}

#[test]
fn purge_expired_removes_only_expired_records() {
    let mut store = MemoryStore::new_in_memory();
    store.add(expired_record("agent", "thing1")).unwrap();
    store.add(live_record("agent", "thing2")).unwrap();
    store.add(eternal_record("agent", "thing3")).unwrap();

    let removed = store.purge_expired();

    assert_eq!(removed, 1, "should remove exactly the expired record");
    assert_eq!(store.all().len(), 2);
    assert!(
        store.all().iter().all(|r| r.target != "thing1"),
        "expired record should be gone"
    );
}

#[test]
fn purge_expired_returns_zero_when_nothing_expired() {
    let mut store = MemoryStore::new_in_memory();
    store.add(live_record("agent", "a")).unwrap();
    store.add(eternal_record("agent", "b")).unwrap();

    let removed = store.purge_expired();

    assert_eq!(removed, 0);
    assert_eq!(store.all().len(), 2);
}

#[test]
fn purge_expired_rebuilds_actor_index() {
    let mut store = MemoryStore::new_in_memory();
    store.add(expired_record("alice", "old_task")).unwrap();
    store.add(live_record("alice", "current_task")).unwrap();
    store.add(eternal_record("bob", "bobs_task")).unwrap();

    store.purge_expired();

    // Index must be consistent: alice should only have 1 result
    let alice_records = store.find_by_actor("alice");
    assert_eq!(alice_records.len(), 1, "alice index must be rebuilt after purge");
    assert_eq!(alice_records[0].target, "current_task");
}

#[test]
fn purge_expired_removes_all_expired_when_all_expired() {
    let mut store = MemoryStore::new_in_memory();
    store.add(expired_record("agent", "a")).unwrap();
    store.add(expired_record("agent", "b")).unwrap();
    store.add(expired_record("agent", "c")).unwrap();

    let removed = store.purge_expired();

    assert_eq!(removed, 3);
    assert_eq!(store.all().len(), 0);
}
```

- [ ] **Step 2: Register the test file**

In `tests/unit/mod.rs`, add after the last `mod` line:

```rust
mod memory_store_eviction_tests;
```

- [ ] **Step 3: Run tests to confirm they fail**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_store_eviction_tests
```

Expected: compilation error — `purge_expired` method not found on `MemoryStore`.

- [ ] **Step 4: Add `rebuild_indices` and `purge_expired` to MemoryStore**

In `src/memory_store.rs`, add the following two methods inside the `impl<B: MemoryBackend> MemoryStore<B>` block. Place them directly after the `load` method (around line 280):

```rust
    /// Rebuild all positional indices from scratch after structural Vec changes.
    /// Called after any retain/remove operation to keep index_actor / index_action /
    /// index_target consistent with the current Vec positions.
    fn rebuild_indices(&mut self) {
        self.index_actor.clear();
        self.index_action.clear();
        self.index_target.clear();
        for (i, rec) in self.records.iter().enumerate() {
            self.index_actor
                .entry(rec.actor.clone())
                .or_default()
                .push(i);
            self.index_action
                .entry(rec.action.clone())
                .or_default()
                .push(i);
            self.index_target
                .entry(rec.target.clone())
                .or_default()
                .push(i);
        }
    }

    /// Remove all records whose `expires_at` is in the past.
    /// Rebuilds indices if any records were removed.
    /// Returns the number of records removed.
    pub fn purge_expired(&mut self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let before = self.records.len();
        self.records
            .retain(|r| r.expires_at.map_or(true, |exp| exp > now));
        let removed = before - self.records.len();
        if removed > 0 {
            self.rebuild_indices();
        }
        removed
    }
```

- [ ] **Step 5: Run tests to confirm they pass**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_store_eviction_tests
```

Expected: 4 tests pass. No warnings about unused variables.

- [ ] **Step 6: Add background eviction thread to web server**

In `src/web_server.rs`, locate the block comment `// G10: Background CoherenceChecker` (near the bottom of `run_with_state`). Add the eviction spawn **before** it:

```rust
    // G11: Background TTL eviction — purges expired records every 5 minutes.
    // This is separate from the read-time filter in query handlers, which hides
    // expired records but does not reclaim storage. This thread actually removes them.
    {
        let eviction_store = memory_store.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                if let Ok(mut ms) = eviction_store.lock() {
                    let removed = ms.purge_expired();
                    if removed > 0 {
                        eprintln!("[EvictionThread] purged {} expired records", removed);
                    }
                }
            }
        });
    }

    // G10: Background CoherenceChecker — runs check_consistency every 60s
```

- [ ] **Step 7: Build to confirm no compilation errors**

```
cargo build --no-default-features --features "petgraph_backend,web-server"
```

Expected: compiles cleanly with zero errors.

- [ ] **Step 8: Commit**

```
git add src/memory_store.rs src/web_server.rs tests/unit/memory_store_eviction_tests.rs tests/unit/mod.rs
git commit -m "feat: add purge_expired + background TTL eviction thread"
```

---

### Task 2: Fix Consolidate to Actually Execute Deletes

`POST /memory/consolidate` currently finds duplicate pairs but never deletes them. It says "use GDPR forget on drop IDs" — which means two round-trips and the caller must know the API. This task makes `dry_run=false` (the default) actually execute the deletes in the same request.

**Files:**
- Modify: `src/memory_store.rs` — add `delete_by_id(id: Uuid) -> bool`
- Modify: `src/web_server.rs` — update `handle_consolidate` to call `delete_by_id` for each drop ID
- Create: `tests/unit/memory_store_delete_tests.rs`
- Modify: `tests/unit/mod.rs` — register new test file

**Interfaces:**
- Consumes: `rebuild_indices(&mut self)` from Task 1
- Produces: `pub fn delete_by_id(&mut self, id: uuid::Uuid) -> bool` — returns `true` if found and deleted, `false` if not found

---

- [ ] **Step 1: Write the failing tests**

Create `tests/unit/memory_store_delete_tests.rs`:

```rust
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;

fn make_record(actor: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    )
}

#[test]
fn delete_by_id_removes_record() {
    let mut store = MemoryStore::new_in_memory();
    let r = make_record("agent", "task_a");
    let id = r.id;
    store.add(r).unwrap();
    assert_eq!(store.all().len(), 1);

    let deleted = store.delete_by_id(id);

    assert!(deleted, "should return true when record existed");
    assert_eq!(store.all().len(), 0);
}

#[test]
fn delete_by_id_returns_false_for_unknown_id() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_record("agent", "task_a")).unwrap();

    let deleted = store.delete_by_id(uuid::Uuid::new_v4());

    assert!(!deleted, "should return false for unknown id");
    assert_eq!(store.all().len(), 1, "existing records untouched");
}

#[test]
fn delete_by_id_rebuilds_actor_index() {
    let mut store = MemoryStore::new_in_memory();
    let r1 = make_record("alice", "task_a");
    let r2 = make_record("alice", "task_b");
    let id_r1 = r1.id;
    store.add(r1).unwrap();
    store.add(r2).unwrap();

    store.delete_by_id(id_r1);

    let alice_records = store.find_by_actor("alice");
    assert_eq!(alice_records.len(), 1, "actor index must be consistent after delete");
    assert_eq!(alice_records[0].target, "task_b");
}

#[test]
fn delete_by_id_does_not_touch_other_records() {
    let mut store = MemoryStore::new_in_memory();
    let r1 = make_record("agent", "keep_me");
    let r2 = make_record("agent", "delete_me");
    let id_r2 = r2.id;
    store.add(r1).unwrap();
    store.add(r2).unwrap();

    store.delete_by_id(id_r2);

    assert_eq!(store.all().len(), 1);
    assert_eq!(store.all()[0].target, "keep_me");
}
```

- [ ] **Step 2: Register the test file**

In `tests/unit/mod.rs`, add:

```rust
mod memory_store_delete_tests;
```

- [ ] **Step 3: Run tests to confirm they fail**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_store_delete_tests
```

Expected: compilation error — `delete_by_id` not found.

- [ ] **Step 4: Add `delete_by_id` to MemoryStore**

In `src/memory_store.rs`, add directly after `purge_expired` (inside `impl<B: MemoryBackend> MemoryStore<B>`):

```rust
    /// Remove the single record with the given `id`.
    /// Rebuilds indices if a record was deleted.
    /// Returns `true` if a record was found and removed, `false` if not found.
    pub fn delete_by_id(&mut self, id: uuid::Uuid) -> bool {
        let before = self.records.len();
        self.records.retain(|r| r.id != id);
        let removed = before - self.records.len();
        if removed > 0 {
            self.rebuild_indices();
            true
        } else {
            false
        }
    }
```

- [ ] **Step 5: Run tests to confirm they pass**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_store_delete_tests
```

Expected: 4 tests pass.

- [ ] **Step 6: Fix `handle_consolidate` to execute deletes**

In `src/web_server.rs`, find `handle_consolidate`. The current function signature is:

```rust
async fn handle_consolidate<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Query(params): Query<ConsolidateParams>,
) -> Json<serde_json::Value> {
```

Replace the entire function body (from `let threshold = ...` to the closing `}` of the outer `match`) with:

```rust
async fn handle_consolidate<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Query(params): Query<ConsolidateParams>,
) -> Json<serde_json::Value> {
    let threshold = params.threshold.unwrap_or(0.80).clamp(0.0, 1.0);
    let dry_run = params.dry_run.unwrap_or(false);

    match store.lock() {
        Err(e) => Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
        Ok(mut ms) => {
            let records = ms.all().to_vec(); // clone to release borrow before mutations
            let candidates: Vec<_> = records
                .iter()
                .filter(|r| params.actor.as_ref().map_or(true, |a| &r.actor == a))
                .collect();

            // Find near-duplicate pairs by Jaccard token similarity on `.target`
            let mut pairs: Vec<(String, String, u32)> = Vec::new(); // (keep_id, drop_id, pct)
            let mut drop_set: std::collections::HashSet<String> = std::collections::HashSet::new();
            for i in 0..candidates.len() {
                for j in (i + 1)..candidates.len() {
                    // Skip records already marked for dropping
                    if drop_set.contains(&candidates[j].id.to_string()) {
                        continue;
                    }
                    let words_i: std::collections::HashSet<&str> =
                        candidates[i].target.split_whitespace().collect();
                    let words_j: std::collections::HashSet<&str> =
                        candidates[j].target.split_whitespace().collect();
                    if words_i.is_empty() || words_j.is_empty() {
                        continue;
                    }
                    let intersection = words_i.intersection(&words_j).count();
                    let sim = intersection as f64
                        / words_i.len().max(words_j.len()) as f64;
                    if sim >= threshold {
                        // Keep newer; drop older
                        let (keep, drop) = if candidates[i].timestamp >= candidates[j].timestamp {
                            (candidates[i].id.to_string(), candidates[j].id.to_string())
                        } else {
                            (candidates[j].id.to_string(), candidates[i].id.to_string())
                        };
                        drop_set.insert(drop.clone());
                        pairs.push((keep, drop, (sim * 100.0) as u32));
                    }
                }
            }

            let found = pairs.len();
            let mut deleted = 0usize;

            if !dry_run && !pairs.is_empty() {
                for (_, drop_id, _) in &pairs {
                    if let Ok(uuid) = uuid::Uuid::parse_str(drop_id) {
                        if ms.delete_by_id(uuid) {
                            deleted += 1;
                        }
                    }
                }
            }

            Json(serde_json::json!({
                "found_duplicates": found,
                "dry_run": dry_run,
                "deleted": deleted,
                "pairs": pairs.iter().map(|(k, d, s)| serde_json::json!({
                    "keep": k, "drop": d, "similarity_pct": s
                })).collect::<Vec<_>>(),
                "note": if dry_run {
                    "Dry run — no changes made. Re-run without ?dry_run=true to execute."
                } else {
                    "Duplicates deleted. Re-run with ?dry_run=true to preview without changes."
                }
            }))
        }
    }
}
```

- [ ] **Step 7: Build to confirm no compilation errors**

```
cargo build --no-default-features --features "petgraph_backend,web-server"
```

Expected: zero errors.

- [ ] **Step 8: Commit**

```
git add src/memory_store.rs src/web_server.rs tests/unit/memory_store_delete_tests.rs tests/unit/mod.rs
git commit -m "feat: add delete_by_id + fix consolidate to actually execute deletes"
```

---

### Task 3: Memory↔Memory Graph Edges

`MemoryRecord`s are currently isolated — there's no way to express "this decision caused that outcome" or "memory A supports memory B". The codebase already has a `CausalTopoGraph` in `src/modules/topological_memory/graph.rs` with cycle detection and `Supports`/`Causal`/`Temporal` edge types. This task bridges `MemoryRecord` UUIDs into that graph (convention: `symbolic_id = "mem-{uuid}"`) and exposes two REST endpoints.

**Files:**
- Modify: `src/modules/topological_memory/graph.rs` — add `get_neighbors(symbolic_id) -> Vec<String>` and `get_incoming(symbolic_id) -> Vec<String>`
- Modify: `src/web_server.rs` — add `topo_graph` field to `AppState`, two request/response structs, two handler functions, and two routes
- Create: `tests/unit/memory_graph_tests.rs`
- Modify: `tests/unit/mod.rs` — register new test file

**Interfaces:**
- Consumes: `CausalTopoGraph::add_node`, `CausalTopoGraph::add_edge`, `EdgeType` (all existing in `src/modules/topological_memory/graph.rs`)
- Produces:
  - `pub fn get_neighbors(symbolic_id: &str) -> Vec<String>` — outgoing neighbors
  - `pub fn get_incoming(symbolic_id: &str) -> Vec<String>` — incoming neighbors
  - `POST /memory/link` — body `{"from_id":"<uuid>","to_id":"<uuid>","relation":"<str>"}`
  - `GET /memory/neighbors/:id` — returns `{neighbors: [...], incoming: [...]}`

---

- [ ] **Step 1: Write the failing tests**

Create `tests/unit/memory_graph_tests.rs`:

```rust
use hipcortex::topological_memory::{CausalTopoGraph, EdgeType};
use std::collections::HashMap;

fn blank_embedding() -> [f32; 128] {
    [0.0f32; 128]
}

#[test]
fn get_neighbors_returns_outgoing_neighbors() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-aaa".into(), blank_embedding(), HashMap::new()).unwrap();
    g.add_node("mem-bbb".into(), blank_embedding(), HashMap::new()).unwrap();
    g.add_edge("mem-aaa".into(), "mem-bbb".into(), EdgeType::Supports, 1.0, 1.0)
        .unwrap();

    let neighbors = g.get_neighbors("mem-aaa");

    assert_eq!(neighbors, vec!["mem-bbb".to_string()]);
}

#[test]
fn get_incoming_returns_incoming_neighbors() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-aaa".into(), blank_embedding(), HashMap::new()).unwrap();
    g.add_node("mem-bbb".into(), blank_embedding(), HashMap::new()).unwrap();
    g.add_edge("mem-aaa".into(), "mem-bbb".into(), EdgeType::Causal, 1.0, 1.0)
        .unwrap();

    let incoming = g.get_incoming("mem-bbb");

    assert_eq!(incoming, vec!["mem-aaa".to_string()]);
}

#[test]
fn get_neighbors_returns_empty_for_unknown_node() {
    let g = CausalTopoGraph::new();

    let neighbors = g.get_neighbors("mem-unknown");

    assert!(neighbors.is_empty());
}

#[test]
fn add_edge_rejects_cycle() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-a".into(), blank_embedding(), HashMap::new()).unwrap();
    g.add_node("mem-b".into(), blank_embedding(), HashMap::new()).unwrap();
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Causal, 1.0, 1.0)
        .unwrap();

    let result = g.add_edge("mem-b".into(), "mem-a".into(), EdgeType::Causal, 1.0, 1.0);

    assert!(result.is_err(), "reverse edge creating a cycle should be rejected");
}

#[test]
fn multiple_neighbors_returned() {
    let mut g = CausalTopoGraph::new();
    for id in ["mem-root", "mem-child1", "mem-child2"] {
        g.add_node(id.into(), blank_embedding(), HashMap::new()).unwrap();
    }
    g.add_edge("mem-root".into(), "mem-child1".into(), EdgeType::Temporal, 1.0, 1.0)
        .unwrap();
    g.add_edge("mem-root".into(), "mem-child2".into(), EdgeType::Supports, 1.0, 1.0)
        .unwrap();

    let mut neighbors = g.get_neighbors("mem-root");
    neighbors.sort(); // order not guaranteed

    assert_eq!(neighbors, vec!["mem-child1".to_string(), "mem-child2".to_string()]);
}
```

- [ ] **Step 2: Register the test file**

In `tests/unit/mod.rs`, add:

```rust
mod memory_graph_tests;
```

- [ ] **Step 3: Run tests to confirm they fail**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_graph_tests
```

Expected: compilation error — `get_neighbors` and `get_incoming` not found.

- [ ] **Step 4: Add `get_neighbors` and `get_incoming` to CausalTopoGraph**

In `src/modules/topological_memory/graph.rs`, add the following two methods inside `impl CausalTopoGraph`, after `extract_localized_subgraph`:

```rust
    /// Return the `symbolic_id`s of all outgoing (downstream) neighbors.
    /// Returns empty Vec if `symbolic_id` is not in the graph.
    pub fn get_neighbors(&self, symbolic_id: &str) -> Vec<String> {
        match self.id_map.get(symbolic_id) {
            None => vec![],
            Some(&idx) => self
                .graph
                .neighbors(idx)
                .filter_map(|n| self.graph.node_weight(n))
                .map(|n| n.symbolic_id.clone())
                .collect(),
        }
    }

    /// Return the `symbolic_id`s of all incoming (upstream) neighbors.
    /// Returns empty Vec if `symbolic_id` is not in the graph.
    pub fn get_incoming(&self, symbolic_id: &str) -> Vec<String> {
        match self.id_map.get(symbolic_id) {
            None => vec![],
            Some(&idx) => self
                .graph
                .neighbors_directed(idx, petgraph::Direction::Incoming)
                .filter_map(|n| self.graph.node_weight(n))
                .map(|n| n.symbolic_id.clone())
                .collect(),
        }
    }
```

- [ ] **Step 5: Run tests to confirm they pass**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_graph_tests
```

Expected: 5 tests pass.

- [ ] **Step 6: Add `topo_graph` to AppState**

In `src/web_server.rs`, find `pub struct AppState<B: ...>`. Add the new field as the last field:

```rust
#[cfg(feature = "web-server")]
pub struct AppState<B: MemoryBackend + Send + Sync + 'static> {
    pub memory_store:   Arc<Mutex<MemoryStore<B>>>,
    pub symbolic_store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
    pub world_model:    Arc<RwLock<WorldModelEnhanced>>,
    pub aureus:         Arc<Mutex<AureusBridge>>,
    pub self_model:     Arc<SelfModel>,
    pub coherence:      Arc<CoherenceChecker>,
    /// Memory-to-memory relational graph. Nodes use "mem-{uuid}" as symbolic_id.
    pub topo_graph:     Arc<Mutex<crate::topological_memory::CausalTopoGraph>>,
}
```

Update the `Clone` impl for `AppState` by adding the field:

```rust
    fn clone(&self) -> Self {
        Self {
            memory_store:   self.memory_store.clone(),
            symbolic_store: self.symbolic_store.clone(),
            world_model:    self.world_model.clone(),
            aureus:         self.aureus.clone(),
            self_model:     self.self_model.clone(),
            coherence:      self.coherence.clone(),
            topo_graph:     self.topo_graph.clone(),
        }
    }
```

Update `run_with_memory` to initialize the field:

```rust
#[cfg(feature = "web-server")]
pub async fn run_with_memory<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr,
    memory_store: Arc<Mutex<MemoryStore<B>>>,
) {
    let state = AppState {
        memory_store,
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model:    Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus:         Arc::new(Mutex::new(AureusBridge::new())),
        self_model:     Arc::new(SelfModel::new()),
        coherence:      Arc::new(CoherenceChecker::new()),
        topo_graph:     Arc::new(Mutex::new(crate::topological_memory::CausalTopoGraph::new())),
    };
    run_with_state(addr, state).await;
}
```

- [ ] **Step 7: Add request/response structs**

In `src/web_server.rs`, add after the existing `CreateEdgeRequest` struct:

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

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct MemoryNeighborsResponse {
    pub id:           String,
    /// Outgoing edges ("this memory supports/caused/follows these")
    pub neighbors:    Vec<String>,
    /// Incoming edges ("these memories support/caused/led to this one")
    pub incoming:     Vec<String>,
}
```

- [ ] **Step 8: Add handler functions**

In `src/web_server.rs`, add both handlers after `handle_delete_node`:

```rust
/// POST /memory/link — link two MemoryRecords with a typed directed edge.
/// Uses "mem-{uuid}" convention in CausalTopoGraph.
/// Rejects cycles (causal DAG invariant preserved by CausalTopoGraph::add_edge).
/// Returns 404 if either record UUID does not exist in the memory store.
#[cfg(feature = "web-server")]
async fn handle_memory_link<B: MemoryBackend + Send + Sync + 'static>(
    memory_store: Arc<Mutex<MemoryStore<B>>>,
    topo: Arc<Mutex<crate::topological_memory::CausalTopoGraph>>,
    Json(req): Json<MemoryLinkRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let from_uuid = uuid::Uuid::parse_str(&req.from_id).map_err(|_| (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"success": false, "error": "invalid from_id UUID"})),
    ))?;
    let to_uuid = uuid::Uuid::parse_str(&req.to_id).map_err(|_| (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"success": false, "error": "invalid to_id UUID"})),
    ))?;

    // Validate both records exist
    {
        let ms = memory_store.lock().map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
        ))?;
        if ms.find_by_id(from_uuid).is_none() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "error": "from_id not found"})),
            ));
        }
        if ms.find_by_id(to_uuid).is_none() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "error": "to_id not found"})),
            ));
        }
    }

    let from_sym = format!("mem-{}", req.from_id);
    let to_sym   = format!("mem-{}", req.to_id);

    let edge_type = match req.relation.as_str() {
        "caused_by" | "causal" => crate::topological_memory::EdgeType::Causal,
        "follows"   | "temporal" => crate::topological_memory::EdgeType::Temporal,
        "contradicts" | "taxonomic" => crate::topological_memory::EdgeType::Taxonomic,
        _ => crate::topological_memory::EdgeType::Supports, // default for "supports" and unknown
    };

    let mut tg = topo.lock().map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
    ))?;

    // add_node is idempotent — returns Err("exists") which we ignore
    let _ = tg.add_node(from_sym.clone(), [0.0f32; 128], std::collections::HashMap::new());
    let _ = tg.add_node(to_sym.clone(), [0.0f32; 128], std::collections::HashMap::new());

    match tg.add_edge(from_sym, to_sym, edge_type, 1.0, 1.0) {
        Ok(_) => Ok(Json(serde_json::json!({"success": true}))),
        Err(e) => Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"success": false, "error": e})),
        )),
    }
}

/// GET /memory/neighbors/:id — return outgoing and incoming linked records for a MemoryRecord.
#[cfg(feature = "web-server")]
async fn handle_memory_neighbors(
    topo: Arc<Mutex<crate::topological_memory::CausalTopoGraph>>,
    id: String,
) -> Json<MemoryNeighborsResponse> {
    let sym_id = format!("mem-{}", id);
    match topo.lock() {
        Err(_) => Json(MemoryNeighborsResponse {
            id,
            neighbors: vec![],
            incoming: vec![],
        }),
        Ok(tg) => {
            // Strip "mem-" prefix to return bare UUIDs
            let neighbors: Vec<String> = tg
                .get_neighbors(&sym_id)
                .into_iter()
                .map(|s| s.trim_start_matches("mem-").to_string())
                .collect();
            let incoming: Vec<String> = tg
                .get_incoming(&sym_id)
                .into_iter()
                .map(|s| s.trim_start_matches("mem-").to_string())
                .collect();
            Json(MemoryNeighborsResponse { id, neighbors, incoming })
        }
    }
}
```

- [ ] **Step 9: Wire routes and state in `run_with_state`**

In `src/web_server.rs`, inside `run_with_state`, add after the `let coherence_arc = state.coherence.clone();` line:

```rust
    let topo_arc = state.topo_graph.clone();
```

Then add the two route closures after the `consolidate_route` closure:

```rust
    let memory_link_route = {
        let ms  = memory_store.clone();
        let tg  = topo_arc.clone();
        post(move |Json(req): Json<MemoryLinkRequest>| async move {
            handle_memory_link(ms, tg, Json(req)).await
        })
    };
    let memory_neighbors_route = {
        let tg = topo_arc.clone();
        get(move |Path(id): Path<String>| async move {
            handle_memory_neighbors(tg, id).await
        })
    };
```

Add the routes to the `Router::new()` chain, after `.route("/memory/consolidate", consolidate_route)`:

```rust
        .route("/memory/link",            memory_link_route)
        .route("/memory/neighbors/:id",   memory_neighbors_route)
```

Also initialize `topo_graph` in the `AppState` struct literal inside `run_with_state`. Find where `AppState {` is built (the call to `run_with_state` comes from `run_with_memory`, which was updated in Step 6). Verify the `state` passed in already has `topo_graph` via the test setup pattern — if writing an integration test you will initialize it as:

```rust
let state = AppState {
    memory_store:   Arc::new(Mutex::new(MemoryStore::new_in_memory())),
    symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
    world_model:    Arc::new(RwLock::new(WorldModelEnhanced::new())),
    aureus:         Arc::new(Mutex::new(AureusBridge::new())),
    self_model:     Arc::new(SelfModel::new()),
    coherence:      Arc::new(CoherenceChecker::new()),
    topo_graph:     Arc::new(Mutex::new(CausalTopoGraph::new())),
};
```

- [ ] **Step 10: Build to confirm no compilation errors**

```
cargo build --no-default-features --features "petgraph_backend,web-server"
```

Expected: zero errors. If you see errors about missing `find_by_id`, verify `find_by_id` is public in `src/memory_store.rs` — it is used by the existing `handle_update_memory` handler so it already exists.

- [ ] **Step 11: Run all new tests**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_graph_tests
```

Expected: 5 tests pass.

- [ ] **Step 12: Run full unit suite to confirm no regressions**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite
```

Expected: all previously passing tests still pass, 14 new tests added (4 eviction + 4 delete + 5 graph + 1 existing).

- [ ] **Step 13: Commit**

```
git add src/modules/topological_memory/graph.rs src/web_server.rs tests/unit/memory_graph_tests.rs tests/unit/mod.rs
git commit -m "feat: memory-to-memory graph edges via CausalTopoGraph (POST /memory/link, GET /memory/neighbors/:id)"
```

---

## Self-Review

### Spec Coverage

| Gap from audit | Task | Addressed |
|---|---|---|
| TTL eviction: records accumulate forever | Task 1 | ✓ `purge_expired` + background thread |
| `ms.all()` returns expired records to stats/export | Task 1 | Not addressed — out of scope (cosmetic, not behavioral) |
| Consolidate finds-but-never-deletes | Task 2 | ✓ `delete_by_id` + handler fixed |
| Memory↔memory edges: zero | Task 3 | ✓ `/memory/link` + `/memory/neighbors` |
| CausalTopoGraph has no REST surface | Task 3 | ✓ bridged via two endpoints |
| `topological_memory/search.rs` is a TODO stub | Not addressed | Out of scope for this plan |
| Coherence score always 1.0 | Not addressed | Separate concern |
| Global write lock for multi-agent | Not addressed | Separate concern |

Two items remain out of scope from this plan: `ms.all()` expiry filter (minor, stats-only) and `topological_memory/search.rs` implementation (PPR traversal — warrants its own plan).

### Placeholder Scan

No TBD, no "handle edge cases without code", no forward references to undefined types. Every step contains the actual code.

### Type Consistency

- `purge_expired` → `usize`: used only in background thread (no downstream tasks consume it)
- `delete_by_id(id: uuid::Uuid) -> bool`: used in `handle_consolidate` and tested directly
- `get_neighbors(&self, symbolic_id: &str) -> Vec<String>`: used in `handle_memory_neighbors`
- `get_incoming(&self, symbolic_id: &str) -> Vec<String>`: used in `handle_memory_neighbors`
- `MemoryLinkRequest.relation: String` → maps to `EdgeType` via match in handler
- Route paths: `/memory/link` (POST), `/memory/neighbors/:id` (GET) — consistent with naming conventions in codebase
