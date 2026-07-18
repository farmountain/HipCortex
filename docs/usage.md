# HipCortex Usage Guide

Welcome to the HipCortex AI Memory Engine! This guide explains how to build, run,
test, and extend the system for agentic, edge, or AI research use.

**Channel support matrix (honest statuses):** [docs/channels.md](channels.md) · `hipcortex channels`.  
Statuses: `native` | `mcp` | `framework` | `guide` | `claimed` | `none`. Hermes / OpenClaw / Grok / Antigravity-specific paths are **claimed**, not first-class wizard targets.

---

## 1. Build & Run

Minimal path (works on a clean machine — no external DB libs):

```sh
cargo build --no-default-features --features "petgraph_backend"
cargo run  --example quickstart --no-default-features --features "petgraph_backend"
# HTTP server (optional)
cargo run --bin webserver --no-default-features --features "web-server,petgraph_backend"
```

* Quickstart demo: `examples/quickstart.rs`. CLI binary: `cargo run --` (see root `CLAUDE.md`).

## 2. Run All Tests

```sh
cargo test --no-default-features --features "petgraph_backend" --lib
cargo test --no-default-features --features "petgraph_backend" --test unit_suite
cargo test --no-default-features --features "petgraph_backend" --test integration_suite
```

* Suites: `tests/unit_suite.rs`, `tests/integration_suite.rs`, `tests/property_suite.rs`.
* E2E user harness (Python): `tests/e2e_user_harness/` — plan in `HipCortex_E2E_User_Testing_Plan.md`.

## 3. Run Benchmarks

```sh
cargo bench --no-default-features --features "petgraph_backend"
python benchmarks/token_reduction_benchmark.py
```

* Criterion benches under `/benches/`. Token matrix: Headroom / Caveman / Proactive substrate.

## 4. Example Usage

For a minimal working example of temporal, procedural, symbolic, and multimodal memory in action:

```sh
cargo run
```

### Encode an image

Use the `VisionEncoder` to convert an image into a simple RGB embedding:

```rust
use hipcortex::vision_encoder::VisionEncoder;
let embedding = VisionEncoder::encode_path("image.png")?;
// Compress to 4 dimensions
let compressed = hipcortex::semantic_compression::compress_embedding(&embedding, 4);
```

The output will show insertions, FSM transitions, symbolic graph operations, and perception adapter traces.

## Agent Integration & Automatic World Model Maintenance

When agents (Claude Code harness, MCP clients, etc.) send messages using `Modality::AgentMessage` (the primary path enabled by the proactive SKILL):

- Safety guardrail + self-model health/rate limits are applied first.
- `PerceptionSession::adapt` (and the underlying `PerceptionAdapter`) normalizes the message and **automatically**:
  - Updates `WorldModelEnhanced` with entity observations (the embedding as measured properties for "perception_input").
  - Runs coherence validation.
- In the auto path inside `IntegrationLayer::handle_mcp`, the engine also calls `PerceptionSession::record_perceived_action(text)` (delegating to `WorldModelEnhanced::record_perceived_action`). This records the agent's message text as a state transition (action) in the Dirichlet-Multinomial world model.

**Outcome**: Basic "latest state and world model" maintenance happens automatically from the agent stream. The substrate's predictive transitions and entity tracking stay current **without the LLM or user having to explicitly call** `ingest`, `add_memory("...decision...")`, or "update world model".

High-fidelity work (rich `Symbolic` records, `Aureus` reflexion / `HypothesesGraph`, explicit causal edges, or high-priority/pinned decisions) continues to use the documented explicit surfaces (`/memory/ingest`, `POST /memory/reflect`, `add_memory` with priority/tags) as mandated by the proactive harness SKILL ("MUST ... after any decision").

See the implementations and comments in:
- `src/modules/integration_layer.rs` (AgentMessage branch)
- `src/modules/perception_adapter.rs` (`record_perceived_action` + adapt hooks)
- `src/modules/world_model_enhanced/mod.rs`
- `openspec/changes/agent-substrate-autonomy/specs/engine-agent-defaults/spec.md`
- Proactive `SKILL.md` in the Python package (`sdk/python/hipcortex/install/SKILL.md`)
- Live observation surface: `GET /memory/live_beliefs`

### Claude Agent Harness & Substrate-First Loop

When configured in `proactive` mode (via `hipcortex install --mode proactive`), Claude Code or other agents adopt a **substrate-first** cognitive loop:

1. **Perceive**: Agent reads the user query.
2. **Substrate Query (MUST)**: Before any frontier reasoning tokens, call `GET /memory/live_beliefs` (aggregates symbolic graph, current hypotheses, world predictions, self-health) to load persistent context.
3. **Reflect (CoT / Counterfactual)**: For complex decisions or high uncertainty, call `POST /memory/reflect` with a target query (e.g. `{"query": "database choice options"}`). Server runs `AureusBridge::reflect_on_memory` to sample hypotheses and return posteriors.
4. **Decide & Act**: Gate actions with `POST /decide/can-execute`; write decisions via `/memory/ingest` (or `add_memory` for full field control).

Install (writes SKILL + harness registration):

```sh
hipcortex install --mode proactive --actor my_project_agent
# optional: point at managed tier
hipcortex install --mode proactive --url https://hipcortex.fly.dev
```

### Project config (`.hipcortex/config.toml`)

`hipcortex install` writes a per-project config under the current working directory:

```toml
# .hipcortex/config.toml
url = "http://127.0.0.1:3030"
actor = "my_project_agent"
mode = "conservative"          # or proactive
channels = ["claude-code", "cursor"]
server_version_policy = "major_minor"
```

Optional user defaults: `~/.hipcortex/user.toml` (same keys).

**URL resolution** (client, `hipcortex doctor`, adapters with no explicit URL):

1. `HIPCORTEX_URL` env  
2. project `.hipcortex/config.toml`  
3. user `~/.hipcortex/user.toml`  
4. default `http://127.0.0.1:3030`

**Actor resolution**: `HIPCORTEX_ACTOR` > project > user. Optional `[aliases]` maps channel-local names to a canonical actor.

```python
from hipcortex import HipCortexClient
from hipcortex.config import load_settings, get_default_actor

client = HipCortexClient()          # uses resolved url
settings = load_settings()          # full merge for current cwd
actor = get_default_actor()         # or settings.actor
```

```sh
hipcortex doctor                    # health against resolved url
hipcortex doctor --url http://…     # explicit override
```

**Using `/memory/reflect` manually:**

```sh
curl -X POST http://localhost:3030/memory/reflect \
     -H "Content-Type: application/json" \
     -d '{"query": "Postgres migration decisions"}'
```

Returns:

```json
{
  "success": true,
  "loops_run": 3,
  "llm_used": true,
  "hypothesis": {
    "id": "hyp_uuid_...",
    "text": "Using Postgres handles multi-user concurrency safely",
    "confidence": 0.89,
    "evidence": ["locks configured", "concurrency tests passed"]
  }
}
```

Related surfaces:

| Surface | Role |
|---------|------|
| `GET /memory/live_beliefs` | Pre-merged observations (facts + hyp + world + intel) |
| `GET /memory/search` / `search-flat` | Top-k recall for Headroom/Caveman budgeting |
| `POST /memory/ingest` | Zero-config write after decisions |
| `POST /decide/can-execute` | Self-model gate before side effects |

### Loop / Ω (Omega) Substrate

Topological memory substrate and full Ω loop (snapshot → simulate → attribute → mutate → gate) exposed via MCP/Integration auto feeds and `PerceptionSession` (with_topo). Use `hipcortex::topological_memory::CausalTopoGraph` and `hipcortex::loop_engine::LoopEngine` directly or via harness for substrate-first ops. See `docs/superpowers/plans/2026-06-20-harness-and-omega-loop-engineering.md` and `src/{mcp_server,modules/{integration_layer,perception_adapter,loop_engine}}`.

### Run a WASM plugin

Compile with the `plugin` feature to enable the `PluginHost` and execute WebAssembly extensions:

```sh
cargo run --example plugin_host --features plugin
```

This runs `examples/plugin_host.rs` which loads a tiny WAT module and prints the returned value.

### Effort & Confidence Example

Measure reasoning effort and decay confidence dynamically:

```rust
use hipcortex::effort::{EffortEvaluator, ConfidenceRegulator};

let mut eval = EffortEvaluator::new();
eval.record(5); // weighted cost

let mut conf = ConfidenceRegulator::new();
conf.decay_exponential(0.2);

println!("Collapse: {}", eval.collapse_score(conf.confidence()));
```

## 5. VS Code Setup

* Open project root in VS Code.
* Install recommended extensions (see `.vscode/extensions.json`).
* Use integrated test and bench tasks via `Ctrl+Shift+B`.

## 6. Add Your Own Logic

* Extend in `src/` with new modules, or modify/expand core modules.
* Write new tests in `/tests/` for TDD (Test-Driven Development).
* See `README.md` and `docs/architecture.md` for module roles.

### Segmented Ring Buffer Example

Efficiently batch and flush streaming data:

```rust
use hipcortex::segmented_buffer::SegmentedRingBuffer;

let mut buf = SegmentedRingBuffer::new(8, 4);
buf.push_back(1);
buf.push_back(2);
buf.push_back(3);
let batch = buf.flush_front_segment().unwrap();
assert_eq!(batch, vec![1, 2, 3]);
```

## 7. Graph Backend Options

`SymbolicStore` works with multiple graph backends. By default the in-memory
backend uses the `petgraph` crate. Enable others via Cargo features:

```toml
[features]
neo4j_backend = ["neo4rs", "tokio"]
postgres_backend = ["tokio-postgres", "tokio"]
```

Switch backends in code:

```rust
use hipcortex::symbolic_store::{SymbolicStore, InMemoryGraph};
let store = SymbolicStore::<InMemoryGraph>::new();
```

For Neo4j, you can execute Cypher queries:

```cypher
MATCH (a)-[:REL]->(b) RETURN a,b
```

Call `assert_graph_invariants()` to verify edges reference existing nodes.

## 8. FSM Backend Options

`ProceduralCache` manages finite state machines. The default backend uses a
`petgraph` based engine but others can be swapped in via the `FSMBackend` trait.
Enable the experimental Temporal backend by compiling with feature flags:

```toml
[features]
rustfsm_backend = []
temporal_backend = ["temporalio"]
```

Create a cache with the default backend:

```rust
use hipcortex::procedural_cache::ProceduralCache;
let mut cache = ProceduralCache::new();
```

Custom backends can be plugged in with `from_backend`.

## 9. Import as a Library

Add this project as a dependency in your own Rust project (`Cargo.toml`):

```toml
hipcortex = { path = "../hipcortex" }
```

Import modules in your code:

```rust
use hipcortex::temporal_indexer::TemporalIndexer;
```

## 10. Get Help

* For architecture and design, see `docs/architecture.md`.
* For integration/API details, see `docs/integration.md`.
* To contribute, see `docs/contributing.md`.
* For the roadmap and additional modules, see `docs/roadmap.md`.

## 11. Reflexion Hypotheses Graph

`AureusBridge` now tracks reasoning as a Bayesian hypothesis graph. Each reflexion step parses the LLM output into a `ReflexionHypothesis`:

```json
{
  "text": "Sky appears blue",
  "confidence": 0.72,
  "evidence": ["sunlight scatters"]
}
```

Edges between nodes mark support or refutation. Posterior confidence is
computed via:

```
P(H|E) = P(E|H)P(H) / [P(E|H)P(H) + P(E|¬H)P(¬H)]
```

Nodes with posterior below the `prune_threshold` are automatically removed.
Use `run_monte_carlo` to sample multiple hypotheses and select the highest mean
confidence.

## 12. HypothesisManager Example

The `HypothesisManager` manages a tree of hypotheses with Bayesian probabilities. Nodes with low probability can be pruned automatically.

```rust
use hipcortex::hypothesis_manager::HypothesisManager;

let mut mgr = HypothesisManager::new();
let root = mgr.add_root("start", 0.6);
mgr.add_child(root, "option_a", 0.7);
mgr.add_child(root, "option_b", 0.4);

let best = mgr.best_path();
for h in best { println!("{} : {}", h.state, h.probability); }

mgr.export_dot("hypotheses.dot");
```

