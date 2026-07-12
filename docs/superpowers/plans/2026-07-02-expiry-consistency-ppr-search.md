# Memory Expiry Consistency + PPR Graph Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix expiry-filter inconsistency across export/query/stats/consolidate endpoints (including a consolidate data-loss bug), and implement Personalized PageRank graph traversal exposed via `GET /memory/search/related`.

**Architecture:** Two independent changes. (1) Expiry consistency: add `include_expired: Option<String>` to the shared `QueryMemoryParams` struct — exactly matching the existing `include_quarantined` pattern — then apply a consistent expiry filter in export and stats, and fix the data-loss path in consolidate where an expired but newer-timestamped record could cause a live record to be deleted. (2) PPR search: implement `ppr()` as a method on `CausalTopoGraph` in `graph.rs` (private fields stay private), then expose `GET /memory/search/related?seed_id=&limit=` in the web server with α=0.85 fixed internally and `limit` as the only caller-tunable parameter.

**Tech Stack:** Rust 2021 edition, Axum 0.6, petgraph (existing — `DiGraph`, `NodeIndex`, `edges`), chrono (existing), tokio (existing). No new dependencies.

## Global Constraints

- Build: `cargo build --no-default-features --features "petgraph_backend"`
- Unit test suite: `cargo test --no-default-features --features "petgraph_backend" --test unit_suite`
- Web-server build: `cargo build --no-default-features --features "petgraph_backend,web-server"`
- New test files go in `tests/unit/` and MUST be registered in `tests/unit/mod.rs` — the suite will silently ignore unregistered files
- `MemoryStore::new_in_memory()` returns `Self` (not `Result<Self>`) — no `.unwrap()` needed on construction
- FileBackend tests create a `.jsonl` file, use it, then delete it — follow pattern in `tests/unit/memory_store_tests.rs`
- No pre-existing cleanup, no unrelated refactors; surgical changes only
- **Task 3 (PPR endpoint) prerequisite:** `AppState.topo_graph: Arc<Mutex<CausalTopoGraph>>` must exist before Task 3. This field is added by tiered-memory-foundation Task 3. Run `grep -n "topo_graph" src/web_server.rs` before starting Task 3 of this plan. If the field is absent, implement tiered-memory-foundation Task 3 first.

---

### Coordination Note

If the tiered-memory-foundation plan's Task 2 (`handle_consolidate` rewrite) has already been applied before this plan runs, apply the consolidate expiry fix (Task 1 Step 7) to the **new** implementation from that task instead of the original. The fix is identical: filter candidates by expiry before finding pairs. The field is the same, the logic is the same — only the surrounding code context differs.

---

### Task 1: Expiry Consistency Across Export, Query, Stats, Consolidate

Adds `include_expired: Option<String>` to `QueryMemoryParams` (shared by both `/memory/query` and `/memory/export`). Fixes four handlers to use consistent expiry semantics: export now excludes expired by default (same as query/search), query honors the new flag, stats gains an `active_records` field alongside `total_records`, and consolidate filters candidates before pair-finding to eliminate the data-loss bug where an expired but recently-written record could be ranked as "keep" and cause a live record to be deleted.

**Files:**
- Modify: `src/web_server.rs` — 5 surgical changes (struct field, 4 handler fixes)
- Create: `tests/unit/memory_expiry_consistency_tests.rs`
- Modify: `tests/unit/mod.rs`

**Interfaces:**
- Produces: `QueryMemoryParams.include_expired: Option<String>` — honored by export and query handlers; defaults to exclude expired
- Produces: `StatsResponse.active_records: usize` — count of non-expired records

---

- [ ] **Step 1: Write the failing tests**

Create `tests/unit/memory_expiry_consistency_tests.rs`:

```rust
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;

fn make_expired(actor: &str, target: &str) -> MemoryRecord {
    let mut r = MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    );
    r.expires_at = Some(chrono::Utc::now().timestamp() - 100); // 100 seconds in the past
    r
}

fn make_live(actor: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    )
    // expires_at = None → never expires
}

// ── Export expiry filter ──────────────────────────────────────────────────────

#[test]
fn export_logic_excludes_expired_by_default() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_expired("agent", "expired_mem")).unwrap();
    store.add(make_live("agent", "live_mem")).unwrap();

    let now_ts = chrono::Utc::now().timestamp();
    // Simulate include_expired = false (default behavior after the fix)
    let active: Vec<_> = store
        .all()
        .iter()
        .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
        .collect();

    assert_eq!(active.len(), 1, "default export should exclude expired records");
    assert_eq!(active[0].target, "live_mem");
}

#[test]
fn export_logic_includes_expired_when_flag_set() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_expired("agent", "expired_mem")).unwrap();
    store.add(make_live("agent", "live_mem")).unwrap();

    // Simulate include_expired = true: no expiry filter applied
    let all = store.all();
    assert_eq!(all.len(), 2, "include_expired=true should return all records");
}

// ── Consolidate data-loss fix ─────────────────────────────────────────────────

#[test]
fn consolidate_candidates_skip_expired_records() {
    let mut store = MemoryStore::new_in_memory();

    // Expired record with similar text AND a newer write timestamp — the data-loss scenario:
    // without the expiry filter, this expired record would be ranked "keep" and the live
    // record "drop", deleting a valid memory.
    let mut expired = make_expired("agent", "use postgres for auth");
    expired.timestamp = chrono::Utc::now(); // mark as newly written but short-lived

    let live = make_live("agent", "use postgres for users");

    store.add(expired).unwrap();
    store.add(live).unwrap();

    let now_ts = chrono::Utc::now().timestamp();
    // Simulate the fixed consolidate candidate filter
    let records = store.all();
    let candidates: Vec<_> = records
        .iter()
        .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
        .collect();

    assert_eq!(candidates.len(), 1, "expired record must be excluded from consolidate candidates");
    assert_eq!(candidates[0].target, "use postgres for users");
}

// ── Stats active/total split ──────────────────────────────────────────────────

#[test]
fn stats_active_records_excludes_expired() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_expired("agent", "exp1")).unwrap();
    store.add(make_expired("agent", "exp2")).unwrap();
    store.add(make_live("agent", "live1")).unwrap();

    let now_ts = chrono::Utc::now().timestamp();
    let total = store.all().len();
    let active = store
        .all()
        .iter()
        .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
        .count();

    assert_eq!(total, 3, "total_records counts all including expired");
    assert_eq!(active, 1, "active_records counts only non-expired");
    assert!(total > active);
}

#[test]
fn stats_active_equals_total_when_no_ttl() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_live("agent", "a")).unwrap();
    store.add(make_live("agent", "b")).unwrap();

    let now_ts = chrono::Utc::now().timestamp();
    let total = store.all().len();
    let active = store
        .all()
        .iter()
        .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
        .count();

    assert_eq!(total, active, "no TTL records: active must equal total");
}
```

- [ ] **Step 2: Register the test file**

In `tests/unit/mod.rs`, add after the last `mod` line:

```rust
mod memory_expiry_consistency_tests;
```

- [ ] **Step 3: Run tests to confirm they pass as written**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_expiry_consistency_tests
```

Expected: all 5 tests **PASS** — these tests exercise logic inline (not handlers), so they should already be correct. If any fails, the logic assumption is wrong; do not proceed until you understand why.

- [ ] **Step 4: Add `include_expired` to `QueryMemoryParams`**

In `src/web_server.rs`, find the `QueryMemoryParams` struct. It currently ends with:

```rust
    /// If "true", include quarantined records. Default: exclude quarantine.
    include_quarantined: Option<String>,
}
```

Replace with:

```rust
    /// If "true", include quarantined records. Default: exclude quarantine.
    include_quarantined: Option<String>,
    /// If "true", include records whose `expires_at` is in the past.
    /// Default: exclude expired (consistent with /memory/query and /memory/search).
    /// Use ?include_expired=true for full backup/migration exports.
    include_expired: Option<String>,
}
```

- [ ] **Step 5: Fix `handle_export_memory` to filter expired by default**

In `src/web_server.rs`, find `handle_export_memory`. The current filter block is:

```rust
            let records = ms.all();
            let filtered: Vec<_> = records.iter().filter(|r| {
                params.actor.as_ref().map_or(true, |a| &r.actor == a)
            }).collect();
```

Replace with:

```rust
            let records = ms.all();
            let now_ts = chrono::Utc::now().timestamp();
            let include_expired = params.include_expired.as_deref() == Some("true");
            let filtered: Vec<_> = records.iter().filter(|r| {
                params.actor.as_ref().map_or(true, |a| &r.actor == a)
                    && (include_expired || r.expires_at.map_or(true, |exp| exp > now_ts))
            }).collect();
```

- [ ] **Step 6: Fix `handle_query_memory` to honor `include_expired`**

In `src/web_server.rs`, find `handle_query_memory`. It contains a hardcoded expiry block:

```rust
            // Exclude records past their TTL
            let now_ts = chrono::Utc::now().timestamp();
            filtered_records.retain(|r| {
                r.expires_at.map_or(true, |exp| exp > now_ts)
            });
```

Replace with:

```rust
            // Exclude records past their TTL (unless ?include_expired=true for debugging)
            let now_ts = chrono::Utc::now().timestamp();
            let include_expired = params.include_expired.as_deref() == Some("true");
            if !include_expired {
                filtered_records.retain(|r| r.expires_at.map_or(true, |exp| exp > now_ts));
            }
```

- [ ] **Step 7: Fix `handle_consolidate` to filter expired candidates**

In `src/web_server.rs`, find `handle_consolidate`. The current candidate-building block is:

```rust
            let records = ms.all();
            let candidates: Vec<_> = records.iter()
                .filter(|r| params.actor.as_ref().map_or(true, |a| &r.actor == a))
                .collect();
```

Replace with:

```rust
            let records = ms.all();
            let now_ts_consolidate = chrono::Utc::now().timestamp();
            let candidates: Vec<_> = records
                .iter()
                .filter(|r| params.actor.as_ref().map_or(true, |a| &r.actor == a))
                .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts_consolidate))
                .collect();
```

(Using `now_ts_consolidate` avoids any name collision with `now_ts` in the same function if one exists. If the function already has `let now_ts = ...`, rename this to match or reuse it.)

- [ ] **Step 8: Fix `handle_stats` to add `active_records`**

In `src/web_server.rs`, find the private `StatsResponse` struct (just before `AuditVerifyResponse`):

```rust
#[cfg(feature = "web-server")]
#[derive(Serialize)]
struct StatsResponse {
    total_records: usize,
    by_type: HashMap<String, usize>,
    unique_actors: usize,
    metering_enabled: bool,
    tier_counts: HashMap<String, u64>,
}
```

Replace with:

```rust
#[cfg(feature = "web-server")]
#[derive(Serialize)]
struct StatsResponse {
    total_records: usize,
    /// Records whose `expires_at` is None or in the future. Always ≤ total_records.
    active_records: usize,
    by_type: HashMap<String, usize>,
    unique_actors: usize,
    metering_enabled: bool,
    tier_counts: HashMap<String, u64>,
}
```

In `handle_stats`, find:

```rust
    let (total_records, by_type, unique_actors) = match store.lock() {
        Ok(ms) => {
            let records = ms.all();
            let total = records.len();
            let mut by_type: HashMap<String, usize> = HashMap::new();
            let mut actors: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for r in records {
                *by_type.entry(format!("{:?}", r.record_type)).or_insert(0) += 1;
                actors.insert(&r.actor);
            }
            (total, by_type, actors.len())
        }
        Err(_) => (0, HashMap::new(), 0),
    };
```

Replace with:

```rust
    let (total_records, active_records, by_type, unique_actors) = match store.lock() {
        Ok(ms) => {
            let records = ms.all();
            let total = records.len();
            let now_ts = chrono::Utc::now().timestamp();
            let active = records
                .iter()
                .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
                .count();
            let mut by_type: HashMap<String, usize> = HashMap::new();
            let mut actors: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for r in records {
                *by_type.entry(format!("{:?}", r.record_type)).or_insert(0) += 1;
                actors.insert(&r.actor);
            }
            (total, active, by_type, actors.len())
        }
        Err(_) => (0, 0, HashMap::new(), 0),
    };
```

Then find the `StatsResponse` construction line:

```rust
    Json(StatsResponse { total_records, by_type, unique_actors, metering_enabled, tier_counts })
```

Replace with:

```rust
    Json(StatsResponse { total_records, active_records, by_type, unique_actors, metering_enabled, tier_counts })
```

- [ ] **Step 9: Build to confirm no compilation errors**

```
cargo build --no-default-features --features "petgraph_backend,web-server"
```

Expected: zero errors.

- [ ] **Step 10: Run full unit suite for regressions**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite
```

Expected: all previously passing tests still pass.

- [ ] **Step 11: Commit**

```
git add src/web_server.rs tests/unit/memory_expiry_consistency_tests.rs tests/unit/mod.rs
git commit -m "fix: expiry consistency — include_expired param, consolidate data-loss bug, stats active_records"
```

---

### Task 2: PPR Implementation on CausalTopoGraph

Implements `pub fn ppr()` directly on `CausalTopoGraph` in `graph.rs`. This keeps the petgraph internals private. The algorithm is Personalized PageRank via power iteration: build edge-weight-normalized adjacency, initialize score at the seed node, diffuse with restart for 20 rounds, return top-k results sorted descending. Edge weight = `strength × confidence`. This task has no dependency on any other plan — it is pure graph math.

**Files:**
- Modify: `src/modules/topological_memory/graph.rs` — add `ppr()` method
- Create: `tests/unit/memory_ppr_tests.rs`
- Modify: `tests/unit/mod.rs`

**Interfaces:**
- Produces: `pub fn ppr(&self, seed_id: &str, limit: usize, alpha: f64, iterations: usize) -> Vec<(String, f64)>`
  - Returns `(symbolic_id, score)` sorted descending, seed excluded, length ≤ `limit`
  - Empty vec if seed not in graph, graph is empty, or no reachable nodes

---

- [ ] **Step 1: Write the failing tests**

Create `tests/unit/memory_ppr_tests.rs`:

```rust
use hipcortex::topological_memory::{CausalTopoGraph, EdgeType};
use std::collections::HashMap;

fn blank() -> [f32; 128] {
    [0.0f32; 128]
}

#[test]
fn ppr_returns_empty_for_unknown_seed() {
    let g = CausalTopoGraph::new();
    assert!(g.ppr("mem-unknown", 10, 0.85, 20).is_empty());
}

#[test]
fn ppr_returns_empty_for_isolated_node() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-a".into(), blank(), HashMap::new()).unwrap();
    g.add_node("mem-b".into(), blank(), HashMap::new()).unwrap();
    // mem-b exists but has no edge to/from mem-a
    assert!(
        g.ppr("mem-a", 10, 0.85, 20).is_empty(),
        "node with no outgoing edges must return empty"
    );
}

#[test]
fn ppr_direct_neighbor_appears_in_results() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-a".into(), blank(), HashMap::new()).unwrap();
    g.add_node("mem-b".into(), blank(), HashMap::new()).unwrap();
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Supports, 1.0, 1.0).unwrap();

    let results = g.ppr("mem-a", 10, 0.85, 20);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "mem-b");
    assert!(results[0].1 > 0.0, "score must be positive");
}

#[test]
fn ppr_seed_never_in_results() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-a".into(), blank(), HashMap::new()).unwrap();
    g.add_node("mem-b".into(), blank(), HashMap::new()).unwrap();
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Supports, 1.0, 1.0).unwrap();

    let results = g.ppr("mem-a", 10, 0.85, 20);

    assert!(
        results.iter().all(|(id, _)| id != "mem-a"),
        "seed node must not appear in its own results"
    );
}

#[test]
fn ppr_direct_neighbor_ranks_above_distant_node() {
    // Chain: a → b → c → d
    let mut g = CausalTopoGraph::new();
    for id in ["mem-a", "mem-b", "mem-c", "mem-d"] {
        g.add_node(id.into(), blank(), HashMap::new()).unwrap();
    }
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Temporal, 1.0, 1.0).unwrap();
    g.add_edge("mem-b".into(), "mem-c".into(), EdgeType::Temporal, 1.0, 1.0).unwrap();
    g.add_edge("mem-c".into(), "mem-d".into(), EdgeType::Temporal, 1.0, 1.0).unwrap();

    let results = g.ppr("mem-a", 10, 0.85, 20);

    let b = results.iter().find(|(id, _)| id == "mem-b").map(|(_, s)| *s).unwrap_or(0.0);
    let d = results.iter().find(|(id, _)| id == "mem-d").map(|(_, s)| *s).unwrap_or(0.0);
    assert!(
        b > d,
        "1-hop node must score higher than 3-hop node; b={:.4} d={:.4}",
        b,
        d
    );
}

#[test]
fn ppr_respects_limit() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-root".into(), blank(), HashMap::new()).unwrap();
    for i in 1..=15 {
        let id = format!("mem-{}", i);
        g.add_node(id.clone(), blank(), HashMap::new()).unwrap();
        g.add_edge("mem-root".into(), id, EdgeType::Supports, 1.0, 1.0).unwrap();
    }

    let results = g.ppr("mem-root", 5, 0.85, 20);

    assert_eq!(results.len(), 5, "must respect limit=5 when more than 5 nodes are reachable");
}

#[test]
fn ppr_results_sorted_descending_by_score() {
    let mut g = CausalTopoGraph::new();
    for id in ["mem-a", "mem-b", "mem-c", "mem-d"] {
        g.add_node(id.into(), blank(), HashMap::new()).unwrap();
    }
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Supports, 1.0, 1.0).unwrap();
    g.add_edge("mem-b".into(), "mem-c".into(), EdgeType::Supports, 1.0, 1.0).unwrap();
    g.add_edge("mem-c".into(), "mem-d".into(), EdgeType::Supports, 1.0, 1.0).unwrap();

    let results = g.ppr("mem-a", 10, 0.85, 20);

    for window in results.windows(2) {
        assert!(
            window[0].1 >= window[1].1,
            "results must be sorted descending: {} ({:.4}) < {} ({:.4})",
            window[0].0,
            window[0].1,
            window[1].0,
            window[1].1
        );
    }
}

#[test]
fn ppr_mem_prefix_convention_strips_correctly() {
    // Verifies that ppr() returns symbolic_ids as-is and the caller strips "mem-"
    let mut g = CausalTopoGraph::new();
    let from = "mem-00000000-0000-0000-0000-000000000001";
    let to   = "mem-00000000-0000-0000-0000-000000000002";
    g.add_node(from.into(), blank(), HashMap::new()).unwrap();
    g.add_node(to.into(),   blank(), HashMap::new()).unwrap();
    g.add_edge(from.into(), to.into(), EdgeType::Supports, 1.0, 1.0).unwrap();

    let results = g.ppr(from, 10, 0.85, 20);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, to, "ppr returns full symbolic_id including mem- prefix");
    // REST handler strips "mem-" — the convention check
    assert_eq!(
        results[0].0.trim_start_matches("mem-"),
        "00000000-0000-0000-0000-000000000002"
    );
}
```

- [ ] **Step 2: Register the test file**

In `tests/unit/mod.rs`, add:

```rust
mod memory_ppr_tests;
```

- [ ] **Step 3: Run tests to confirm they fail**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_ppr_tests
```

Expected: compilation error — `ppr` method not found on `CausalTopoGraph`.

- [ ] **Step 4: Implement `ppr()` on `CausalTopoGraph`**

In `src/modules/topological_memory/graph.rs`, add the following method inside `impl CausalTopoGraph`, after the `get_incoming` method (added by tiered-memory-foundation Task 3 — if that task hasn't run, place it after `extract_localized_subgraph`):

```rust
    /// Personalized PageRank over this directed graph.
    ///
    /// Returns up to `limit` nodes (excluding seed) ranked by their PPR score —
    /// a measure of weighted graph proximity from the seed via power iteration.
    ///
    /// Edge weight = `strength × confidence`. Dangling nodes (no outgoing edges)
    /// contribute only through the teleport term.
    ///
    /// # Parameters
    /// - `seed_id`: symbolic_id of the starting node (e.g. `"mem-{uuid}"`)
    /// - `limit`: maximum number of results to return
    /// - `alpha`: damping factor — 0.85 is standard PageRank (15% restart).
    ///   Lower values concentrate scores near the seed; higher values spread further.
    ///   Fixed at 0.85 at call-sites for v1; expose as a param in a future change
    ///   if empirical evidence calls for a different default.
    /// - `iterations`: power iteration rounds; 20 is sufficient for graphs ≤ 10K nodes.
    ///
    /// Returns empty vec if seed is not in graph, graph is empty, or no
    /// nodes are reachable from the seed.
    pub fn ppr(
        &self,
        seed_id: &str,
        limit: usize,
        alpha: f64,
        iterations: usize,
    ) -> Vec<(String, f64)> {
        let node_count = self.graph.node_count();
        if node_count == 0 || limit == 0 {
            return vec![];
        }

        let seed_idx = match self.id_map.get(seed_id) {
            None     => return vec![],
            Some(&i) => i,
        };

        // Build a contiguous usize position for each NodeIndex so we can use plain Vec
        let all_indices: Vec<petgraph::graph::NodeIndex> = self.graph.node_indices().collect();
        let idx_to_pos: std::collections::HashMap<petgraph::graph::NodeIndex, usize> =
            all_indices.iter().enumerate().map(|(pos, &idx)| (idx, pos)).collect();
        let seed_pos = idx_to_pos[&seed_idx];

        // Build normalized adjacency list: adj[src_pos] = Vec<(dst_pos, weight)>
        // where weights are row-normalized so each source's outgoing weights sum to 1.
        let mut adj: Vec<Vec<(usize, f64)>> = vec![vec![]; node_count];
        for (src_pos, &src_idx) in all_indices.iter().enumerate() {
            let mut out: Vec<(usize, f64)> = self
                .graph
                .edges(src_idx)
                .filter_map(|e| {
                    let w = e.weight().strength as f64 * e.weight().confidence as f64;
                    if w > 0.0 {
                        idx_to_pos.get(&e.target()).map(|&dst| (dst, w))
                    } else {
                        None
                    }
                })
                .collect();
            let total: f64 = out.iter().map(|(_, w)| w).sum();
            if total > 0.0 {
                for (_, w) in &mut out {
                    *w /= total;
                }
                adj[src_pos] = out;
            }
            // Dangling nodes (total == 0): contribute only to teleport, handled below
        }

        // Power iteration
        let mut scores     = vec![0.0f64; node_count];
        let mut new_scores = vec![0.0f64; node_count];
        scores[seed_pos] = 1.0;

        for _ in 0..iterations {
            for s in new_scores.iter_mut() { *s = 0.0; }
            // Teleport: (1 - alpha) of all mass returns to seed
            new_scores[seed_pos] += 1.0 - alpha;
            // Diffusion: alpha * normalized adjacency * current scores
            for (src_pos, edges) in adj.iter().enumerate() {
                for &(dst_pos, w) in edges {
                    new_scores[dst_pos] += alpha * w * scores[src_pos];
                }
            }
            std::mem::swap(&mut scores, &mut new_scores);
        }

        // Collect, exclude seed, remove zero-score nodes, sort descending, truncate
        let mut results: Vec<(String, f64)> = all_indices
            .iter()
            .enumerate()
            .filter_map(|(pos, &idx)| {
                if idx == seed_idx { return None; }
                let score = scores[pos];
                if score <= 0.0 { return None; }
                self.graph.node_weight(idx).map(|n| (n.symbolic_id.clone(), score))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }
```

- [ ] **Step 5: Run tests to confirm they pass**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite memory_ppr_tests
```

Expected: 8 tests pass.

- [ ] **Step 6: Build to confirm no compilation errors**

```
cargo build --no-default-features --features "petgraph_backend"
```

Expected: zero errors. The `petgraph::graph::NodeIndex` type is already in scope since `graph.rs` imports petgraph.

- [ ] **Step 7: Commit**

```
git add src/modules/topological_memory/graph.rs tests/unit/memory_ppr_tests.rs tests/unit/mod.rs
git commit -m "feat: Personalized PageRank (ppr) on CausalTopoGraph — graph-based associative memory retrieval"
```

---

### Task 3: PPR REST Endpoint — `GET /memory/search/related`

**Prerequisite check — run this before any code changes:**

```
grep -n "topo_graph" src/web_server.rs
```

Expected: finds `pub topo_graph: Arc<Mutex<crate::topological_memory::CausalTopoGraph>>` in `AppState`. If not found, stop and implement tiered-memory-foundation Task 3 first.

Exposes the PPR algorithm as a REST endpoint. The `seed_id` is a bare MemoryRecord UUID; the handler adds the `"mem-"` prefix before calling `ppr()`. Alpha is fixed at 0.85 (internal constant, documented in source). The `limit` param is the only caller-tunable knob.

**Files:**
- Modify: `src/web_server.rs` — add params struct, handler function, route closure, route registration

**Interfaces:**
- Consumes: `CausalTopoGraph::ppr(&self, seed_id: &str, limit: usize, alpha: f64, iterations: usize)` from Task 2
- Consumes: `AppState.topo_graph: Arc<Mutex<CausalTopoGraph>>` from tiered-memory-foundation Task 3
- Produces: `GET /memory/search/related?seed_id=<uuid>&limit=<n>` → `{seed_id, related: [{id, score}], limit, algorithm, alpha}`

---

- [ ] **Step 1: Add the params struct and handler**

In `src/web_server.rs`, add the following block after the `MemoryNeighborsResponse` struct (added by tiered-memory-foundation Task 3). If that struct doesn't exist yet, add directly after the `GraphWriteResponse` struct:

```rust
/// GET /memory/search/related — find memories related to a seed by Personalized PageRank.
/// The seed must have been linked via POST /memory/link before it will appear in the graph.
/// Returns an empty `related` array (not 404) if the seed has no graph edges yet.
#[cfg(feature = "web-server")]
#[derive(serde::Deserialize)]
pub struct MemoryRelatedParams {
    /// UUID of the seed MemoryRecord (bare UUID, no "mem-" prefix)
    pub seed_id: String,
    /// Max results (default 10, capped at 50)
    pub limit: Option<usize>,
}

/// GET /memory/search/related handler.
///
/// Runs PPR (α=0.85 fixed, 20 iterations) over the CausalTopoGraph rooted at
/// "mem-{seed_id}". Strips the "mem-" prefix from results before returning.
///
/// α=0.85 is standard PageRank damping (15% restart probability). This value
/// works well for graphs up to ~10K nodes. Expose as a query param in a future
/// change if empirical evidence calls for a different default.
#[cfg(feature = "web-server")]
async fn handle_memory_search_related(
    topo: Arc<Mutex<crate::topological_memory::CausalTopoGraph>>,
    Query(params): Query<MemoryRelatedParams>,
) -> Json<serde_json::Value> {
    let limit    = params.limit.unwrap_or(10).min(50);
    let seed_sym = format!("mem-{}", params.seed_id);

    match topo.lock() {
        Err(e) => Json(serde_json::json!({
            "seed_id": params.seed_id,
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
                        "related": [],
                        "note":    "seed_id has no graph edges — link it first via POST /memory/link",
                    }));
                }
            }

            let related: Vec<serde_json::Value> = raw
                .into_iter()
                .map(|(sym_id, score)| {
                    let id = sym_id.trim_start_matches("mem-").to_string();
                    serde_json::json!({
                        "id":    id,
                        "score": (score * 1000.0).round() / 1000.0,
                    })
                })
                .collect();

            Json(serde_json::json!({
                "seed_id":   params.seed_id,
                "related":   related,
                "limit":     limit,
                "algorithm": "ppr",
                "alpha":     0.85,
            }))
        }
    }
}
```

- [ ] **Step 2: Wire the route in `run_with_state`**

In `src/web_server.rs`, inside `run_with_state`, the `topo_arc` should already be extracted by tiered-memory-foundation Task 3 as:

```rust
    let topo_arc = state.topo_graph.clone();
```

If it's missing, add it right after `let coherence_arc = state.coherence.clone();`.

Then add the route closure — place it after the `memory_neighbors_route` closure (tiered-memory-foundation Task 3), or after `consolidate_route` if that task hasn't run:

```rust
    let memory_search_related_route = {
        let tg = topo_arc.clone();
        get(move |Query(p): Query<MemoryRelatedParams>| async move {
            handle_memory_search_related(tg, Query(p)).await
        })
    };
```

Add the route to the `Router::new()` chain — place after `.route("/memory/neighbors/:id", memory_neighbors_route)` (or after `.route("/memory/consolidate", consolidate_route)` if memory/neighbors doesn't exist yet):

```rust
        .route("/memory/search/related", memory_search_related_route)
```

- [ ] **Step 3: Build to confirm no compilation errors**

```
cargo build --no-default-features --features "petgraph_backend,web-server"
```

Expected: zero errors. If `AppState.topo_graph` is missing, stop and implement tiered-memory-foundation Task 3 first. If `get_neighbors` / `get_incoming` are missing, those were added by tiered-memory-foundation Task 3 — implement that task first.

- [ ] **Step 4: Run full unit suite for regressions**

```
cargo test --no-default-features --features "petgraph_backend" --test unit_suite
```

Expected: all previously passing tests still pass.

- [ ] **Step 5: Commit**

```
git add src/web_server.rs
git commit -m "feat: GET /memory/search/related — PPR graph-based associative memory retrieval (alpha=0.85, limit param)"
```

---

## Self-Review

### Spec Coverage

| Decision / Requirement from exploration | Task | Covered |
|---|---|---|
| Export excludes expired by default | Task 1 | ✓ handle_export_memory fix |
| `?include_expired=true` for backup/migration | Task 1 | ✓ QueryMemoryParams field |
| query honors include_expired flag | Task 1 | ✓ handle_query_memory fix |
| Consolidate data-loss bug (expired candidates) | Task 1 | ✓ filter before pair-finding |
| stats gains `active_records` alongside `total_records` | Task 1 | ✓ StatsResponse + handle_stats |
| PPR `ppr()` on `CausalTopoGraph` | Task 2 | ✓ full power iteration impl |
| α=0.85 fixed, documented, not exposed in v1 | Task 3 | ✓ hardcoded + comment in handler |
| `limit` as sole caller-tunable param | Task 3 | ✓ MemoryRelatedParams.limit |
| `GET /memory/search/related` endpoint | Task 3 | ✓ handler + route |
| Graceful response when seed not yet linked | Task 3 | ✓ note field in JSON |
| Task 3 prerequisite: topo_graph in AppState | Global constraints + Task 3 Step 1 | ✓ grep check before proceeding |

### Placeholder Scan

No TBD, no "handle edge cases" without code, no forward references to undefined types. Every step contains complete code.

### Type Consistency

- `QueryMemoryParams.include_expired: Option<String>` — added in Task 1 Step 4, used in Steps 5 and 6 with exact same `.as_deref() == Some("true")` pattern
- `StatsResponse.active_records: usize` — added in Step 8 struct, populated in same step, used in same step's struct literal
- `let (total_records, active_records, by_type, unique_actors)` — 4-tuple destructure matches 4-tuple return in both Ok and Err branches
- `ppr(&self, seed_id: &str, limit: usize, alpha: f64, iterations: usize) -> Vec<(String, f64)>` — exact signature used in Task 2 tests and Task 3 handler
- `get_neighbors(&sym_id)` and `get_incoming(&sym_id)` — called in Task 3 handler; defined in tiered-memory-foundation Task 3 with signature `pub fn get_neighbors(&self, symbolic_id: &str) -> Vec<String>` and `pub fn get_incoming(&self, symbolic_id: &str) -> Vec<String>`
- `topo_arc: Arc<Mutex<CausalTopoGraph>>` — passed as `tg` into route closure and received as `topo: Arc<Mutex<...>>` in handler; consistent with tiered-memory-foundation Task 3 pattern
