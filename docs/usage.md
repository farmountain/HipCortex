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

## 7. Import as a Library

Add this project as a dependency in your own Rust project (`Cargo.toml`):

```toml
hipcortex = { path = "../hipcortex" }
```

Import modules in your code:

```rust
use hipcortex::temporal_indexer::TemporalIndexer;
```

## 8. Get Help

* For architecture and design, see `docs/architecture.md`.
* For integration/API details, see `docs/integration.md`.
* To contribute, see `docs/contributing.md`.
* For roadmap and planned modules, see `docs/roadmap.md`.
