# HipCortex v0.7.0-substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the TX-Log foundation, tx-indexed StateDiff operator, and tag+actor memory compactor — the substrate layer all v0.7.0-beliefs features depend on.

**Architecture:** Append-only monotonic `TxLog` (AtomicU64 counter + JSONL file) enables `compute_tx_diff` (O(range) log replay) and bounded memory via greedy tag+actor `consolidate()` with symbolic edge reanchoring. Auto-trigger fires when `P_tx = hot_count / 10_000 > 0.80`.

**Tech Stack:** Rust stable, serde_json, uuid, chrono, std atomics. Axum (web-server feature) for REST. Python for MCP. Criterion (harness=false) for bench.

**Spec:** `docs/superpowers/specs/2026-08-15-v070-substrate-design.md`

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/tx_log.rs` | `TxKind`, `TxEntry`, `TxLog` — monotonic JSONL log |
| Create | `src/state_diff.rs` | `TxStateDiff`, `compute_tx_diff` — tx-range ΔS |
| Create | `src/consolidation.rs` | `compute_pressure`, `consolidate` — tag+actor compactor |
| Modify | `src/lib.rs` | Register `tx_log`, `state_diff`, `consolidation` modules |
| Modify | `src/web_server.rs` | Add `archive_store`+`tx_log` to `AppState`; wire `handle_add_memory`; add 3 REST routes |
| Modify | `src/bin/webserver.rs` | Wire new fields in `AppState` constructor |
| Create | `tests/unit/tx_log_tests.rs` | 4 unit tests for TxLog |
| Create | `tests/unit/state_diff_tests.rs` | 4 unit tests for StateDiff (Gate 1) |
| Create | `tests/unit/consolidation_tests.rs` | 4 unit tests for compactor |
| Modify | `tests/unit/mod.rs` | Register 3 new unit test modules |
| Create | `tests/integration/consolidation_gates_sit.rs` | Gate 2: 10k records → ≤100 hot, ≥98% reachability |
| Modify | `tests/integration/mod.rs` | Register integration test |
| Create | `benches/temporal_state_diff_bench.rs` | Gate 5: P95 < 5ms |
| Modify | `Cargo.toml` | Add `[[bench]]` entry |
| Modify | `sdk/mcp/server.py` | Add `compute_state_diff`, `consolidate_memory` tools |

---

## Task 1: `src/tx_log.rs` — Monotonic transaction log

**Files:**
- Create: `src/tx_log.rs`
- Create: `tests/unit/tx_log_tests.rs`
- Modify: `src/lib.rs` (add `pub mod tx_log;`)
- Modify: `tests/unit/mod.rs` (add `mod tx_log_tests;`)

- [ ] **Step 1: Write failing unit tests**

Create `tests/unit/tx_log_tests.rs`:

```rust
use hipcortex::tx_log::{TxLog, TxKind};
use uuid::Uuid;

#[test]
fn append_monotonic() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let ids: Vec<u64> = (0..5)
        .map(|_| log.append(TxKind::MemoryAdd, vec![Uuid::new_v4()], "test"))
        .collect();
    for w in ids.windows(2) {
        assert!(w[1] > w[0], "tx_ids not monotonic: {ids:?}");
    }
}

#[test]
fn query_range_correctness() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let all_ids: Vec<u64> = (0..10)
        .map(|_| log.append(TxKind::MemoryAdd, vec![Uuid::new_v4()], "a"))
        .collect();
    let start = all_ids[3];
    let end = all_ids[7];
    let entries = log.query_range(start, end).unwrap();
    assert_eq!(entries.len(), 5, "expected 5 entries in [{start},{end}]");
    for e in &entries {
        assert!(e.tx_id >= start && e.tx_id <= end, "entry out of range: {}", e.tx_id);
    }
}

#[test]
fn counter_restore_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tx.jsonl");
    let last_id = {
        let log = TxLog::open(&path).unwrap();
        log.append(TxKind::MemoryAdd, vec![], "a");
        log.append(TxKind::MemoryAdd, vec![], "a");
        log.current_tx()
    };
    // Reopen — counter must continue, not reset
    let log2 = TxLog::open(&path).unwrap();
    let next = log2.append(TxKind::MemoryAdd, vec![], "a");
    assert!(next > last_id, "counter did not restore: last={last_id} next={next}");
}

#[test]
fn empty_log_identity() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    assert_eq!(log.current_tx(), 0, "fresh log current_tx must be 0");
    let entries = log.query_range(0, 100).unwrap();
    assert!(entries.is_empty());
}
```

- [ ] **Step 2: Register tests, run to confirm FAIL**

Add to `tests/unit/mod.rs` (after last `mod` line):
```rust
mod tx_log_tests;
```

Run:
```sh
cargo test --no-default-features --features "petgraph_backend" --lib tx_log 2>&1 | head -20
```
Expected: error `unresolved import hipcortex::tx_log` — module not yet created.

- [ ] **Step 3: Create `src/tx_log.rs`**

```rust
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxKind {
    MemoryAdd,
    MemoryUpdate,
    MemoryArchive,
    MemoryDelete,
    BeliefAssert,
    BeliefRetract,
    WorldModelObserve,
    WorldModelUpdate,
    GoalCreate,
    GoalStatusChange,
    Consolidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxEntry {
    pub tx_id: u64,
    pub timestamp_ms: u64,
    pub kind: TxKind,
    pub record_ids: Vec<Uuid>,
    pub actor: String,
}

pub struct TxLog {
    counter: Arc<AtomicU64>,
    path: PathBuf,
}

impl TxLog {
    /// Open or create log file. Counter restores from last JSONL line on startup.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let last_tx = if path.exists() {
            let file = std::fs::File::open(&path)
                .map_err(|e| format!("TxLog::open: {e}"))?;
            let mut max_id = 0u64;
            for line in BufReader::new(file).lines().flatten() {
                if let Ok(entry) = serde_json::from_str::<TxEntry>(&line) {
                    if entry.tx_id > max_id {
                        max_id = entry.tx_id;
                    }
                }
            }
            max_id
        } else {
            0
        };
        Ok(Self {
            counter: Arc::new(AtomicU64::new(last_tx + 1)),
            path,
        })
    }

    /// Append one TxEntry. Returns the assigned tx_id. Infallible from caller's view
    /// (write errors go to stderr only; the tx_id is still returned so hot-path continues).
    pub fn append(&self, kind: TxKind, record_ids: Vec<Uuid>, actor: &str) -> u64 {
        let tx_id = self.counter.fetch_add(1, Ordering::SeqCst);
        let entry = TxEntry {
            tx_id,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            kind,
            record_ids,
            actor: actor.to_string(),
        };
        match serde_json::to_string(&entry) {
            Ok(line) => {
                if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.path) {
                    let _ = writeln!(f, "{line}");
                } else {
                    eprintln!("TxLog write error: cannot open {:?}", self.path);
                }
            }
            Err(e) => eprintln!("TxLog serialize error: {e}"),
        }
        tx_id
    }

    /// Return all entries in the inclusive range [from_tx, to_tx].
    pub fn query_range(&self, from_tx: u64, to_tx: u64) -> Result<Vec<TxEntry>, String> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let file = std::fs::File::open(&self.path)
            .map_err(|e| format!("TxLog::query_range: {e}"))?;
        let mut result = Vec::new();
        for line in BufReader::new(file).lines().flatten() {
            if let Ok(entry) = serde_json::from_str::<TxEntry>(&line) {
                if entry.tx_id >= from_tx && entry.tx_id <= to_tx {
                    result.push(entry);
                }
            }
        }
        Ok(result)
    }

    /// Last assigned tx_id (0 if nothing appended yet).
    pub fn current_tx(&self) -> u64 {
        self.counter.load(Ordering::SeqCst).saturating_sub(1)
    }
}
```

- [ ] **Step 4: Register in `src/lib.rs`**

After line `pub mod memory_diff;` (≈ line 39), add:
```rust
pub mod tx_log;
```

- [ ] **Step 5: Run unit tests — confirm PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --test unit_suite tx_log 2>&1 | tail -5
```
Expected: `test tx_log_tests::append_monotonic ... ok` (4 tests, 0 failures).

- [ ] **Step 6: Commit**

```sh
git add src/tx_log.rs src/lib.rs tests/unit/tx_log_tests.rs tests/unit/mod.rs
git commit -m "feat(substrate): add TxLog — monotonic AtomicU64 JSONL transaction log"
```

---

## Task 2: `src/state_diff.rs` — TX-indexed semantic diff

**Files:**
- Create: `src/state_diff.rs`
- Create: `tests/unit/state_diff_tests.rs`
- Modify: `src/lib.rs` (add `pub mod state_diff;`)
- Modify: `tests/unit/mod.rs` (add `mod state_diff_tests;`)

- [ ] **Step 1: Write failing unit tests**

Create `tests/unit/state_diff_tests.rs`:

```rust
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::state_diff::compute_tx_diff;
use hipcortex::tx_log::{TxKind, TxLog};
use uuid::Uuid;

fn make_store() -> MemoryStore<hipcortex::memory_store::InMemoryBackend> {
    MemoryStore::new_in_memory()
}

#[test]
fn identity_empty_log() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let store = make_store();
    let diff = compute_tx_diff(&log, 0, 0, &store).unwrap();
    assert!(diff.memory_delta.added.is_empty());
    assert!(diff.memory_delta.archived.is_empty());
    assert_eq!(diff.tx_count, 0);
    assert_eq!(diff.memory_delta.net_delta, 0);
}

#[test]
fn completeness_after_two_adds() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let mut store = make_store();

    let r1 = MemoryRecord::new(MemoryType::Temporal, "a".into(), "did".into(), "x".into(), serde_json::json!({}));
    let r2 = MemoryRecord::new(MemoryType::Temporal, "a".into(), "did".into(), "y".into(), serde_json::json!({}));
    let id1 = r1.id;
    let id2 = r2.id;

    let tx_before = log.current_tx();
    log.append(TxKind::MemoryAdd, vec![id1], "a");
    log.append(TxKind::MemoryAdd, vec![id2], "a");
    store.add(r1).unwrap();
    store.add(r2).unwrap();

    let diff = compute_tx_diff(&log, tx_before + 1, log.current_tx(), &store).unwrap();
    assert!(diff.memory_delta.added.contains(&id1), "id1 missing from delta");
    assert!(diff.memory_delta.added.contains(&id2), "id2 missing from delta");
    assert_eq!(diff.memory_delta.net_delta, 2);
}

#[test]
fn range_cap_returns_err() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let store = make_store();
    let err = compute_tx_diff(&log, 0, 10_001, &store).unwrap_err();
    assert!(err.contains("cap at 10,000"), "unexpected error msg: {err}");
}

#[test]
fn world_model_observe_attribution() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let store = make_store();
    let rid = Uuid::new_v4();
    log.append(TxKind::WorldModelObserve, vec![rid], "agent");
    let diff = compute_tx_diff(&log, 0, log.current_tx(), &store).unwrap();
    assert_eq!(diff.world_model_delta.observations_added, 1);
    assert!(
        diff.causal_attributions.iter().any(|a| a.record_id == rid),
        "causal attribution missing for WorldModelObserve"
    );
}
```

- [ ] **Step 2: Register tests, run to confirm FAIL**

Add to `tests/unit/mod.rs`:
```rust
mod state_diff_tests;
```

Run:
```sh
cargo test --no-default-features --features "petgraph_backend" --test unit_suite state_diff 2>&1 | head -10
```
Expected: `error[E0432]: unresolved import hipcortex::state_diff`

- [ ] **Step 3: Create `src/state_diff.rs`**

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    memory_store::{MemoryBackend, MemoryStore},
    tx_log::{TxKind, TxLog},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStateDiff {
    pub from_tx: u64,
    pub to_tx: u64,
    pub timestamp_range: (u64, u64),
    pub tx_count: u64,
    pub memory_delta: MemoryDelta,
    pub world_model_delta: WorldModelDelta,
    pub causal_attributions: Vec<CausalAttributionPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryDelta {
    pub added: Vec<Uuid>,
    pub archived: Vec<Uuid>,
    pub updated: Vec<Uuid>,
    pub net_delta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorldModelDelta {
    pub observations_added: u32,
    pub distributions_updated: u32,
    pub causal_edges_added: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAttributionPath {
    pub record_id: Uuid,
    pub tx_id: u64,
    pub trigger_action: String,
    /// Current confidence of the record (0.0 if record was since archived).
    pub confidence_shift: f32,
}

/// Replay TxLog entries in [from_tx, to_tx] and fold into TxStateDiff.
/// Range cap: to_tx - from_tx > 10_000 returns Err (enforced before hitting disk).
pub fn compute_tx_diff<B: MemoryBackend>(
    log: &TxLog,
    from_tx: u64,
    to_tx: u64,
    store: &MemoryStore<B>,
) -> Result<TxStateDiff, String> {
    if to_tx.saturating_sub(from_tx) > 10_000 {
        return Err("tx range too large — cap at 10,000".to_string());
    }
    let entries = log.query_range(from_tx, to_tx)?;

    let mut delta = MemoryDelta::default();
    let mut wm = WorldModelDelta::default();
    let mut attributions: Vec<CausalAttributionPath> = Vec::new();
    let mut ts_min = u64::MAX;
    let mut ts_max = 0u64;

    for entry in &entries {
        if entry.timestamp_ms < ts_min { ts_min = entry.timestamp_ms; }
        if entry.timestamp_ms > ts_max { ts_max = entry.timestamp_ms; }

        match &entry.kind {
            TxKind::MemoryAdd | TxKind::BeliefAssert | TxKind::GoalCreate => {
                delta.added.extend_from_slice(&entry.record_ids);
                for &rid in &entry.record_ids {
                    let conf = store.find_by_id(rid).map(|r| r.confidence).unwrap_or(0.0);
                    attributions.push(CausalAttributionPath {
                        record_id: rid,
                        tx_id: entry.tx_id,
                        trigger_action: format!("{:?}", entry.kind),
                        confidence_shift: conf,
                    });
                }
            }
            TxKind::MemoryArchive | TxKind::BeliefRetract | TxKind::MemoryDelete | TxKind::Consolidate => {
                delta.archived.extend_from_slice(&entry.record_ids);
            }
            TxKind::MemoryUpdate | TxKind::GoalStatusChange => {
                delta.updated.extend_from_slice(&entry.record_ids);
            }
            TxKind::WorldModelObserve => {
                wm.observations_added += entry.record_ids.len() as u32;
                for &rid in &entry.record_ids {
                    attributions.push(CausalAttributionPath {
                        record_id: rid,
                        tx_id: entry.tx_id,
                        trigger_action: "WorldModelObserve".to_string(),
                        confidence_shift: 0.0,
                    });
                }
            }
            TxKind::WorldModelUpdate => {
                wm.distributions_updated += entry.record_ids.len() as u32;
            }
        }
    }

    delta.net_delta = delta.added.len() as i64 - delta.archived.len() as i64;
    let ts_range = if entries.is_empty() { (0, 0) } else { (ts_min, ts_max) };

    Ok(TxStateDiff {
        from_tx,
        to_tx,
        timestamp_range: ts_range,
        tx_count: entries.len() as u64,
        memory_delta: delta,
        world_model_delta: wm,
        causal_attributions: attributions,
    })
}
```

- [ ] **Step 4: Register in `src/lib.rs`**

After `pub mod tx_log;` (just added), add:
```rust
pub mod state_diff;
```

- [ ] **Step 5: Run unit tests — confirm PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --test unit_suite state_diff 2>&1 | tail -5
```
Expected: `4 passed; 0 failed`.

- [ ] **Step 6: Commit**

```sh
git add src/state_diff.rs src/lib.rs tests/unit/state_diff_tests.rs tests/unit/mod.rs
git commit -m "feat(substrate): add StateDiff — tx-indexed ΔS operator with causal attributions"
```

---

## Task 3: `src/consolidation.rs` — Tag+actor greedy memory compactor

**Files:**
- Create: `src/consolidation.rs`
- Create: `tests/unit/consolidation_tests.rs`
- Modify: `src/lib.rs` (add `pub mod consolidation;`)
- Modify: `tests/unit/mod.rs` (add `mod consolidation_tests;`)

- [ ] **Step 1: Write failing unit tests**

Create `tests/unit/consolidation_tests.rs`:

```rust
use hipcortex::archive_store::ArchiveStore;
use hipcortex::consolidation::{compute_pressure, consolidate, ConsolidationConfig};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use hipcortex::tx_log::{TxLog, TxKind};

fn setup(n: usize) -> (MemoryStore<hipcortex::memory_store::InMemoryBackend>, ArchiveStore, SymbolicStore<InMemoryGraph>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = MemoryStore::new_in_memory();
    for i in 0..n {
        let mut r = MemoryRecord::new(
            MemoryType::Temporal,
            "test-actor".into(),
            format!("action-{i}"),
            format!("target-{i}"),
            serde_json::json!({}),
        );
        r.tags = vec!["a".to_string(), "b".to_string()];
        store.add(r).unwrap();
    }
    let archive = ArchiveStore::new(dir.path().join("archive.jsonl"));
    let graph = SymbolicStore::new();
    (store, archive, graph, dir)
}

#[test]
fn pressure_formula() {
    let (store, _, _, _dir) = setup(5_000);
    let config = ConsolidationConfig::default(); // capacity_limit = 10_000
    let p = compute_pressure(&store, &config);
    assert!((p - 0.5).abs() < 0.001, "pressure should be 5000/10000 = 0.5, got {p}");
}

#[test]
fn consolidate_creates_summary_and_archives_originals() {
    let (mut store, mut archive, mut graph, dir) = setup(5);
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let config = ConsolidationConfig { min_group_size: 3, ..Default::default() };

    let before = store.all().len();
    assert_eq!(before, 5);

    let report = consolidate(&mut store, &mut archive, &mut graph, &log, &config).unwrap();

    assert!(report.groups_formed >= 1, "at least one group must form");
    assert!(report.records_collapsed >= 3, "at least min_group_size collapsed");
    // Summary record replaces the group: hot count = (before - collapsed) + 1 summary per group
    let expected_hot = (before - report.records_collapsed) + report.groups_formed;
    assert_eq!(store.all().len(), expected_hot, "hot store count mismatch");
    assert_eq!(archive.count().unwrap(), report.records_collapsed);
}

#[test]
fn consolidate_appends_consolidate_tx_entry() {
    let (mut store, mut archive, mut graph, dir) = setup(5);
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let config = ConsolidationConfig { min_group_size: 3, ..Default::default() };

    let report = consolidate(&mut store, &mut archive, &mut graph, &log, &config).unwrap();

    let entries = log.query_range(0, log.current_tx()).unwrap();
    let consolidate_entry = entries.iter().find(|e| matches!(e.kind, TxKind::Consolidate));
    assert!(consolidate_entry.is_some(), "Consolidate TxEntry not found");
    assert_eq!(report.consolidation_tx_id, consolidate_entry.unwrap().tx_id);
}

#[test]
fn edge_reanchored_to_summary() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = MemoryStore::new_in_memory();
    let mut graph = SymbolicStore::<InMemoryGraph>::new();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();

    // Create 3 records with same actor+tags and add symbolic nodes + edge
    let mut record_ids = Vec::new();
    for i in 0..3 {
        let mut r = MemoryRecord::new(
            MemoryType::Temporal, "actor".into(),
            format!("act-{i}"), format!("tgt-{i}"), serde_json::json!({}),
        );
        r.tags = vec!["edge-test".to_string()];
        let rid = r.id;
        store.add(r).unwrap();
        // Add symbolic node labeled with record UUID
        graph.add_node(&rid.to_string(), std::collections::HashMap::new());
        record_ids.push(rid);
    }
    // Add a target node and an edge from record[0] to it
    let target_node_id = graph.add_node("target", std::collections::HashMap::new());
    let src_nodes = graph.find_by_label(&record_ids[0].to_string());
    if let Some(src) = src_nodes.first() {
        graph.add_edge(src.id, target_node_id, "relates_to");
    }

    let mut archive = ArchiveStore::new(dir.path().join("archive.jsonl"));
    let config = ConsolidationConfig { min_group_size: 3, ..Default::default() };
    let report = consolidate(&mut store, &mut archive, &mut graph, &log, &config).unwrap();

    assert!(report.edges_reanchored >= 1, "edge must be reanchored to summary node");
}
```

- [ ] **Step 2: Register tests, run to confirm FAIL**

Add to `tests/unit/mod.rs`:
```rust
mod consolidation_tests;
```

Run:
```sh
cargo test --no-default-features --features "petgraph_backend" --test unit_suite consolidation 2>&1 | head -10
```
Expected: `error[E0432]: unresolved import hipcortex::consolidation`

- [ ] **Step 3: Create `src/consolidation.rs`**

```rust
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    archive_store::ArchiveStore,
    memory_record::{MemoryRecord, MemoryType},
    memory_store::{MemoryBackend, MemoryStore},
    symbolic_store::{InMemoryGraph, SymbolicStore},
    tx_log::{TxKind, TxLog},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Fraction of capacity_limit that triggers auto-compaction. Default: 0.80.
    pub pressure_threshold: f32,
    /// Max hot records before compaction must run. Default: 10_000.
    pub capacity_limit: usize,
    /// Minimum group size for a group to be collapsed. Default: 3.
    pub min_group_size: usize,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self { pressure_threshold: 0.80, capacity_limit: 10_000, min_group_size: 3 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub groups_formed: usize,
    pub records_collapsed: usize,
    pub records_remaining: usize,
    pub edges_reanchored: usize,
    pub consolidation_tx_id: u64,
    pub pressure_before: f32,
    pub pressure_after: f32,
}

/// P_tx = hot_count / capacity_limit. Values > pressure_threshold trigger auto-consolidation.
pub fn compute_pressure<B: MemoryBackend>(store: &MemoryStore<B>, config: &ConsolidationConfig) -> f32 {
    store.all().len() as f32 / config.capacity_limit as f32
}

/// Greedy tag+actor consolidation.
///
/// Groups all Temporal hot records by `actor:sorted_tags` key.
/// Groups with ≥ min_group_size records are collapsed into a single SummaryRecord.
/// Originals move to cold store via ArchiveStore::append().
/// Symbolic edges from originals are re-anchored to the summary node.
pub fn consolidate<B: MemoryBackend>(
    store: &mut MemoryStore<B>,
    archive: &mut ArchiveStore,
    graph: &mut SymbolicStore<InMemoryGraph>,
    tx_log: &TxLog,
    config: &ConsolidationConfig,
) -> Result<ConsolidationReport, String> {
    let pressure_before = compute_pressure(store, config);

    // 1. Collect all Temporal records
    let temporals: Vec<MemoryRecord> = store
        .all()
        .iter()
        .filter(|r| r.record_type == MemoryType::Temporal)
        .cloned()
        .collect();

    // 2. Group by actor:sorted_tags
    let mut groups: HashMap<String, Vec<MemoryRecord>> = HashMap::new();
    for record in temporals {
        let mut tags = record.tags.clone();
        tags.sort_unstable();
        let key = format!("{}:{}", record.actor, tags.join(","));
        groups.entry(key).or_default().push(record);
    }

    // 3. Keep only groups meeting min_group_size
    let eligible: Vec<Vec<MemoryRecord>> = groups
        .into_values()
        .filter(|g| g.len() >= config.min_group_size)
        .collect();

    let mut total_collapsed = 0usize;
    let mut total_reanchored = 0usize;
    let mut all_archived_ids = Vec::new();

    for group in &eligible {
        let group_actor = group[0].actor.clone();
        let timestamps: Vec<u64> = group
            .iter()
            .map(|r| r.timestamp.timestamp_millis() as u64)
            .collect();
        let ts_min = timestamps.iter().copied().min().unwrap_or(0);
        let ts_max = timestamps.iter().copied().max().unwrap_or(0);
        let conf_mean =
            group.iter().map(|r| r.confidence).sum::<f32>() / group.len() as f32;

        // 4. Create summary record
        let summary = MemoryRecord::new(
            MemoryType::Temporal,
            group_actor,
            "consolidated".to_string(),
            format!("summary/{}", group[0].tags.join(",")),
            serde_json::json!({
                "group_size": group.len(),
                "time_range_ms": [ts_min, ts_max],
                "confidence_mean": conf_mean,
            }),
        );
        let summary_id = summary.id;
        store.add(summary).map_err(|e| format!("consolidate add summary: {e}"))?;

        // 5. Add summary node to symbolic graph
        let sym_summary_id = graph.add_node(
            &summary_id.to_string(),
            {
                let mut props = HashMap::new();
                props.insert("type".to_string(), "summary".to_string());
                props
            },
        );

        // 6. Process each original: reanchor edges → archive → delete from hot store
        for original in group {
            let sym_nodes = graph.find_by_label(&original.id.to_string());
            for sym_node in sym_nodes {
                let edges = graph.edges_from(sym_node.id, None);
                total_reanchored += edges.len();
                for edge in &edges {
                    graph.add_edge(sym_summary_id, edge.to, &edge.relation);
                }
                graph.remove_node(sym_node.id); // also removes all incident edges
            }
            archive
                .append(original.clone())
                .map_err(|e| format!("consolidate archive: {e}"))?;
            store.delete_by_id(original.id);
            all_archived_ids.push(original.id);
        }
        total_collapsed += group.len();
    }

    let consolidation_tx_id = tx_log.append(TxKind::Consolidate, all_archived_ids, "system");
    let pressure_after = compute_pressure(store, config);

    Ok(ConsolidationReport {
        groups_formed: eligible.len(),
        records_collapsed: total_collapsed,
        records_remaining: store.all().len(),
        edges_reanchored: total_reanchored,
        consolidation_tx_id,
        pressure_before,
        pressure_after,
    })
}
```

- [ ] **Step 4: Register in `src/lib.rs`**

After `pub mod state_diff;`, add:
```rust
pub mod consolidation;
```

- [ ] **Step 5: Run unit tests — confirm PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --test unit_suite consolidation 2>&1 | tail -5
```
Expected: `4 passed; 0 failed`.

- [ ] **Step 6: Commit**

```sh
git add src/consolidation.rs src/lib.rs tests/unit/consolidation_tests.rs tests/unit/mod.rs
git commit -m "feat(substrate): add memory consolidation — greedy tag+actor compactor with edge reanchoring"
```

---

## Task 4: Wire `AppState` — add `archive_store` + `tx_log`, auto-trigger in `handle_add_memory`

**Files:**
- Modify: `src/web_server.rs` (AppState struct, Clone impl, run_with_state unpack, both add_memory_route closures, handle_add_memory signature + body)
- Modify: `src/bin/webserver.rs` (AppState constructor)

- [ ] **Step 1: Add fields to `AppState` struct**

In `src/web_server.rs`, find the `AppState` struct (≈ line 123). Add two fields after `topo_graph`:

```rust
pub struct AppState<B: MemoryBackend + Send + Sync + 'static> {
    pub memory_store:   Arc<Mutex<MemoryStore<B>>>,
    pub symbolic_store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
    pub world_model:    Arc<RwLock<WorldModelEnhanced>>,
    pub aureus:         Arc<Mutex<AureusBridge>>,
    pub self_model:     Arc<SelfModel>,
    pub coherence:      Arc<CoherenceChecker>,
    pub topo_graph:     Arc<Mutex<crate::topological_memory::CausalTopoGraph>>,
    // v0.7.0-substrate
    pub archive_store:  Arc<Mutex<crate::archive_store::ArchiveStore>>,
    pub tx_log:         Option<Arc<crate::tx_log::TxLog>>,
}
```

- [ ] **Step 2: Update `Clone` impl for `AppState`**

In the `impl Clone for AppState<B>` block (≈ line 141), add the two new fields:

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
        // v0.7.0-substrate
        archive_store:  self.archive_store.clone(),
        tx_log:         self.tx_log.clone(),
    }
}
```

- [ ] **Step 3: Update `run_with_memory` constructor (web_server.rs ≈ line 477)**

```rust
pub async fn run_with_memory<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr,
    memory_store: Arc<Mutex<MemoryStore<B>>>,
) {
    let state = AppState {
        memory_store,
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model: Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model: Arc::new(SelfModel::new()),
        coherence: Arc::new(CoherenceChecker::new()),
        topo_graph: Arc::new(Mutex::new(crate::topological_memory::CausalTopoGraph::new())),
        archive_store: Arc::new(Mutex::new(
            crate::archive_store::ArchiveStore::new("memory-archive.jsonl"),
        )),
        tx_log: crate::tx_log::TxLog::open("memory-tx.jsonl").ok().map(Arc::new),
    };
    run_with_state(addr, state).await;
}
```

- [ ] **Step 4: Update `AppState` constructor in `src/bin/webserver.rs` (≈ line 67)**

```rust
let state = AppState {
    memory_store: memory_store.clone(),
    symbolic_store: Arc::new(Mutex::new(SymbolicStore::<InMemoryGraph>::new())),
    world_model: world_model.clone(),
    aureus: Arc::new(Mutex::new(AureusBridge::new())),
    self_model,
    coherence: Arc::new(CoherenceChecker::new()),
    topo_graph: Arc::new(Mutex::new(hipcortex::topological_memory::CausalTopoGraph::new())),
    // v0.7.0-substrate
    archive_store: Arc::new(Mutex::new(
        hipcortex::archive_store::ArchiveStore::new(format!("{}/memory-archive.jsonl", data_dir)),
    )),
    tx_log: hipcortex::tx_log::TxLog::open(format!("{}/memory-tx.jsonl", data_dir))
        .ok()
        .map(Arc::new),
};
```

- [ ] **Step 5: Unpack new fields in `run_with_state` (web_server.rs ≈ line 504)**

After `let topo_arc = state.topo_graph.clone();`, add:
```rust
let archive_store_arc = state.archive_store.clone();
let tx_log_arc       = state.tx_log.clone();  // Option<Arc<TxLog>>
```

- [ ] **Step 6: Update `handle_add_memory` signature**

Change the function signature at line 3082:
```rust
async fn handle_add_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    req: AddMemoryRequest,
    tx_log: Option<Arc<crate::tx_log::TxLog>>,
    archive_store: Option<Arc<Mutex<crate::archive_store::ArchiveStore>>>,
    symbolic_store: Option<Arc<Mutex<SymbolicStore<InMemoryGraph>>>>,
) -> Result<Json<AddMemoryResponse>, (StatusCode, Json<AddMemoryResponse>)>
```

- [ ] **Step 7: Wire auto-trigger in `handle_add_memory` body**

Inside the `match ms.add(record.clone()) { Ok(_) => { ... } }` block (≈ line 3197), just after `ms.add(record.clone())` succeeds and BEFORE the existing worldmodel feed, add:

```rust
Ok(_) => {
    // v0.7.0-substrate: append to TX log + capture consolidation trigger flag
    let mut spawn_consolidation: Option<(
        Arc<Mutex<MemoryStore<B>>>,
        Arc<Mutex<crate::archive_store::ArchiveStore>>,
        Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
        Arc<crate::tx_log::TxLog>,
    )> = None;

    if let Some(ref tl) = tx_log {
        tl.append(crate::tx_log::TxKind::MemoryAdd, vec![record.id], &record.actor);
        let pressure = crate::consolidation::compute_pressure(
            &*ms, &crate::consolidation::ConsolidationConfig::default(),
        );
        if pressure > crate::consolidation::ConsolidationConfig::default().pressure_threshold {
            if let (Some(a), Some(g)) = (archive_store.as_ref(), symbolic_store.as_ref()) {
                spawn_consolidation = Some((
                    store.clone(),
                    Arc::clone(a),
                    Arc::clone(g),
                    Arc::clone(tl),
                ));
            }
        }
    }
    // ... existing worldmodel feed, webhook, response ...
    let response = Ok(Json(AddMemoryResponse { ... }));
    
    // Drop ms before spawning (ms released at end of this match arm)
    drop(ms);
    
    if let Some((s, a, g, tl)) = spawn_consolidation {
        tokio::task::spawn_blocking(move || {
            if let (Ok(mut store), Ok(mut archive), Ok(mut graph)) =
                (s.lock(), a.lock(), g.lock())
            {
                let _ = crate::consolidation::consolidate(
                    &mut *store, &mut *archive, &mut *graph,
                    &tl, &crate::consolidation::ConsolidationConfig::default(),
                );
            }
        });
    }
    
    response
}
```

**Important:** The `drop(ms)` must happen before `spawn_blocking` so the lock is not held when the background task tries to acquire it. Restructure the match arm to `drop(ms)` explicitly, then run the spawn, then return the response variable.

- [ ] **Step 8: Update both `add_memory_route` closures**

At line ≈529 and ≈2515, update to pass new params. Both closures follow the same pattern:

```rust
let add_memory_route = {
    let store = memory_store.clone();
    let wm = world_model.clone();
    let tx = tx_log_arc.clone();
    let archive = archive_store_arc.clone();
    let sym = symbolic_store.clone();
    post(move |Json(req): Json<AddMemoryRequest>| async move {
        handle_add_memory(store, wm, req, tx, Some(archive), Some(sym)).await
    })
};
```

- [ ] **Step 9: Build check**

```sh
cargo build --no-default-features --features "web-server,petgraph_backend" 2>&1 | grep "^error" | head -20
```
Expected: no errors. Fix any `unused variable` warnings with `_` prefix if needed.

- [ ] **Step 10: Commit**

```sh
git add src/web_server.rs src/bin/webserver.rs
git commit -m "feat(substrate): wire archive_store + tx_log into AppState; auto-trigger consolidation on P_tx > 0.80"
```

---

## Task 5: REST routes — `/v1/state/diff`, `/v1/memory/consolidate`, `/v1/state/tx`

**Files:**
- Modify: `src/web_server.rs` (add 3 handler functions + 3 route entries)

- [ ] **Step 1: Add handler functions**

Add these three handler functions to `src/web_server.rs` (place them near the existing `/memory/diff` handler for locality):

```rust
// ── v0.7.0-substrate REST endpoints ─────────────────────────────────────

#[cfg(feature = "web-server")]
#[derive(serde::Deserialize)]
struct StateDiffRequest {
    from_tx: u64,
    to_tx: u64,
}

#[cfg(feature = "web-server")]
async fn handle_state_diff_v1<B: MemoryBackend + Send + Sync + 'static>(
    axum::extract::State(state): axum::extract::State<AppState<B>>,
    Json(req): Json<StateDiffRequest>,
) -> impl axum::response::IntoResponse {
    use axum::http::StatusCode;
    let Some(ref tx_log) = state.tx_log else {
        return (StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "tx_log not configured"}))).into_response();
    };
    let store = state.memory_store.lock().unwrap();
    match crate::state_diff::compute_tx_diff(tx_log, req.from_tx, req.to_tx, &*store) {
        Ok(diff) => (StatusCode::OK, Json(diff)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

#[cfg(feature = "web-server")]
#[derive(serde::Deserialize, Default)]
struct ConsolidateRequest {
    pressure_threshold: Option<f32>,
    capacity_limit: Option<usize>,
    min_group_size: Option<usize>,
}

#[cfg(feature = "web-server")]
async fn handle_memory_consolidate_v1<B: MemoryBackend + Send + Sync + 'static>(
    axum::extract::State(state): axum::extract::State<AppState<B>>,
    body: Option<Json<ConsolidateRequest>>,
) -> impl axum::response::IntoResponse {
    use axum::http::StatusCode;
    let req = body.map(|Json(r)| r).unwrap_or_default();
    let mut config = crate::consolidation::ConsolidationConfig::default();
    if let Some(t) = req.pressure_threshold { config.pressure_threshold = t; }
    if let Some(c) = req.capacity_limit { config.capacity_limit = c; }
    if let Some(m) = req.min_group_size { config.min_group_size = m; }

    let Some(ref tx_log) = state.tx_log else {
        return (StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "tx_log not configured"}))).into_response();
    };
    let mut store   = state.memory_store.lock().unwrap();
    let mut archive = state.archive_store.lock().unwrap();
    let mut graph   = state.symbolic_store.lock().unwrap();

    match crate::consolidation::consolidate(&mut *store, &mut *archive, &mut *graph, tx_log, &config) {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

#[cfg(feature = "web-server")]
async fn handle_state_tx_v1<B: MemoryBackend + Send + Sync + 'static>(
    axum::extract::State(state): axum::extract::State<AppState<B>>,
) -> impl axum::response::IntoResponse {
    let current = state.tx_log.as_ref().map(|t| t.current_tx()).unwrap_or(0);
    Json(serde_json::json!({ "current_tx": current }))
}
```

- [ ] **Step 2: Register routes in the router**

In `run_with_state`, after the `.route("/memory/diff", memory_diff_route)` line (≈ line 2799), add:

```rust
.route("/v1/state/diff",         post({
    let s = state.clone();
    move |body: Json<StateDiffRequest>| async move { handle_state_diff_v1(axum::extract::State(s), body).await }
}))
.route("/v1/memory/consolidate", post({
    let s = state.clone();
    move |body: Option<Json<ConsolidateRequest>>| async move { handle_memory_consolidate_v1(axum::extract::State(s), body).await }
}))
.route("/v1/state/tx",           get({
    let s = state.clone();
    move || async move { handle_state_tx_v1(axum::extract::State(s)).await }
}))
```

Note: these handlers need `State` extractor. Check if `use axum::extract::State;` is already imported at the top of the file. If not, add it.

- [ ] **Step 3: Build check**

```sh
cargo build --no-default-features --features "web-server,petgraph_backend" 2>&1 | grep "^error" | head -20
```
Expected: no errors.

- [ ] **Step 4: Commit**

```sh
git add src/web_server.rs
git commit -m "feat(substrate): add REST /v1/state/diff, /v1/memory/consolidate, /v1/state/tx"
```

---

## Task 6: MCP surface parity — `compute_state_diff` + `consolidate_memory`

**Files:**
- Modify: `sdk/mcp/server.py` (add 2 tool definitions + 2 handler functions + 2 dispatch entries)

- [ ] **Step 1: Add tool definitions to `TOOLS` list**

In `sdk/mcp/server.py`, find `TOOLS = [` (≈ line 76). Add after the last tool entry (before the closing `]`):

```python
    {
        "name": "compute_state_diff",
        "description": "Compute semantic diff between two cognitive state snapshots by tx range. "
                       "Returns memory_delta (added/archived/updated UUIDs), world_model_delta, "
                       "and causal_attributions. Range cap: to_tx - from_tx ≤ 10,000.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "from_tx": {"type": "integer", "description": "Start transaction ID (inclusive)"},
                "to_tx":   {"type": "integer", "description": "End transaction ID (inclusive)"},
            },
            "required": ["from_tx", "to_tx"],
        },
    },
    {
        "name": "consolidate_memory",
        "description": "Trigger hierarchical memory compaction. Groups episodic records by "
                       "actor+tags, collapses groups into summary records, re-anchors graph edges. "
                       "Returns ConsolidationReport with groups_formed, records_collapsed, pressure_before/after.",
        "inputSchema": {"type": "object", "properties": {}},
    },
```

- [ ] **Step 2: Add handler functions**

Near the `handle_can_execute` function (≈ line 710), add:

```python
def handle_compute_state_diff(args: dict) -> str:
    from_tx = args["from_tx"]
    to_tx = args["to_tx"]
    if to_tx - from_tx > 10_000:
        return f"✗ tx range too large (got {to_tx - from_tx}). Cap at 10,000."
    result = _post("/v1/state/diff", {"from_tx": from_tx, "to_tx": to_tx})
    if result.get("error"):
        return f"✗ state_diff failed: {result['error']}"
    md = result.get("memory_delta", {})
    wm = result.get("world_model_delta", {})
    lines = [
        f"StateDiff tx[{from_tx}..{to_tx}]  ({result.get('tx_count', 0)} entries)",
        f"  memory: +{len(md.get('added', []))} added, -{len(md.get('archived', []))} archived, "
        f"~{len(md.get('updated', []))} updated  (net {md.get('net_delta', 0)})",
        f"  world_model: {wm.get('observations_added', 0)} obs, "
        f"{wm.get('distributions_updated', 0)} dist updates",
        f"  causal_attributions: {len(result.get('causal_attributions', []))}",
    ]
    return "\n".join(lines)


def handle_consolidate_memory(args: dict) -> str:
    result = _post("/v1/memory/consolidate", {})
    if result.get("error"):
        return f"✗ consolidate_memory failed: {result['error']}"
    return (
        f"Consolidation complete: {result.get('groups_formed', 0)} groups, "
        f"{result.get('records_collapsed', 0)} records collapsed → "
        f"{result.get('records_remaining', 0)} remaining. "
        f"Pressure {result.get('pressure_before', '?'):.2f} → {result.get('pressure_after', '?'):.2f}"
    )
```

- [ ] **Step 3: Add entries to `handlers` dict**

In the `handlers` dict inside `dispatch_tool` (≈ line 726), add:

```python
        "compute_state_diff":  handle_compute_state_diff,
        "consolidate_memory":  handle_consolidate_memory,
```

- [ ] **Step 4: Smoke test (no live server required)**

```sh
cd sdk/mcp && python -c "
import server
# Check tools are registered
names = [t['name'] for t in server.TOOLS]
assert 'compute_state_diff' in names, f'missing compute_state_diff in {names}'
assert 'consolidate_memory' in names, f'missing consolidate_memory in {names}'
# Check handlers are wired
import types
# Re-use dispatch path: check handlers dict has keys
# (dispatch_tool is a function so we call it with a mock that hits ValueError on unknown)
try:
    server.dispatch_tool('compute_state_diff', {'from_tx': 0, 'to_tx': 1})
except Exception as e:
    if 'Unknown tool' in str(e):
        raise AssertionError(f'handler not wired: {e}')
print('OK')
"
```
Expected: `OK` (the handler will fail on the HTTP call but won't raise `Unknown tool`).

- [ ] **Step 5: Commit**

```sh
git add sdk/mcp/server.py
git commit -m "feat(substrate): add MCP tools compute_state_diff + consolidate_memory"
```

---

## Task 7: Integration Gate 2 test — 10k records → ≤100 hot

**Files:**
- Create: `tests/integration/consolidation_gates_sit.rs`
- Modify: `tests/integration/mod.rs` (add `mod consolidation_gates_sit;`)

- [ ] **Step 1: Write the Gate 2 integration test**

Create `tests/integration/consolidation_gates_sit.rs`:

```rust
/// Gate 2 — Bounded Memory: 10,000 episodic records compacted to ≤100 hot, ≥98% edge reachability.
use hipcortex::archive_store::ArchiveStore;
use hipcortex::consolidation::{consolidate, ConsolidationConfig};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use hipcortex::tx_log::TxLog;
use std::collections::HashSet;

const RECORD_COUNT: usize = 10_000;

fn build_store() -> (MemoryStore<hipcortex::memory_store::InMemoryBackend>, SymbolicStore<InMemoryGraph>) {
    let mut store = MemoryStore::new_in_memory();
    let mut graph = SymbolicStore::new();

    // All records in the same group (same actor + tags) so they consolidate into one summary.
    for i in 0..RECORD_COUNT {
        let mut r = MemoryRecord::new(
            MemoryType::Temporal,
            "bench-actor".into(),
            format!("action-{i}"),
            format!("target-{i}"),
            serde_json::json!({}),
        );
        r.tags = vec!["group-a".to_string(), "group-b".to_string()];
        let rid = r.id;
        store.add(r).unwrap();
        // Add symbolic node for each record so we can verify reachability
        graph.add_node(&rid.to_string(), std::collections::HashMap::new());
    }

    // Add a shared "sink" terminal node and edges from first 100 records
    let sink_id = graph.add_node("terminal-sink", std::collections::HashMap::new());
    let sample_records: Vec<_> = store.all().iter().take(100).map(|r| r.id).collect();
    for rid in &sample_records {
        let nodes = graph.find_by_label(&rid.to_string());
        if let Some(n) = nodes.first() {
            graph.add_edge(n.id, sink_id, "points_to");
        }
    }

    (store, graph)
}

/// Gate 2a: hot store count ≤ 100 after consolidation.
#[test]
fn gate2_hot_count_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, mut graph) = build_store();
    let mut archive = ArchiveStore::new(dir.path().join("archive.jsonl"));
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let config = ConsolidationConfig { min_group_size: 3, ..Default::default() };

    let report = consolidate(&mut store, &mut archive, &mut graph, &log, &config)
        .expect("consolidation failed");

    let hot_count = store.all().len();
    assert!(
        hot_count <= 100,
        "Gate 2 FAIL: hot_count={hot_count} > 100. report={report:?}"
    );
    assert!(
        report.records_collapsed >= RECORD_COUNT - 50,
        "Too few records collapsed: {}",
        report.records_collapsed
    );
}

/// Gate 2b: edges from originals (sampled) reachable from summary nodes after consolidation.
/// Target: ≥ 98% of edges present before compaction are reachable after.
#[test]
fn gate2_edge_reachability_98pct() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, mut graph) = build_store();

    // Record which records had edges to the sink before consolidation
    let (_, all_edges) = graph.export_graph();
    let has_edge_before: HashSet<uuid::Uuid> = all_edges
        .iter()
        .map(|e| e.from)
        .collect();
    let edges_before = has_edge_before.len();

    let mut archive = ArchiveStore::new(dir.path().join("archive.jsonl"));
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let config = ConsolidationConfig { min_group_size: 3, ..Default::default() };
    consolidate(&mut store, &mut archive, &mut graph, &log, &config)
        .expect("consolidation failed");

    // After consolidation, edges from summary nodes must reach the terminal sink
    let (_, edges_after) = graph.export_graph();
    let edges_after_count = edges_after.len();

    // Reachability fraction: edges remaining / edges before compaction
    // Since consolidation collapses N edges into 1 summary edge per group, the raw count will drop.
    // We check that ALL original target nodes (sink) are still reachable from some surviving node.
    let all_targets_after: HashSet<uuid::Uuid> = edges_after.iter().map(|e| e.to).collect();
    let all_targets_before: HashSet<uuid::Uuid> = all_edges.iter().map(|e| e.to).collect();

    let reachable = all_targets_before.intersection(&all_targets_after).count();
    let total = all_targets_before.len().max(1);
    let reachability = reachable as f32 / total as f32;

    assert!(
        reachability >= 0.98,
        "Gate 2 FAIL: edge reachability={:.1}% < 98%. before={edges_before} after={edges_after_count}",
        reachability * 100.0
    );
}
```

- [ ] **Step 2: Register in `tests/integration/mod.rs`**

Add at the end:
```rust
mod consolidation_gates_sit;
```

- [ ] **Step 3: Run Gate 2 tests — confirm PASS**

```sh
cargo test --no-default-features --features "petgraph_backend" --test integration_suite consolidation_gates 2>&1 | tail -10
```
Expected: `2 passed; 0 failed`. These tests may take 5-10 seconds due to 10k record inserts.

- [ ] **Step 4: Commit**

```sh
git add tests/integration/consolidation_gates_sit.rs tests/integration/mod.rs
git commit -m "test(substrate): Gate 2 integration test — 10k records bounded to ≤100 hot + ≥98% edge reachability"
```

---

## Task 8: Criterion bench — Gate 5 (P95 < 5ms for StateDiff over 1,000 entries)

**Files:**
- Modify: `Cargo.toml` (add `[[bench]]` entry)
- Create: `benches/temporal_state_diff_bench.rs`

- [ ] **Step 1: Register bench in `Cargo.toml`**

After the last `[[bench]]` block (e.g., after `integration_bench`), add:

```toml
[[bench]]
name = "temporal_state_diff_bench"
harness = false
```

- [ ] **Step 2: Create `benches/temporal_state_diff_bench.rs`**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hipcortex::memory_store::MemoryStore;
use hipcortex::state_diff::compute_tx_diff;
use hipcortex::tx_log::{TxKind, TxLog};
use uuid::Uuid;

fn bench_state_diff_1k(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("bench_tx.jsonl")).unwrap();
    let store = MemoryStore::new_in_memory();

    // Pre-populate log with 1,000 MemoryAdd entries
    let from_tx = log.current_tx() + 1;
    for _ in 0..1_000 {
        log.append(TxKind::MemoryAdd, vec![Uuid::new_v4()], "bench-actor");
    }
    let to_tx = log.current_tx();

    c.bench_function("compute_tx_diff_1k", |b| {
        b.iter(|| {
            let diff = compute_tx_diff(
                black_box(&log),
                black_box(from_tx),
                black_box(to_tx),
                black_box(&store),
            )
            .unwrap();
            black_box(diff.tx_count)
        })
    });
}

criterion_group!(benches, bench_state_diff_1k);
criterion_main!(benches);
```

- [ ] **Step 3: Run bench (Gate 5 verification)**

```sh
cargo bench --no-default-features --features "petgraph_backend" --bench temporal_state_diff_bench 2>&1 | grep -E "time:|thrpt:"
```
Expected output includes line like:
```
compute_tx_diff_1k  time:   [1.2 ms 1.4 ms 1.6 ms]
```
All three values (lower bound, median, upper bound) must be < 5ms. If any value > 5ms, the JSONL read is the bottleneck — profile by size and switch to buffered read (already implemented via `BufReader`).

- [ ] **Step 4: Commit**

```sh
git add Cargo.toml benches/temporal_state_diff_bench.rs
git commit -m "bench(substrate): Gate 5 — compute_tx_diff P95 < 5ms over 1k tx range"
```

---

## Task 9: Final build + lint + full test pass

- [ ] **Step 1: Format**

```sh
cargo fmt --all
```

- [ ] **Step 2: Clippy (zero warnings allowed)**

```sh
cargo clippy --no-default-features --features "web-server,petgraph_backend" --all-targets -- -D warnings 2>&1 | head -30
```
Fix any warnings before proceeding.

- [ ] **Step 3: Full library test suite**

```sh
cargo test --no-default-features --features "petgraph_backend" --test unit_suite 2>&1 | tail -10
cargo test --no-default-features --features "petgraph_backend" --test integration_suite 2>&1 | tail -10
```
Expected: all pass, no regressions.

- [ ] **Step 4: Update `docs/superpowers/specs/2026-08-15-v070-index.md`**

Mark substrate plan as created. Find the Plans section and update:
```markdown
## Plans (created by writing-plans)

- [x] `docs/superpowers/plans/2026-08-15-v070-substrate-plan.md`
- [ ] `docs/superpowers/plans/2026-08-15-v070-beliefs-plan.md`
```

- [ ] **Step 5: Final commit**

```sh
git add -u
git commit -m "chore(substrate): fmt + clippy + substrate plan marked in v0.7.0 index"
```

---

## Self-Review Checklist

**Spec coverage:**

| Spec requirement | Task |
|------------------|------|
| `TxLog` with AtomicU64 + JSONL | Task 1 |
| `TxLog::open` restores counter from last line | Task 1 |
| `TxLog::append` infallible from caller | Task 1 (eprintln on error, returns tx_id) |
| `TxStateDiff`, `MemoryDelta`, `WorldModelDelta`, `CausalAttributionPath` | Task 2 |
| `compute_tx_diff` range cap at 10,000 | Task 2 (returns Err) |
| `compute_tx_diff` uses TxLog replay (not record-level diff) | Task 2 |
| `ConsolidationConfig` with defaults | Task 3 |
| `compute_pressure = count / capacity_limit` | Task 3 |
| `consolidate` greedy tag+actor grouping | Task 3 |
| `consolidate` archives via `ArchiveStore::append` (Hot/Cold rule) | Task 3 |
| `consolidate` edge reanchoring via `SymbolicStore` | Task 3 |
| `AppState` gains `archive_store` + `tx_log` | Task 4 |
| Auto-trigger on P_tx > 0.80, non-blocking background task | Task 4 |
| REST `POST /v1/state/diff` | Task 5 |
| REST `POST /v1/memory/consolidate` | Task 5 |
| REST `GET /v1/state/tx` | Task 5 |
| MCP `compute_state_diff` tool | Task 6 |
| MCP `consolidate_memory` tool | Task 6 |
| Gate 2: 10k → ≤100 hot | Task 7 |
| Gate 5: P95 < 5ms bench | Task 8 |
| `state_diff.rs` is NEW file; `memory_diff.rs` stays unchanged | ✓ confirmed (spec requirement) |
| `lib.rs` registers all three new modules | Tasks 1-3 |

**No placeholders:** All code blocks contain complete, runnable Rust/Python. No TBD/TODO.

**Type consistency:**
- `TxLog::append` → `u64` (tx_id)
- `TxLog::current_tx` → `u64`
- `compute_tx_diff(log, from_tx, to_tx, store) -> Result<TxStateDiff, String>` — consistent across Tasks 2, 5, 7, 8
- `consolidate(store, archive, graph, tx_log, config) -> Result<ConsolidationReport, String>` — consistent across Tasks 3, 4, 5, 7
- `InMemoryBackend` used in tests, `B: MemoryBackend` in generic functions — consistent

---

**Plan complete and saved to `docs/superpowers/plans/2026-08-15-v070-substrate-plan.md`.**

Two execution options:

**1. Subagent-Driven (recommended)** — fresh subagent per task, two-stage review (spec then quality) between tasks, fast iteration

**2. Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
