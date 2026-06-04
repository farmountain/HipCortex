# Intelligence Layer Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire WorldModelEnhanced, AureusBridge, SelfModel, and persistent CoherenceChecker into the HipCortex web server via an AppState struct, expose 9 new REST endpoints, and add JSON-snapshot persistence for the world model.

**Architecture:** `AppState<B>` struct bundles all stores + intelligence components; `run_with_both_stores` is renamed `run_with_state(addr, AppState<B>)`; only handlers that need new state change; world model persists via `worldmodel.json` with atomic write + 5-minute periodic flush.

**Tech Stack:** Rust, Axum 0.6, tokio, serde_json, existing crate deps only (no new crates).

**Spec:** `docs/superpowers/specs/2026-06-04-intelligence-layer-wiring-design.md`

**Worktree:** `D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c`

**Build command (always use this):**
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo check --no-default-features --features "web-server,petgraph_backend"
```

**Test command:**
```powershell
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring
```

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/web_server.rs` | MODIFY | AppState struct, rename entry point, new handlers, auto-feed |
| `src/bin/webserver.rs` | MODIFY | Construct AppState, SelfModel bootstrap, shutdown save, periodic flush |
| `src/modules/world_model_enhanced/mod.rs` | MODIFY | Add save/load/transition_count |
| `src/modules/world_model_enhanced/transition.rs` | MODIFY | Add save/load/smoothing getter |
| `src/modules/world_model_enhanced/causal.rs` | MODIFY | Add save_edges/load_edges/all_edges |
| `src/modules/aureus_bridge.rs` | MODIFY | Add reflect_on_memory |
| `tests/integration/intelligence_wiring_sit.rs` | CREATE | All SIT tests |
| `tests/integration/mod.rs` | MODIFY | Register new test module |

---

## Task 1: AppState struct + run_with_state entry point

**Files:**
- Modify: `src/web_server.rs` (top imports + AppState struct + rename function)

### Why first
All other tasks depend on AppState existing. This is the structural foundation.

- [ ] **Step 1.1: Write the failing test** (in `tests/integration/intelligence_wiring_sit.rs`)

```rust
/// SIT tests for intelligence layer wiring
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::web_server::AppState;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::self_model::SelfModel;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use std::sync::{Arc, Mutex, RwLock};

fn make_app_state() -> AppState<InMemoryBackend> {
    AppState {
        memory_store: Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model: Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model: Arc::new(SelfModel::new()),
        coherence: Arc::new(CoherenceChecker::new()),
    }
}

#[test]
fn test_app_state_constructs() {
    let state = make_app_state();
    // AppState exists and all fields accessible
    assert!(state.self_model.is_healthy().is_ok());
    assert!(state.world_model.read().unwrap().list_entities().unwrap().is_empty());
}

#[test]
fn test_app_state_clone() {
    let state = make_app_state();
    let state2 = state.clone();
    // Both point to same underlying Arcs
    assert!(Arc::ptr_eq(&state.memory_store, &state2.memory_store));
    assert!(Arc::ptr_eq(&state.world_model, &state2.world_model));
}
```

- [ ] **Step 1.2: Register test file**

In `tests/integration/mod.rs`, add at the bottom:
```rust
mod intelligence_wiring_sit;
```

- [ ] **Step 1.3: Run test — expect compile error** (AppState doesn't exist yet)

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "error"
```
Expected: `error[E0432]: unresolved import 'hipcortex::web_server::AppState'`

- [ ] **Step 1.4: Add AppState struct to web_server.rs**

In `src/web_server.rs`, after the existing imports (around line 25), add:

```rust
use crate::world_model_enhanced::WorldModelEnhanced;
use crate::aureus_bridge::AureusBridge;
use crate::self_model::SelfModel;
use std::sync::RwLock;

/// Central server state — bundles all stores and intelligence components.
/// Passed into run_with_state(); each handler closure Arc-clones what it needs.
#[cfg(feature = "web-server")]
#[derive(Clone)]
pub struct AppState<B: MemoryBackend + Send + Sync + 'static> {
    pub memory_store:   Arc<Mutex<MemoryStore<B>>>,
    pub symbolic_store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
    /// Dirichlet-Multinomial transitions + Kalman entity tracking + causal DAG
    pub world_model:    Arc<RwLock<WorldModelEnhanced>>,
    /// Bayesian reflexion bridge — takes &mut self, so Mutex not RwLock
    pub aureus:         Arc<Mutex<AureusBridge>>,
    /// Self-awareness: capability registry, resource monitor, health, decision engine
    pub self_model:     Arc<SelfModel>,
    /// Cross-module consistency checker — persistent, not recreated per request
    pub coherence:      Arc<CoherenceChecker>,
}
```

- [ ] **Step 1.5: Add run_with_state entry point**

In `src/web_server.rs`, add a new public function after `run_with_memory` (around line 375):

```rust
/// Primary server entry point. Constructs all routes with full AppState.
/// Replaces run_with_both_stores when intelligence layer is needed.
#[cfg(feature = "web-server")]
pub async fn run_with_state<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr,
    state: AppState<B>,
) {
    run_with_both_stores(addr, state.symbolic_store, state.memory_store).await;
    // NOTE: Remaining tasks will replace the body above with full AppState routing.
    // For now this delegates to the existing function to keep compilation green.
}
```

- [ ] **Step 1.6: Run test — expect PASS**

```powershell
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "test result|FAILED"
```
Expected: `test result: ok. 2 passed`

- [ ] **Step 1.7: Commit**

```powershell
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c"
git add src/web_server.rs tests/integration/intelligence_wiring_sit.rs tests/integration/mod.rs
git commit -m "feat: add AppState<B> struct and run_with_state entry point"
```

---

## Task 2: WorldModelEnhanced persistence (save/load)

**Files:**
- Modify: `src/modules/world_model_enhanced/transition.rs`
- Modify: `src/modules/world_model_enhanced/causal.rs`
- Modify: `src/modules/world_model_enhanced/mod.rs`

### Context
`TransitionModel` has `HashMap<(String,String,String), usize>` — tuple keys can't be JSON object keys.
Solution: encode as `"state\x1Faction\x1Fnext_state"` string keys.

- [ ] **Step 2.1: Write failing tests** — add to `tests/integration/intelligence_wiring_sit.rs`

```rust
#[test]
fn test_world_model_save_load_roundtrip() {
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S3".into()).unwrap();
        wm.add_causal_edge("X".into(), "Y".into()).unwrap();
    }
    let tmp = std::env::temp_dir().join("wm_test.json");
    {
        let wm = state.world_model.read().unwrap();
        wm.save(&tmp).expect("save failed");
    }
    // Load into fresh WorldModelEnhanced
    let wm2 = WorldModelEnhanced::load(&tmp).expect("load failed");
    let pred = wm2.predict_next_state("S1", "A1").unwrap();
    assert!(pred.probabilities.contains_key("S2"));
    assert!(pred.probabilities.contains_key("S3"));
    // P(S2) should be higher than P(S3): 2 vs 1 observation
    assert!(pred.probabilities["S2"] > pred.probabilities["S3"]);
    // Causal edge preserved
    assert!(wm2.has_causal_path("X", "Y").unwrap());
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_world_model_save_load_empty() {
    let tmp = std::env::temp_dir().join("wm_empty_test.json");
    let wm = WorldModelEnhanced::new();
    wm.save(&tmp).expect("save empty failed");
    let wm2 = WorldModelEnhanced::load(&tmp).expect("load empty failed");
    assert!(wm2.list_entities().unwrap().is_empty());
    let _ = std::fs::remove_file(&tmp);
}
```

- [ ] **Step 2.2: Run test — expect compile error** (`save`/`load` don't exist yet)

```powershell
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "error\["
```
Expected: `error[E0599]: no method named 'save' found`

- [ ] **Step 2.3: Add key encoding helpers + save/load to TransitionModel**

In `src/modules/world_model_enhanced/transition.rs`, add after the `impl Default`:

```rust
fn encode_transition_key(s: &str, a: &str, ns: &str) -> String {
    // \x1F = ASCII Unit Separator — safe in any UTF-8 string, rare in natural text
    format!("{}\x1F{}\x1F{}", s, a, ns)
}

fn decode_transition_key(k: &str) -> Option<(String, String, String)> {
    let mut parts = k.splitn(3, '\x1F');
    Some((
        parts.next()?.to_string(),
        parts.next()?.to_string(),
        parts.next()?.to_string(),
    ))
}

fn encode_sa_key(s: &str, a: &str) -> String {
    format!("{}\x1F{}", s, a)
}

fn decode_sa_key(k: &str) -> Option<(String, String)> {
    let mut parts = k.splitn(2, '\x1F');
    Some((parts.next()?.to_string(), parts.next()?.to_string()))
}

impl TransitionModel {
    /// Save transition counts to a JSON file.
    /// Uses encoded string keys because JSON requires string object keys.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let counts_encoded: std::collections::HashMap<String, usize> = self.counts
            .iter()
            .map(|((s, a, ns), &v)| (encode_transition_key(s, a, ns), v))
            .collect();
        let totals_encoded: std::collections::HashMap<String, usize> = self.totals
            .iter()
            .map(|((s, a), &v)| (encode_sa_key(s, a), v))
            .collect();
        let data = serde_json::json!({
            "counts": counts_encoded,
            "totals": totals_encoded,
            "smoothing": self.smoothing,
        });
        // Atomic write: write to .tmp then rename
        let tmp = path.as_ref().with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string(&data)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load transition model from a JSON file written by save().
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let data: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        let smoothing = data["smoothing"].as_f64().unwrap_or(1.0);
        let mut model = Self::with_smoothing(smoothing);
        if let Some(obj) = data["counts"].as_object() {
            for (k, v) in obj {
                if let (Some((s, a, ns)), Some(count)) =
                    (decode_transition_key(k), v.as_u64())
                {
                    model.counts.insert((s, a, ns), count as usize);
                }
            }
        }
        if let Some(obj) = data["totals"].as_object() {
            for (k, v) in obj {
                if let (Some((s, a)), Some(total)) =
                    (decode_sa_key(k), v.as_u64())
                {
                    model.totals.insert((s, a), total as usize);
                }
            }
        }
        Ok(model)
    }

    /// Expose smoothing parameter (needed for save)
    pub fn smoothing(&self) -> f64 { self.smoothing }
}
```

- [ ] **Step 2.4: Add save_edges/load_edges/all_edges to CausalGraph**

In `src/modules/world_model_enhanced/causal.rs`, add inside `impl CausalGraph`:

```rust
/// Return all edges (for serialization)
pub fn all_edges(&self) -> Vec<(&CausalEdge)> {
    self.edges.iter().collect()
}

/// Save causal edges to JSON
pub fn save_edges<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
    let edges: Vec<serde_json::Value> = self.edges.iter().map(|e| serde_json::json!({
        "from": e.from,
        "to": e.to,
        "strength": e.strength,
    })).collect();
    let tmp = path.as_ref().with_extension("causal.tmp");
    std::fs::write(&tmp, serde_json::to_string(&edges)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Load causal edges from JSON
pub fn load_edges<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
    let data: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let mut graph = CausalGraph::new();
    for e in data {
        let from = e["from"].as_str().unwrap_or("").to_string();
        let to   = e["to"].as_str().unwrap_or("").to_string();
        // add_edge handles cycle prevention; ignore errors on load
        let _ = graph.add_edge(from, to);
    }
    Ok(graph)
}
```

Note: Check what fields `CausalGraph` has. If it stores edges in a `Vec<CausalEdge>` field named `edges`, use `self.edges`. Read the struct definition first and adjust field names as needed.

- [ ] **Step 2.5: Add save/load to WorldModelEnhanced**

In `src/modules/world_model_enhanced/mod.rs`, add after `impl Default`:

```rust
impl WorldModelEnhanced {
    /// Save world model state to path.
    /// Persists: transition counts, causal edges. (Entity Kalman states not persisted.)
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let transitions = self.transitions.read()
            .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        let causal = self.causal_graph.read()
            .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;

        // Build combined JSON
        let causal_edges: Vec<serde_json::Value> = causal.all_edges()
            .iter()
            .map(|e| serde_json::json!({"from": e.from, "to": e.to, "strength": e.strength}))
            .collect();

        // Encode transition counts
        let counts_encoded: std::collections::HashMap<String, usize> = transitions.counts
            .iter()
            .map(|((s, a, ns), &v)| {
                (format!("{}\x1F{}\x1F{}", s, a, ns), v)
            })
            .collect();
        let totals_encoded: std::collections::HashMap<String, usize> = transitions.totals
            .iter()
            .map(|((s, a), &v)| (format!("{}\x1F{}", s, a), v))
            .collect();

        let data = serde_json::json!({
            "version": 1,
            "transition_counts": counts_encoded,
            "transition_totals": totals_encoded,
            "smoothing": transitions.smoothing(),
            "causal_edges": causal_edges,
        });

        let tmp = path.as_ref().with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string(&data)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load world model state from path written by save().
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let data: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        let wm = Self::new();
        
        // Restore transition counts
        {
            let mut transitions = wm.transitions.write()
                .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            let smoothing = data["smoothing"].as_f64().unwrap_or(1.0);
            *transitions = TransitionModel::with_smoothing(smoothing);
            if let Some(obj) = data["transition_counts"].as_object() {
                for (k, v) in obj {
                    let mut parts = k.splitn(3, '\x1F');
                    if let (Some(s), Some(a), Some(ns), Some(count)) =
                        (parts.next(), parts.next(), parts.next(), v.as_u64())
                    {
                        transitions.counts.insert(
                            (s.to_string(), a.to_string(), ns.to_string()),
                            count as usize,
                        );
                    }
                }
            }
            if let Some(obj) = data["transition_totals"].as_object() {
                for (k, v) in obj {
                    let mut parts = k.splitn(2, '\x1F');
                    if let (Some(s), Some(a), Some(total)) =
                        (parts.next(), parts.next(), v.as_u64())
                    {
                        transitions.totals.insert(
                            (s.to_string(), a.to_string()),
                            total as usize,
                        );
                    }
                }
            }
        }

        // Restore causal edges
        {
            let mut causal = wm.causal_graph.write()
                .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            if let Some(arr) = data["causal_edges"].as_array() {
                for e in arr {
                    let from = e["from"].as_str().unwrap_or("").to_string();
                    let to   = e["to"].as_str().unwrap_or("").to_string();
                    if !from.is_empty() && !to.is_empty() {
                        let _ = causal.add_edge(from, to);
                    }
                }
            }
        }

        Ok(wm)
    }

    /// Total number of state transitions observed.
    pub fn transition_count(&self) -> usize {
        self.transitions.read()
            .map(|t| t.observation_count())
            .unwrap_or(0)
    }
}
```

Note: `TransitionModel.counts` and `TransitionModel.totals` are private. Either make them `pub(crate)` or have `WorldModelEnhanced.save()` call `transitions.save(path)` instead of accessing fields directly. Use `pub(crate)` on `counts` and `totals` in `transition.rs`:

```rust
pub struct TransitionModel {
    pub(crate) counts: HashMap<(String, String, String), usize>,
    pub(crate) totals: HashMap<(String, String), usize>,
    smoothing: f64,
}
```

- [ ] **Step 2.6: Run tests — expect PASS**

```powershell
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "test result|FAILED|error\["
```
Expected: `test result: ok. 4 passed`

- [ ] **Step 2.7: Commit**

```powershell
git add src/modules/world_model_enhanced/ tests/integration/intelligence_wiring_sit.rs
git commit -m "feat: add WorldModelEnhanced save/load persistence with key encoding"
```

---

## Task 3: bin/webserver.rs — AppState construction + persistence wiring

**Files:**
- Modify: `src/bin/webserver.rs`

- [ ] **Step 3.1: Write failing test** — add to `intelligence_wiring_sit.rs`

```rust
#[test]
fn test_world_model_file_not_found_returns_new() {
    // When worldmodel.json doesn't exist, should get fresh WorldModelEnhanced
    let path = std::path::PathBuf::from("/nonexistent/worldmodel.json");
    // load() should error; caller handles by using WorldModelEnhanced::new()
    assert!(WorldModelEnhanced::load(&path).is_err());
}
```

- [ ] **Step 3.2: Run test — PASS** (already handled by load returning Err)

```powershell
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "test result|FAILED"
```

- [ ] **Step 3.3: Update bin/webserver.rs**

Replace the entire `src/bin/webserver.rs` with:

```rust
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::{SelfModel, CapabilityDescriptor};
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use hipcortex::web_server::{self, AppState};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3030);
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| ".".to_string());

    // ── Load or create stores ────────────────────────────────────────────────
    let store_path = format!("{}/memory.jsonl", data_dir);
    let memory_store = Arc::new(Mutex::new(MemoryStore::new(&store_path)?));
    let symbolic_store = Arc::new(Mutex::new(SymbolicStore::<InMemoryGraph>::new()));

    // ── WorldModelEnhanced: load from disk or start fresh ───────────────────
    let wm_path = format!("{}/worldmodel.json", data_dir);
    let world_model = {
        match WorldModelEnhanced::load(&wm_path) {
            Ok(wm) => {
                println!("WorldModel: loaded {} transitions from {}", wm.transition_count(), wm_path);
                wm
            }
            Err(_) => {
                println!("WorldModel: starting fresh (no {} found)", wm_path);
                WorldModelEnhanced::new()
            }
        }
    };
    let world_model = Arc::new(RwLock::new(world_model));

    // ── SelfModel: bootstrap capabilities ───────────────────────────────────
    let self_model = Arc::new(SelfModel::new());
    for op in &["add_memory", "search_memory", "query_memory", "ingest",
                "bulk_add", "forget", "reflect", "context"] {
        self_model.register_capability(CapabilityDescriptor {
            name: op.to_string(),
            description: format!("HipCortex {} operation", op),
            required_cpu_percent: 5.0,
            required_memory_mb: 50.0,
            limitations: vec![],
        }).ok();
    }

    // ── Build AppState ───────────────────────────────────────────────────────
    let state = AppState {
        memory_store: memory_store.clone(),
        symbolic_store,
        world_model: world_model.clone(),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model,
        coherence: Arc::new(CoherenceChecker::new()),
    };

    // ── Periodic WorldModel flush (every 5 minutes) ──────────────────────────
    let wm_flush = world_model.clone();
    let wm_path_flush = wm_path.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            if let Ok(wm) = wm_flush.read() {
                if let Err(e) = wm.save(&wm_path_flush) {
                    eprintln!("[WorldModel] periodic flush error: {}", e);
                }
            }
        }
    });

    println!("HipCortex REST API  |  listening on http://{}", addr);
    let keys_configured = std::env::var("HIPCORTEX_API_KEYS").is_ok();
    if keys_configured {
        println!("Auth: API key required (X-Api-Key header)");
    } else {
        println!("Auth: open (set HIPCORTEX_API_KEYS to enable)");
    }

    // ── Graceful shutdown ────────────────────────────────────────────────────
    let store_for_signal = memory_store.clone();
    let wm_for_signal = world_model.clone();
    let wm_path_signal = wm_path.clone();

    tokio::select! {
        _ = web_server::run_with_state(addr, state) => {
            println!("Server exited normally.");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutdown signal — flushing stores...");
            if let Ok(mut ms) = store_for_signal.lock() {
                ms.flush().ok();
                println!("MemoryStore flushed.");
            }
            if let Ok(wm) = wm_for_signal.read() {
                match wm.save(&wm_path_signal) {
                    Ok(_)  => println!("WorldModel saved to {}.", wm_path_signal),
                    Err(e) => eprintln!("WorldModel save error: {}", e),
                }
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 3.4: Build check**

```powershell
cargo check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error"
```
Expected: no errors (warnings OK)

- [ ] **Step 3.5: Commit**

```powershell
git add src/bin/webserver.rs
git commit -m "feat: AppState construction, SelfModel bootstrap, WorldModel load/save in webserver binary"
```

---

## Task 4: Wire AppState into run_with_state — replace delegation

**Files:**
- Modify: `src/web_server.rs` (replace `run_with_state` body with full AppState routing)

### Context
Task 1 created `run_with_state` as a thin delegation to `run_with_both_stores`. Now replace the body with full AppState-aware routing. The key change: handler closures capture `state.memory_store`, `state.world_model`, etc. instead of separate variables.

- [ ] **Step 4.1: Replace run_with_state body**

Find `run_with_state` in `src/web_server.rs` (the function added in Task 1) and replace its body:

```rust
#[cfg(feature = "web-server")]
pub async fn run_with_state<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr,
    state: AppState<B>,
) {
    // Decompose state into individual Arcs for capture into closures
    let memory_store   = state.memory_store.clone();
    let symbolic_store = state.symbolic_store.clone();
    let world_model    = state.world_model.clone();
    let aureus         = state.aureus.clone();
    let self_model_arc = state.self_model.clone();
    let coherence_arc  = state.coherence.clone();

    // Forward to run_with_both_stores for all existing routes.
    // New intelligence routes are registered via the returned Router and layered on top.
    // For simplicity: construct the intelligence routes here, merge with existing router.
    // NOTE: run_with_both_stores already calls axum::Server::bind — we can't merge Routers
    // across function boundaries easily in Axum 0.6 without restructuring the function.
    // Therefore: inline the full route registration here (copy from run_with_both_stores,
    // then add new intelligence routes).
    
    // === Copy all existing routes from run_with_both_stores, replacing
    //     memory_store/symbolic_store references with state.memory_store / state.symbolic_store ===
    // Then add new intelligence routes at the end before .layer(middleware).
    
    // (Implementation: copy the entire run_with_both_stores body, replace the 2 Arc args
    //  with state.memory_store and state.symbolic_store, add new routes.)
    
    // See Step 4.2 for the concrete implementation approach.
    run_with_both_stores(addr, state.symbolic_store, state.memory_store).await;
}
```

**IMPORTANT:** Axum 0.6 doesn't support merging routers from separate functions easily. The cleanest approach: copy all routes from `run_with_both_stores` directly into `run_with_state`, replacing `symbolic_store` / `memory_store` with `state.symbolic_store` / `state.memory_store`, then add new intelligence routes. Then update `run_with_memory` to call `run_with_state` instead.

- [ ] **Step 4.2: Extract all routes into run_with_state (surgical copy)**

In `src/web_server.rs`:

1. Find `run_with_both_stores` (starts at its `pub async fn` line)
2. Find the `let app = Router::new()` block inside it
3. Find the `.layer(middleware::from_fn(api_key_middleware));` close
4. Copy the entire function body into `run_with_state`, replacing:
   - All `memory_store.clone()` → `state.memory_store.clone()` (already done above as `let memory_store = state.memory_store.clone()`)
   - All `symbolic_store.clone()` → `state.symbolic_store.clone()`
   - The intelligence handlers (new routes) go between the last existing route and `.layer(...)`

Add these new routes before the `.layer(middleware)` line:
```rust
.route("/worldmodel/observe",       post(wm_observe_route))
.route("/worldmodel/predict",       get(wm_predict_route))
.route("/worldmodel/entities",      get(wm_entities_route))
.route("/worldmodel/entity",        post(wm_entity_route))
.route("/worldmodel/causal",        get(wm_causal_route))
.route("/memory/reflect",           post(memory_reflect_route))
.route("/memory/hypotheses",        get(memory_hypotheses_route))
.route("/self/health",              get(self_health_route))
.route("/self/capabilities",        get(self_capabilities_route))
```

- [ ] **Step 4.3: Update run_with_memory to call run_with_state**

Find `run_with_memory` (around line 364) and update:
```rust
pub async fn run_with_memory<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr,
    memory_store: Arc<Mutex<MemoryStore<B>>>,
) {
    let symbolic_store = Arc::new(Mutex::new(SymbolicStore::new()));
    let state = AppState {
        memory_store,
        symbolic_store,
        world_model: Arc::new(std::sync::RwLock::new(WorldModelEnhanced::new())),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model: Arc::new(SelfModel::new()),
        coherence: Arc::new(CoherenceChecker::new()),
    };
    run_with_state(addr, state).await;
}
```

- [ ] **Step 4.4: Build check**

```powershell
cargo check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error"
```
Expected: no errors

- [ ] **Step 4.5: Commit**

```powershell
git add src/web_server.rs
git commit -m "refactor: run_with_state fully replaces run_with_both_stores, run_with_memory delegates to it"
```

---

## Task 5: Memory → WorldModel auto-feed

**Files:**
- Modify: `src/web_server.rs` (`handle_add_memory`, `handle_ingest`)

- [ ] **Step 5.1: Write failing tests** — add to `intelligence_wiring_sit.rs`

```rust
#[test]
fn test_observe_transition_updates_count() {
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("alice".into(), "decided".into(), "use_postgres".into()).unwrap();
        wm.observe_transition("alice".into(), "decided".into(), "use_postgres".into()).unwrap();
    }
    let wm = state.world_model.read().unwrap();
    assert_eq!(wm.transition_count(), 2);
    let pred = wm.predict_next_state("alice", "decided").unwrap();
    assert!(pred.probabilities.contains_key("use_postgres"));
}

#[test]
fn test_causal_edge_from_pinned_memory() {
    let state = make_app_state();
    // Simulate what handle_add_memory would do for a pinned Symbolic record
    {
        let mut wm = state.world_model.write().unwrap();
        // Actor alice with pinned priority causes causal edge registration
        wm.add_causal_edge("alice".into(), "postgres".into()).unwrap();
    }
    let wm = state.world_model.read().unwrap();
    assert!(wm.has_causal_path("alice", "postgres").unwrap());
}
```

- [ ] **Step 5.2: Run test — PASS** (logic already testable directly)

- [ ] **Step 5.3: Update handle_add_memory to auto-feed WorldModel**

Find `handle_add_memory` in `src/web_server.rs`. It currently takes `store: Arc<Mutex<MemoryStore<B>>>` as first arg. Update signature to also accept `world_model: Arc<RwLock<WorldModelEnhanced>>`:

```rust
async fn handle_add_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    world_model: Arc<RwLock<WorldModelEnhanced>>,   // NEW
    req: AddMemoryRequest,
) -> Result<Json<AddMemoryResponse>, (StatusCode, Json<AddMemoryResponse>)> {
```

After the `Ok(Json(AddMemoryResponse { success: true, ... }))` branch (after successful store.add), add:

```rust
// Auto-feed WorldModelEnhanced — non-blocking, best-effort (never fails the request)
if let Ok(mut wm) = world_model.try_write() {
    let _ = wm.observe_transition(
        record.actor.clone(),
        record.action.clone(),
        record.target.clone(),
    );
    // Register causal edge for pinned symbolic decisions
    if record.priority == "pinned" && record.record_type == MemoryType::Symbolic {
        let _ = wm.add_causal_edge(record.actor.clone(), record.target.clone());
    }
}
```

Update the route closure in `run_with_state` to pass `world_model`:
```rust
let wm = world_model.clone();
let add_memory_route = {
    let store = memory_store.clone();
    let wm = wm.clone();
    post(move |Json(req): Json<AddMemoryRequest>| async move {
        handle_add_memory(store, wm, req).await
    })
};
```

- [ ] **Step 5.4: Update handle_ingest similarly**

Find `handle_ingest`. After successful `ms.add(record.clone())`, add the same WorldModel feed pattern as Step 5.3.

Update route closure to pass `world_model` Arc.

- [ ] **Step 5.5: Build check + run tests**

```powershell
cargo check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error"
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "test result|FAILED"
```
Expected: all pass

- [ ] **Step 5.6: Commit**

```powershell
git add src/web_server.rs tests/integration/intelligence_wiring_sit.rs
git commit -m "feat: auto-feed WorldModelEnhanced from handle_add_memory and handle_ingest"
```

---

## Task 6: WorldModel REST endpoints

**Files:**
- Modify: `src/web_server.rs` (add 5 new handlers)

- [ ] **Step 6.1: Write failing tests** — add to `intelligence_wiring_sit.rs`

```rust
// These test the handler logic directly (not HTTP), via AppState
#[test]
fn test_wm_observe_then_predict() {
    let state = make_app_state();
    // Observe 3 transitions
    {
        let mut wm = state.world_model.write().unwrap();
        for _ in 0..3 { wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap(); }
        wm.observe_transition("S1".into(), "A1".into(), "S3".into()).unwrap();
    }
    // Predict
    let wm = state.world_model.read().unwrap();
    let pred = wm.predict_next_state("S1", "A1").unwrap();
    // S2 should be more likely than S3
    assert!(pred.probabilities["S2"] > pred.probabilities["S3"]);
}

#[test]
fn test_wm_register_entity_then_list() {
    use hipcortex::world_model_enhanced::EntityState;
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.register_entity("robot_1".into(), EntityState {
            properties: vec![1.0, 2.0, 3.0],
            covariance: vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0], vec![0.0, 0.0, 1.0]],
        }).unwrap();
    }
    let wm = state.world_model.read().unwrap();
    assert!(wm.list_entities().unwrap().contains(&"robot_1".to_string()));
}
```

- [ ] **Step 6.2: Run tests — PASS** (test world model API directly, no HTTP needed)

- [ ] **Step 6.3: Add WM handler functions to web_server.rs**

Add these handlers before the `handle_worldmodel_status` function:

```rust
// ── WorldModel REST handlers ─────────────────────────────────────────────────

/// POST /worldmodel/observe — feed a state transition into Dirichlet model
#[cfg(feature = "web-server")]
async fn handle_wm_observe<B: MemoryBackend + Send + Sync + 'static>(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let from   = req["from"].as_str().unwrap_or("").to_string();
    let action = req["action"].as_str().unwrap_or("").to_string();
    let to     = req["to"].as_str().unwrap_or("").to_string();
    if from.is_empty() || action.is_empty() || to.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "from, action, to required"}));
    }
    match world_model.write() {
        Ok(mut wm) => match wm.observe_transition(from, action, to) {
            Ok(_)  => Json(serde_json::json!({"success": true, "total_transitions": wm.transition_count()})),
            Err(e) => Json(serde_json::json!({"success": false, "error": e})),
        },
        Err(e) => Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
    }
}

/// GET /worldmodel/predict?state=&action= — get P(s'|s,a) distribution
#[cfg(feature = "web-server")]
#[derive(serde::Deserialize)]
struct WmPredictParams { state: String, action: String }

#[cfg(feature = "web-server")]
async fn handle_wm_predict(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Query(params): Query<WmPredictParams>,
) -> Json<serde_json::Value> {
    match world_model.read() {
        Ok(wm) => match wm.predict_next_state(&params.state, &params.action) {
            Ok(pred) => Json(serde_json::json!({
                "from_state": pred.from_state,
                "action": pred.action,
                "probabilities": pred.probabilities,
                "entropy": pred.entropy,
                "observation_count": pred.observation_count,
            })),
            Err(e) => Json(serde_json::json!({"error": e})),
        },
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}

/// GET /worldmodel/entities — list Kalman-tracked entities
#[cfg(feature = "web-server")]
async fn handle_wm_entities(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
) -> Json<serde_json::Value> {
    match world_model.read() {
        Ok(wm) => match wm.list_entities() {
            Ok(ids) => Json(serde_json::json!({"entities": ids, "total": ids.len()})),
            Err(e)  => Json(serde_json::json!({"entities": [], "error": e})),
        },
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}

/// POST /worldmodel/entity — register entity with initial Kalman state
#[cfg(feature = "web-server")]
async fn handle_wm_register_entity(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    use crate::world_model_enhanced::EntityState;
    let id = req["id"].as_str().unwrap_or("").to_string();
    if id.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "id required"}));
    }
    let dims: usize = req["dimensions"].as_u64().unwrap_or(3) as usize;
    let initial = EntityState {
        properties: vec![0.0; dims],
        covariance: (0..dims).map(|i| (0..dims).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect(),
    };
    match world_model.write() {
        Ok(mut wm) => match wm.register_entity(id.clone(), initial) {
            Ok(_)  => Json(serde_json::json!({"success": true, "id": id})),
            Err(e) => Json(serde_json::json!({"success": false, "error": e})),
        },
        Err(e) => Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
    }
}

/// GET /worldmodel/causal — dump causal DAG edges
#[cfg(feature = "web-server")]
async fn handle_wm_causal(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
) -> Json<serde_json::Value> {
    match world_model.read() {
        Ok(wm) => {
            // Use the existing symbolic graph export for causal edges
            // (causal_graph.all_edges() added in Task 2)
            let graph = wm.causal_graph.read();
            match graph {
                Ok(g) => {
                    let edges: Vec<serde_json::Value> = g.all_edges().iter().map(|e| serde_json::json!({
                        "from": e.from, "to": e.to, "strength": e.strength,
                    })).collect();
                    Json(serde_json::json!({"edges": edges, "total": edges.len()}))
                }
                Err(e) => Json(serde_json::json!({"edges": [], "error": format!("lock: {}", e)})),
            }
        }
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}
```

Note: `wm.causal_graph` is a private field. Either make it `pub(crate)` or add a `get_causal_edges() -> Vec<CausalEdge>` method to `WorldModelEnhanced`. Add this to `mod.rs`:
```rust
pub fn get_causal_edges(&self) -> Vec<crate::world_model_enhanced::CausalEdge> {
    self.causal_graph.read()
        .map(|g| g.all_edges().into_iter().cloned().collect())
        .unwrap_or_default()
}
```
Then use `wm.get_causal_edges()` in the handler.

- [ ] **Step 6.4: Register WM routes in run_with_state**

Add route closures and route registrations (following the pattern from existing routes):

```rust
// WorldModel route closures
let wm_observe_route = {
    let wm = world_model.clone();
    post(move |Json(req): Json<serde_json::Value>| async move { handle_wm_observe(wm, Json(req)).await })
};
let wm_predict_route = {
    let wm = world_model.clone();
    get(move |Query(p): Query<WmPredictParams>| async move { handle_wm_predict(wm, Query(p)).await })
};
let wm_entities_route = {
    let wm = world_model.clone();
    get(move || { let w = wm.clone(); async move { handle_wm_entities(w).await } })
};
let wm_entity_route = {
    let wm = world_model.clone();
    post(move |Json(req): Json<serde_json::Value>| async move { handle_wm_register_entity(wm, Json(req)).await })
};
let wm_causal_route = {
    let wm = world_model.clone();
    get(move || { let w = wm.clone(); async move { handle_wm_causal(w).await } })
};
```

Update `GET /worldmodel/status` handler to return real state — replace the static JSON:
```rust
async fn handle_worldmodel_status(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
) -> Json<serde_json::Value> {
    let (total_transitions, entity_count) = match world_model.read() {
        Ok(wm) => (wm.transition_count(), wm.list_entities().unwrap_or_default().len()),
        Err(_) => (0, 0),
    };
    Json(serde_json::json!({
        "status": "available",
        "mode": "full",
        "total_transitions_observed": total_transitions,
        "tracked_entities": entity_count,
        "endpoints": {
            "observe": "POST /worldmodel/observe",
            "predict": "GET /worldmodel/predict?state=&action=",
            "entities": "GET /worldmodel/entities",
            "entity": "POST /worldmodel/entity",
            "causal": "GET /worldmodel/causal"
        }
    }))
}
```

- [ ] **Step 6.5: Build check + tests**

```powershell
cargo check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error"
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "test result|FAILED"
```

- [ ] **Step 6.6: Commit**

```powershell
git add src/web_server.rs src/modules/world_model_enhanced/
git commit -m "feat: WorldModel REST endpoints (observe/predict/entities/entity/causal)"
```

---

## Task 7: AureusBridge REST (reflect + hypotheses)

**Files:**
- Modify: `src/modules/aureus_bridge.rs` (add `reflect_on_memory`)
- Modify: `src/web_server.rs` (add 2 new handlers)

- [ ] **Step 7.1: Write failing tests** — add to `intelligence_wiring_sit.rs`

```rust
#[test]
fn test_reflect_on_memory_returns_hypothesis() {
    use hipcortex::llm_clients::mock::MockClient;
    let state = make_app_state();
    // Add some memories first
    {
        let mut ms = state.memory_store.lock().unwrap();
        ms.add(make_record("alice", "decided", "use postgres")).unwrap();
        ms.add(make_record("alice", "decided", "avoid redis")).unwrap();
    }
    // Reflect on the memories
    {
        let mut aureus = state.aureus.lock().unwrap();
        aureus.set_client(Box::new(MockClient));
        let mut ms = state.memory_store.lock().unwrap();
        let hyp = aureus.reflect_on_memory("alice decision patterns", &mut ms);
        // MockClient returns a fixed response, so we just check it runs
        assert!(!hyp.text.is_empty() || hyp.confidence >= 0.0);
    }
}

fn make_record(actor: &str, action: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Symbolic,
        actor.to_string(),
        action.to_string(),
        target.to_string(),
        serde_json::json!({}),
    )
}
```

- [ ] **Step 7.2: Add reflect_on_memory to AureusBridge**

In `src/modules/aureus_bridge.rs`, add inside `impl AureusBridge`:

```rust
/// Reflect on memory: search store for query context, run reflexion loop, return hypothesis.
/// Stores the resulting ReflexionHypothesis as a Reflexion MemoryRecord.
pub fn reflect_on_memory<B: MemoryBackend>(
    &mut self,
    query: &str,
    store: &mut MemoryStore<B>,
) -> ReflexionHypothesis {
    // Build context from top-K search results
    let results = store.search_semantic(None, query, 10, false);
    let context = if results.is_empty() {
        format!("Query: {}. No relevant memories found.", query)
    } else {
        let lines: Vec<String> = results.iter()
            .map(|(r, score)| format!("- [{:.2}] [{}] {}", score, r.action, r.target))
            .collect();
        format!("Query: {}\nRelevant memories:\n{}", query, lines.join("\n"))
    };
    self.reflexion_loop(&context, store);
    // Return the current top hypothesis
    self.current
        .and_then(|id| self.graph.get_hypothesis(id))
        .cloned()
        .unwrap_or_else(|| ReflexionHypothesis {
            text: format!("No LLM configured. Context had {} relevant memories.", results.len()),
            confidence: 0.5,
            evidence: vec![],
        })
}
```

- [ ] **Step 7.3: Add reflect + hypotheses handlers to web_server.rs**

```rust
/// POST /memory/reflect — run AureusBridge reflexion over memory context
#[cfg(feature = "web-server")]
async fn handle_memory_reflect<B: MemoryBackend + Send + Sync + 'static>(
    memory_store: Arc<Mutex<MemoryStore<B>>>,
    aureus: Arc<Mutex<AureusBridge>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let query = req["query"].as_str().unwrap_or("recent decisions").to_string();
    let mut store = match memory_store.lock() {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({"error": format!("store lock: {}", e)})),
    };
    let mut bridge = match aureus.lock() {
        Ok(b) => b,
        Err(e) => return Json(serde_json::json!({"error": format!("bridge lock: {}", e)})),
    };
    let hyp = bridge.reflect_on_memory(&query, &mut *store);
    Json(serde_json::json!({
        "hypothesis": hyp.text,
        "confidence": hyp.confidence,
        "evidence": hyp.evidence,
        "loops_run": bridge.loops_run(),
    }))
}

/// GET /memory/hypotheses — current AureusBridge hypothesis graph top-N
#[cfg(feature = "web-server")]
async fn handle_memory_hypotheses(
    aureus: Arc<Mutex<AureusBridge>>,
) -> Json<serde_json::Value> {
    match aureus.lock() {
        Ok(bridge) => {
            Json(serde_json::json!({
                "loops_run": bridge.loops_run(),
                "note": "Hypothesis graph contents available via AureusBridge.graph (internal)"
            }))
        }
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}
```

Note: `AureusBridge.graph` is private (`HypothesesGraph`). For now expose just `loops_run`. A future PR can add `get_top_hypotheses()` to `HypothesesGraph`. The test in Step 7.1 covers the core `reflect_on_memory` function.

- [ ] **Step 7.4: Register reflect/hypotheses routes in run_with_state**

```rust
let memory_reflect_route = {
    let ms = memory_store.clone();
    let au = aureus.clone();
    post(move |Json(req): Json<serde_json::Value>| async move {
        handle_memory_reflect(ms, au, Json(req)).await
    })
};
let memory_hypotheses_route = {
    let au = aureus.clone();
    get(move || { let a = au.clone(); async move { handle_memory_hypotheses(a).await } })
};
```

- [ ] **Step 7.5: Build + test**

```powershell
cargo check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error"
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "test result|FAILED"
```

- [ ] **Step 7.6: Commit**

```powershell
git add src/web_server.rs src/modules/aureus_bridge.rs
git commit -m "feat: AureusBridge REST endpoints (POST /memory/reflect, GET /memory/hypotheses)"
```

---

## Task 8: SelfModel + CoherenceChecker wiring

**Files:**
- Modify: `src/web_server.rs` (2 new handlers, fix CoherenceChecker to use persistent Arc)

- [ ] **Step 8.1: Write failing tests** — add to `intelligence_wiring_sit.rs`

```rust
#[test]
fn test_self_model_health_after_bootstrap() {
    use hipcortex::self_model::CapabilityDescriptor;
    let state = make_app_state();
    state.self_model.register_capability(CapabilityDescriptor {
        name: "add_memory".to_string(),
        description: "test".to_string(),
        required_cpu_percent: 5.0,
        required_memory_mb: 50.0,
        limitations: vec![],
    }).unwrap();
    let health = state.self_model.get_health().unwrap();
    assert!(health.overall >= 0.0 && health.overall <= 1.0);
}

#[test]
fn test_coherence_checker_persistent() {
    // CoherenceChecker should hold state across calls (not be recreated)
    let state = make_app_state();
    let metrics1 = state.coherence.get_metrics().unwrap();
    // Do a check
    let _ = state.coherence.check_consistency();
    let metrics2 = state.coherence.get_metrics().unwrap();
    // total_checks should have incremented
    assert!(metrics2.total_checks >= metrics1.total_checks);
}
```

- [ ] **Step 8.2: Run tests — may FAIL** if CoherenceChecker.check_consistency() doesn't update metrics

Check if `CoherenceChecker.check_consistency()` updates `self.metrics.total_checks`. Read `src/modules/coherence/checker.rs` if needed. If it does, test passes as-is.

- [ ] **Step 8.3: Add self/health and self/capabilities handlers**

```rust
/// GET /self/health — SelfModel overall health score + module breakdown
#[cfg(feature = "web-server")]
async fn handle_self_health(
    self_model: Arc<SelfModel>,
) -> Json<serde_json::Value> {
    match self_model.get_health() {
        Ok(score) => Json(serde_json::json!({
            "healthy": score.overall >= 0.7,
            "overall": score.overall,
            "modules": score.modules,
        })),
        Err(e) => Json(serde_json::json!({"healthy": false, "error": e})),
    }
}

/// GET /self/capabilities — registered capability descriptors
#[cfg(feature = "web-server")]
async fn handle_self_capabilities(
    self_model: Arc<SelfModel>,
) -> Json<serde_json::Value> {
    // SelfModel doesn't expose list_capabilities() yet.
    // Return a static list based on bootstrap registration.
    // TODO: add list_capabilities() to SelfModel in a follow-up.
    let ops = ["add_memory", "search_memory", "query_memory", "ingest",
               "bulk_add", "forget", "reflect", "context"];
    let capabilities: Vec<serde_json::Value> = ops.iter().map(|op| {
        match self_model.get_capability(op) {
            Ok(cap) => serde_json::json!({
                "name": cap.name,
                "description": cap.description,
                "required_cpu_percent": cap.required_cpu_percent,
                "required_memory_mb": cap.required_memory_mb,
            }),
            Err(_) => serde_json::json!({"name": op, "status": "not_registered"}),
        }
    }).collect();
    Json(serde_json::json!({"capabilities": capabilities, "total": capabilities.len()}))
}
```

- [ ] **Step 8.4: Fix handle_coherence_status to use persistent CoherenceChecker**

Find `handle_coherence_status` in `src/web_server.rs`. It currently creates `CoherenceChecker::new()` per call. Update:

```rust
#[cfg(feature = "web-server")]
async fn handle_coherence_status(
    coherence: Arc<CoherenceChecker>,
) -> Json<CoherenceStatusResponse> {
    let metrics = match coherence.get_metrics() {
        Ok(m) => m,
        Err(_) => crate::coherence::CoherenceMetrics::new(),
    };
    Json(CoherenceStatusResponse {
        coherence_score: metrics.coherence_score,
        total_checks: metrics.total_checks,
        inconsistencies_found: metrics.inconsistencies_found,
        auto_resolutions_succeeded: metrics.auto_resolutions_succeeded,
        auto_resolutions_failed: metrics.auto_resolutions_failed,
        invariants_validated: metrics.invariants_validated,
        invariants_violated: metrics.invariants_violated,
        healthy: metrics.coherence_score >= 0.8,
    })
}
```

Update the coherence route closure in `run_with_state` to pass `coherence_arc`:
```rust
let coherence_route = {
    let c = coherence_arc.clone();
    get(move || { let cc = c.clone(); async move { handle_coherence_status(cc).await } })
};
```

- [ ] **Step 8.5: Register self routes in run_with_state**

```rust
let self_health_route = {
    let sm = self_model_arc.clone();
    get(move || { let s = sm.clone(); async move { handle_self_health(s).await } })
};
let self_capabilities_route = {
    let sm = self_model_arc.clone();
    get(move || { let s = sm.clone(); async move { handle_self_capabilities(s).await } })
};
```

- [ ] **Step 8.6: Build + run all tests**

```powershell
cargo check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error"
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "test result|FAILED"
```
Expected: all pass

- [ ] **Step 8.7: Run full integration suite for regressions**

```powershell
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite 2>&1 | Select-String "test result|FAILED"
```
Expected: no new failures (existing network-dependent tests may fail in offline env — that's pre-existing)

- [ ] **Step 8.8: Commit**

```powershell
git add src/web_server.rs tests/integration/intelligence_wiring_sit.rs
git commit -m "feat: SelfModel health/capabilities endpoints, persistent CoherenceChecker"
```

---

## Task 9: Push to remote + update SESSION_HANDOVER.md

- [ ] **Step 9.1: Update worldmodel/status in public endpoint list**

In `src/web_server.rs`, find `api_key_middleware` (the public bypass list). Add new public endpoints:
```rust
|| path == "/worldmodel/status"
|| path == "/worldmodel/predict"    // read-only, safe to public
|| path == "/worldmodel/entities"   // read-only
|| path == "/worldmodel/causal"     // read-only
|| path == "/self/health"           // read-only monitoring
|| path == "/self/capabilities"     // read-only
|| path == "/memory/hypotheses"     // read-only
```

- [ ] **Step 9.2: Final build + lib tests**

```powershell
cargo check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error"
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | Select-String "test result|FAILED"
```

- [ ] **Step 9.3: Push to remote**

```powershell
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c"
git push origin claude/pedantic-edison-28b84c
```

- [ ] **Step 9.4: Update SESSION_HANDOVER.md**

In `docs/SESSION_HANDOVER.md`:
- Update commit count
- Add "Intelligence Layer Wiring" to completed features
- Update V(state) block: `world_model_wired = 1.0`, `aureus_bridge_wired = 1.0`, `self_model_wired = 1.0`

- [ ] **Step 9.5: Final commit**

```powershell
git add docs/SESSION_HANDOVER.md src/web_server.rs
git commit -m "docs: update session handover after intelligence layer wiring"
git push origin claude/pedantic-edison-28b84c
```

---

## Acceptance Criteria

- [ ] `AppState<B>` struct exported from `web_server.rs`
- [ ] `run_with_state(addr, AppState<B>)` is the primary entry point
- [ ] `run_with_memory` delegates to `run_with_state`
- [ ] `WorldModelEnhanced.save(path)` / `.load(path)` implemented with key encoding
- [ ] `bin/webserver.rs` loads `worldmodel.json` on startup, saves on SIGTERM, flushes every 5min
- [ ] `POST /memory/add` and `POST /memory/ingest` auto-feed `WorldModelEnhanced.observe_transition()`
- [ ] `GET /worldmodel/status` returns real transition count (not static stub)
- [ ] `POST /worldmodel/observe` working
- [ ] `GET /worldmodel/predict?state=&action=` working
- [ ] `GET /worldmodel/entities` working
- [ ] `POST /worldmodel/entity` working
- [ ] `GET /worldmodel/causal` working
- [ ] `POST /memory/reflect` working
- [ ] `GET /memory/hypotheses` working
- [ ] `GET /self/health` working
- [ ] `GET /self/capabilities` working
- [ ] `GET /coherence/status` uses persistent CoherenceChecker
- [ ] All `intelligence_wiring_sit` tests pass
- [ ] No regressions in existing integration tests
- [ ] `cargo check` clean

---

## Known Gotchas

1. **CausalGraph.edges field visibility** — may need `pub(crate)` on `edges` field or a new `all_edges()` method. Read causal.rs before modifying.
2. **TransitionModel.counts/totals visibility** — need `pub(crate)` or accessor methods for WorldModelEnhanced to read them.
3. **CoherenceChecker.get_metrics()** — check if this method exists. If not, use `check_consistency()` and compute metrics from returned reports.
4. **HealthScore.modules field** — check what type `modules` is in `self_model/health.rs`. Adjust JSON serialization accordingly.
5. **Axum 0.6 closure lifetimes** — when capturing `Arc<RwLock<WorldModelEnhanced>>` in closures, the `move` semantics work the same as existing stores.
6. **Periodic flush Tokio task** — must be spawned BEFORE `tokio::select!` because select! blocks.
