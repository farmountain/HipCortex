# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

HipCortex — modular Rust AI memory engine combining temporal, procedural, and symbolic memory with a metacognitive intelligence layer (self-model, predictive world-model, coherence checker). Library + CLI + optional REST/gRPC server + optional Tauri GUI. Crate name: `hipcortex` (Apache-2.0, edition 2021, MSRV ~1.70).

## Build / Test / Run Commands

The default feature set (`petgraph_backend`) needs no external deps. Most other backends require system libraries (PostgreSQL, SQLite, RocksDB, Neo4j) and will fail without them — explicitly opt out when iterating locally.

Minimal dev loop (always works on a clean machine):

```sh
cargo build --no-default-features --features "petgraph_backend"
cargo test  --no-default-features --features "petgraph_backend" --lib
cargo run   --example quickstart --no-default-features --features "petgraph_backend"
```

Running a single test:

```sh
cargo test --no-default-features --features "petgraph_backend" --lib <test_name>
# Integration test files live in tests/integration/, registered through tests/integration_suite.rs
cargo test --no-default-features --features "petgraph_backend" --test integration_suite <test_name>
# Property tests are aggregated in tests/property_suite.rs
cargo test --no-default-features --features "petgraph_backend" --test property_suite
# Unit tests aggregated in tests/unit_suite.rs
```

Benchmarks (Criterion, `harness=false`):

```sh
cargo bench --no-default-features --features "petgraph_backend"
# Named benches: temporal_indexer_bench, symbolic_store_bench
```

Web/gRPC/GUI variants:

```sh
cargo build --no-default-features --features "web-server,petgraph_backend"
cargo run   --example mcp_server --no-default-features --features "web-server,grpc-server,petgraph_backend"
cargo build --no-default-features --features "gui,petgraph_backend"          # needs Node + Tauri toolchain
cargo build --no-default-features --features "plugin,petgraph_backend"       # WASM plugin host
```

`cargo build --all-features` requires every external library installed and is **not** the default expectation — prefer additive feature lists.

Lint/format:

```sh
cargo fmt --all
cargo clippy --no-default-features --features "petgraph_backend" --all-targets -- -D warnings
```

CLI binary (`hipcortex` → `src/bin/cli.rs` → `memory_cli::run`):

```sh
cargo run -- add --actor a --action did --target t
cargo run -- query --actor a
cargo run -- llm-generate "Hello"
cargo run -- worldmodel-predict '{"state":"robot","action":"move"}'
cargo run -- safety-audit
```

Web binary: `src/bin/webserver.rs` (requires `web-server` feature).

## Architecture

### Two-layer design

1. **Storage / memory layer** (the original engine): perception → temporal → symbolic + procedural FSM → reflexion → integration.
2. **Intelligence layer** (added under `openspec/changes/intelligence-foundation/`): `self_model`, `world_model_enhanced`, `coherence` — metacognition, prediction, and cross-module consistency. Wired in opt-in alongside the storage layer; backward compatible.

### Crate layout (the parts you can't infer from `ls`)

`src/lib.rs` is the source of truth for what is a public module and **how files map to module paths**. Many modules live physically under `src/modules/` but are re-exported at the crate root via `#[path = "modules/..."]` (e.g. `aureus_bridge`, `procedural_cache`, `symbolic_store`, `temporal_indexer`, `world_model`, `coherence`, `self_model`, `world_model_enhanced`). Edit `lib.rs` whenever you add or move a module — adding a file under `src/modules/` alone does **not** register it.

Backend modules live in `src/backends/` and are gated behind feature flags inside the `pub mod backends { ... }` block in `lib.rs`. Default backend is `petgraph` (in-memory). Anything Postgres/Neo4j/RocksDB/Diesel-related is feature-gated and not compiled in the minimal build.

LLM connectors are aggregated through `src/llm_clients/mod.rs` (OpenAI, Claude, Mistral, Falcon, DeepSeek, Llama, Ollama, local, mock). New connectors should implement the connector trait there and be exported from that `mod.rs`.

### Memory model

`MemoryRecord` (`src/memory_record.rs`) with `MemoryType` (Temporal / Symbolic / Procedural / Reflexion) is the canonical record. `MemoryStore` (`src/memory_store.rs`) is the persistence facade — it supports plain JSONL, AES-GCM encrypted JSONL (`new_encrypted` / `new_encrypted_envelope`), an append-only Merkle-chained `audit.log`, a write-ahead log for crash recovery, snapshot rollback, batched writes, and (under `async-store`) a buffered async backend. Every record carries a SHA-256 integrity hash that is verified on load; never bypass these paths when adding a new persistence route.

Pluggable persistence is via the `MemoryBackend` trait. RocksDB (`src/rocksdb_backend.rs`) is the reference non-default backend.

### Subsystem responsibilities

- **PerceptionAdapter** (`src/modules/perception_adapter.rs`) — multimodal input (text, embeddings, agent messages, vision via `VisionEncoder`), rate-limited, PCA decorrelation, schema validation. Optional GPU encoder via `wgpu` falls back to CPU when no device is present.
- **TemporalIndexer** (`src/modules/temporal_indexer.rs`) — segmented ring buffer, per-trace decay, LRU pruning, Markov-ordered.
- **SymbolicStore** (`src/modules/symbolic_store.rs`) — graph store via the `GraphDatabase` trait + LRU label cache. Backends: `petgraph` (default), `sled`, optional `neo4j`/`postgres`.
- **ProceduralCache** (`src/modules/procedural_cache.rs`) — FSM-driven traces. `advance_batch` for bulk transitions, checkpoint save/load. Backed by `TemporalFSMBackend`.
- **AureusBridge** (`src/modules/aureus_bridge.rs`) — reflexion / chain-of-thought hook; consumes from temporal+symbolic, optionally calls LLM connectors and the world model.
- **IntegrationLayer** (`src/modules/integration_layer.rs`) — bridges OpenManus and MCP payloads to memory ops; API key + OAuth2 bearer validation; routes to LLM connectors. Combined REST + gRPC server lives in `src/mcp_server.rs` (gated on both `web-server` and `grpc-server`).
- **SafetyGuardrail** (`src/safety_guardrail.rs`) — every mutation across graph store, FSM backend, and LLM connectors must call `check_precondition` first. Violations are logged and can trigger rollback. View with `cargo run -- safety-audit`.
- **Intelligence layer**:
  - `self_model/` — capability registry, resource monitor (linear regression), perf tracker (EWMA + Bayesian), health aggregator (weighted geometric mean), decision engine (expected utility).
  - `world_model_enhanced/` — Dirichlet-Multinomial transitions, Kalman entity tracking, causal graphs with do-calculus, uncertainty quantification.
  - `coherence/` — `ConsistencyChecker` + `ConflictResolver` (consensus / recency / confidence) + `SystemInvariants` (acyclicity, decay monotonicity, conservation). Holds shared state through `Arc<RwLock<...>>` — respect locking order to avoid deadlocks.

### Optional surfaces

- REST: `src/web_server.rs` (Axum 0.6, feature `web-server`).
- gRPC: `src/grpc_server.rs` (Tonic, feature `grpc-server`); proto in `proto/memory.proto`, generated by `build.rs` only when the feature is enabled.
- Combined MCP: `src/mcp_server.rs` (needs both `web-server` + `grpc-server`).
- GUI: Tauri app under `ui/` + `src/gui.rs` (feature `gui`, `tauri.conf.json`).
- WASM plugins: `src/plugin_host.rs` (feature `plugin`).
- VS Code extension: `vscode-extension/` (TypeScript, packaged `.vsix`).

## Testing layout

Tests are split into three top-level test binaries — adding a file in the subdirectory is not enough; register it in the matching `mod.rs` / `*_suite.rs`:

- `tests/unit_suite.rs` ← files in `tests/unit/`
- `tests/integration_suite.rs` ← files in `tests/integration/` (SIT/UAT scenarios end with `_sit.rs` / `_uat.rs`)
- `tests/property_suite.rs` ← `tests/property/` (proptest)

Property tests assert graph connectivity (`connectivity.rs`), FSM reachability (`fsm_reachability.rs`), and intelligence-layer invariants (`self_model_props.rs`, `world_model_props.rs`). Run them after touching anything in `temporal_indexer`, `symbolic_store`, `procedural_cache`, or the intelligence layer.

Fixtures live in `fixtures/`; JSON Schemas for record types in `schemas/`.

## Conventions specific to this repo

- **Module wiring rule**: register every new module in `src/lib.rs` (with `#[path]` if it lives under `src/modules/`). The build will silently ignore unregistered files.
- **Feature gating rule**: external-database / async / web / gpu / plugin code must be `#[cfg(feature = "...")]`-gated so the minimal `petgraph_backend` build keeps compiling on a fresh machine.
- **Safety rule**: never mutate the graph store, FSM backend, or call an LLM connector without going through `SafetyGuardrail::check_precondition` first. The audit log is part of the contract.
- **Persistence rule**: don't write `MemoryRecord`s by hand — go through `MemoryStore` so the SHA-256 integrity hash, AES-GCM envelope, WAL, and `audit.log` Merkle chain stay in sync.
- **CoT comment convention**: each module begins with a short "Chain-of-Thought" block summarizing its reasoning flow (see `src/modules/coherence/mod.rs` for the canonical shape). Preserve this when refactoring.
- **OpenSpec workflow**: large changes have a spec under `openspec/changes/<change-name>/` with `proposal.md`, `design.md`, `tasks.md`, and `specs/`. Read the proposal before modifying anything in the touched capability.

## Reference docs in-repo

- `README.md` — feature catalog and crate-user-facing overview.
- `DEVELOPMENT.md` — feature-flag matrix and per-OS setup.
- `Hipcortex_Env_Setup_Guide.md` — Windows environment setup with database backends.
- `docs/architecture.md`, `docs/memory_design.md`, `docs/data_model.md`, `docs/integration.md`, `docs/usage.md`, `docs/roadmap.md`.
- `openspec/changes/intelligence-foundation/` — design + tasks for the self-model / world-model / coherence layer.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **HipCortex** (5665 symbols, 11432 relationships, 259 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/HipCortex/context` | Codebase overview, check index freshness |
| `gitnexus://repo/HipCortex/clusters` | All functional areas |
| `gitnexus://repo/HipCortex/processes` | All execution flows |
| `gitnexus://repo/HipCortex/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
