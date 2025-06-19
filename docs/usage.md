# HipCortex Usage Guide

Welcome to the HipCortex AI Memory Engine! This guide explains how to build, run,
test, and extend the system for agentic, edge, or AI research use.

---

## 1. Build & Run

```sh
cargo build
cargo run
```

* `main.rs` demonstrates all major features and acts as a quickstart demo.

## 2. Run All Tests

```sh
cargo test
```

* Unit, integration, and property-based tests are in `/tests/integration_tests.rs`.

## 3. Run Benchmarks

```sh
cargo bench
```

* Criterion benchmarks are in `/benches/criterion_bench.rs`.

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

### Run a WASM plugin

Compile with the `plugin` feature to enable the `PluginHost` and execute WebAssembly extensions:

```sh
cargo run --example plugin_host --features plugin
```

This runs `examples/plugin_host.rs` which loads a tiny WAT module and prints the returned value.

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
neo4j_backend = ["reqwest"]
postgres_backend = ["sqlx"]
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
