# Design: Intelligence Layer Wiring

**Date:** 2026-06-04  
**Status:** Approved  
**Source:** Sessions 2026-05-30 → 2026-06-04 (opsx-explore + brainstorming)

---

## Problem

HipCortex has three fully-implemented intelligence modules that are **completely disconnected** from the REST API and from each other:

- `WorldModelEnhanced` — Dirichlet-Multinomial transitions, Kalman entity tracking, causal DAG, do-calculus. **Zero call sites outside its own tests.**
- `AureusBridge` — Bayesian belief update, reflexion loop, Monte Carlo hypothesis sampling. **Only wired into GUI (Tauri) and MCP combo-server — not the main web server.**
- `SelfModel` — Capability registry, resource monitor, health aggregator, decision engine. **Only exported from `lib.rs`. Zero call sites anywhere.**
- `CoherenceChecker` — Inconsistency detection + resolution. **Instantiated fresh per request in web_server.rs (stateless, accumulates nothing).**

The REST server (`src/web_server.rs`) currently wires only `MemoryStore<B>` and `SymbolicStore`. The intelligence layer was built (under `openspec/changes/intelligence-foundation/`) but never connected.

---

## Objectives

1. **Wire AppState** — introduce `AppState<B>` struct holding all stores + intelligence components, replacing the flat 2-arg `run_with_both_stores` signature.
2. **Memory → WorldModel auto-feed** — every `POST /memory/add` and `POST /memory/ingest` automatically feeds `WorldModelEnhanced.observe_transition(actor, action, target)`.
3. **WorldModel REST** — expose real WorldModelEnhanced state via `POST /worldmodel/observe`, `GET /worldmodel/predict`, `GET /worldmodel/entities`, `POST /worldmodel/entity`, `GET /worldmodel/causal`.
4. **AureusBridge REST** — `POST /memory/reflect` searches memory for context → reflexion_loop → store ReflexionHypothesis; `GET /memory/hypotheses` returns current belief state.
5. **SelfModel + CoherenceChecker** — persistent instances; `GET /self/health`, `GET /self/capabilities`; SelfModel bootstrapped with capabilities on startup; CoherenceChecker held as persistent Arc.
6. **WorldModel persistence** — `worldmodel.json` in DATA_DIR; saves on clean shutdown + every 5 minutes via background Tokio task.

---

## Architecture Decisions

### D1: AppState<B> struct (not flat args, not Axum Extension)

**Decision:** Introduce `AppState<B>` struct that bundles all stores and intelligence components. `run_with_both_stores` becomes `run_with_state(addr, state: AppState<B>)`.

**Why not flat args:** Signature would grow from 2 to 6 args, any future addition requires a signature change at every call site.

**Why not Axum Extension:** All 30+ existing handlers use closure-capture pattern (`let store = memory_store.clone()`). Migrating to Extension extractors means rewriting every handler. The generic `B: MemoryBackend` creates additional friction with Extension. Axum 0.6 (current version) Extension is less type-safe than 0.7 State. Cost >> benefit.

**Why AppState:** Only the ~8-10 handlers that use new state change. The other 22+ handlers change only at the closure capture site (replace `memory_store.clone()` with `state.memory_store.clone()`), and only if they need new state.

```rust
#[derive(Clone)]
pub struct AppState<B: MemoryBackend + Send + Sync + 'static> {
    pub memory_store:   Arc<Mutex<MemoryStore<B>>>,
    pub symbolic_store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
    pub world_model:    Arc<RwLock<WorldModelEnhanced>>,   // RwLock: read-heavy
    pub aureus:         Arc<Mutex<AureusBridge>>,          // Mutex: &mut self in reflexion_loop
    pub self_model:     Arc<SelfModel>,                    // already Arc<RwLock> internally
    pub coherence:      Arc<CoherenceChecker>,             // already Arc<RwLock> internally
}
```

### D2: WorldModel persistence — JSON snapshot + periodic flush

**Decision:** `worldmodel.json` in `DATA_DIR`. Explicit `save(path)`/`load(path)` methods on `WorldModelEnhanced`. Saved on clean shutdown (SIGTERM) + every 5 minutes via a background `tokio::spawn` loop.

**Why not in-memory only:** Every deploy/restart loses all learned Dirichlet counts. After 7 days of 100 memory adds/day = 700 observations. Cold-start amnesia on every deploy is unacceptable in production.

**Why not MemoryStore-backed (Option 3):** Pollutes `memory.jsonl` with system records. `GET /memory/query` would return world model entries unless filtered everywhere. GDPR forget could delete world model records. No `upsert` in MemoryStore — needs 2 ops per transition update. 6-10x slower hot path.

**Why periodic flush:** SIGTERM-only save loses data on crash (OOM, `kill -9`, Fly.io timeout). Periodic flush means max 5 minutes of learning lost on any crash.

**The tuple-key problem:** `HashMap<(String,String,String), usize>` — tuple keys not serializable by serde to JSON (JSON requires string keys). Solution: key encoding with `\x1F` (ASCII Unit Separator, safe in any UTF-8 string, never appears in normal text):

```rust
fn encode_transition_key(s: &str, a: &str, ns: &str) -> String {
    format!("{}\x1F{}\x1F{}", s, a, ns)
}
fn decode_transition_key(k: &str) -> Option<(&str, &str, &str)> {
    let mut parts = k.splitn(3, '\x1F');
    Some((parts.next()?, parts.next()?, parts.next()?))
}
```

**Persist only:** `TransitionModel.counts` + `TransitionModel.totals` + `CausalGraph` edges. NOT entity Kalman states (recovered from live MemoryStore reads if needed). NOT AureusBridge hypothesis graph (transient reasoning state, acceptable to lose). NOT SelfModel (capability registry rebuilt at startup, health/perf resets are fine).

**worldmodel.json schema:**
```json
{
  "version": 1,
  "transition_counts": { "alice\x1Fdecided\x1Fuse_postgres": 42 },
  "transition_totals": { "alice\x1Fdecided": 45 },
  "smoothing": 1.0,
  "causal_edges": [{ "from": "alice", "to": "postgres", "strength": 0.9 }]
}
```

### D3: Memory → WorldModel auto-feed (non-blocking, best-effort)

After successful `MemoryStore.add()` in `handle_add_memory` and `handle_ingest`, call `world_model.write().observe_transition(actor, action, target)`. If this fails (lock contention, etc.), log and continue — never block the memory write.

For `handle_ingest`, additionally: if `record_type == "Symbolic"` and `priority == "pinned"`, also call `wm.add_causal_edge(actor, target)` to register strong causal claims.

### D4: AureusBridge reflect endpoint — auto-context from MemoryStore

`POST /memory/reflect { query: "..." }`:
1. `search_semantic(None, query, 10, false)` → top-K MemoryRecords
2. Format as context string: `"Memory context:\n- [action] target\n..."`
3. `aureus.lock().reflexion_loop(context, &mut memory_store.lock())`
4. Returns `{ hypothesis: "...", confidence: f32, evidence: [...] }`

### D5: SelfModel bootstrap on server startup

Register capabilities in `bin/webserver.rs` after constructing `AppState`:
```rust
let ops = ["add_memory", "search_memory", "query_memory", "ingest", "bulk_add", "forget", "reflect"];
for op in ops {
    state.self_model.register_capability(CapabilityDescriptor {
        name: op.to_string(),
        description: format!("HipCortex {} operation", op),
        required_cpu_percent: 5.0,
        required_memory_mb: 50.0,
        limitations: vec![],
    }).ok();
}
```

SelfModel gating is **advisory only** (log warning, don't reject) — blocking requests based on untuned resource predictions would break the API. This will be made hard-gate after the SelfModel has enough observations to calibrate.

---

## New REST Endpoints

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| POST | `/worldmodel/observe` | `handle_wm_observe` | Feed `{from, action, to}` → Dirichlet update |
| GET | `/worldmodel/predict?state=&action=` | `handle_wm_predict` | Get P(s'\|s,a) distribution + entropy |
| GET | `/worldmodel/entities` | `handle_wm_entities` | List Kalman-tracked entities |
| POST | `/worldmodel/entity` | `handle_wm_register_entity` | Register entity with initial state |
| GET | `/worldmodel/causal` | `handle_wm_causal` | Dump causal DAG (nodes + edges) |
| POST | `/memory/reflect` | `handle_memory_reflect` | Reflexion loop over memory context |
| GET | `/memory/hypotheses` | `handle_memory_hypotheses` | Current AureusBridge hypothesis graph |
| GET | `/self/health` | `handle_self_health` | SelfModel health score + module breakdown |
| GET | `/self/capabilities` | `handle_self_capabilities` | Registered capability descriptors |

---

## Files Changed

| File | Change |
|------|--------|
| `src/web_server.rs` | Add `AppState<B>`, rename `run_with_both_stores` → `run_with_state`, add 9 new handlers, update add_memory+ingest to auto-feed WorldModel |
| `src/bin/webserver.rs` | Construct `AppState`, bootstrap SelfModel capabilities, add `worldmodel.json` load/save in shutdown hook, add periodic flush task |
| `src/modules/world_model_enhanced/mod.rs` | Add `save(path)`, `load(path)`, `transition_count()` public methods |
| `src/modules/world_model_enhanced/transition.rs` | Add `save(path)`, `load(path)`, `smoothing()` getter |
| `src/modules/world_model_enhanced/causal.rs` | Add `save_edges(path)`, `load_edges(path)`, `all_edges()` public method |
| `src/modules/aureus_bridge.rs` | Add `reflect_on_memory<B>(&mut self, query, store) -> ReflexionHypothesis` method |
| `tests/integration/intelligence_wiring_sit.rs` | All new SIT tests |
| `tests/integration/mod.rs` | Register new test file |

---

## Out of Scope (this change)

- Hard-gate request rejection via SelfModel (log only, not block)
- Entity Kalman state persistence
- AureusBridge hypothesis graph persistence
- Memory provenance graph (`derived_from` field)
- Full namespace/multi-tenancy
- Semantic dedup
- Working memory auto-promotion

---

## Risk Register

| Risk | Mitigation |
|------|-----------|
| `run_with_both_stores` rename breaks callers | Only 1 external call site: `bin/webserver.rs`. `run_with_memory` wrapper becomes internal. Update both. |
| WorldModel RwLock deadlock if write held during search | World model write in handle_add is non-blocking: `if let Ok(mut wm) = state.world_model.try_write() { ... }` |
| `worldmodel.json` corrupt on partial write | Write to `.tmp` then `rename()` — atomic on POSIX, near-atomic on Windows NTFS |
| SelfModel capability bootstrap fails | All `register_capability` calls wrapped in `.ok()` — non-fatal |
| AureusBridge Mutex held during long LLM call (none configured) | No LLM configured in free tier → reflexion_loop is fast (no LLM client) |
