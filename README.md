# HipCortex

A modular, agentic AI memory engine for reasoning, edge, and multi-agent workflows.  
Built in Rust, integrating temporal, procedural, and symbolic memory with agentic and multimodal APIs.

## 📘 Business Context
HipCortex enables persistent memory and reasoning for bots and edge automation. It can operate as a lightweight library, a REST microservice or a desktop app. See [docs/business_context.md](docs/business_context.md) for details.

---

## ✨ Features

HipCortex is built from modular building blocks so you can mix and match memory
and reasoning components.

- **Temporal Indexer:** Short/long-term buffer with decay and LRU pruning.
- **Procedural FSM Cache:** Regenerative memory driven by finite state logic for
  workflows and actions.
- **Symbolic Store:** Graph-based concept store with semantic key/value pairs.
- **Perception Adapter:** Multimodal input handler (text, embeddings, agent
  messages, vision). Includes a simple VisionEncoder for image embeddings.
- **Aureus Bridge:** Reflexion and reasoning hook for chain-of-thought engines.
- **Integration Layer:** REST/gRPC and protocol stubs (OpenManus, MCP).
- **Fully Test-Driven:** Extensive unit tests and Criterion benchmarks.
- **Optional Web Server:** compile with `--features web-server` for an Axum REST API.
- **Optional GUI:** compile with `--features gui` to launch a Tauri desktop client.

---

## 🏗️ Project Structure

| Path/Module                    | Purpose                                 |
|------------------------------- |-----------------------------------------|
| `src/lib.rs`                   | Main library module, re-exports others  |
| `src/main.rs`                  | CLI/demo entry (optional)               |
| `src/temporal_indexer.rs`      | STM/LTM temporal buffer                 |
| `src/procedural_cache.rs`      | FSM-based procedural cache              |
| `src/symbolic_store.rs`        | Symbolic graph & key-value memory       |
| `src/perception_adapter.rs`    | Multimodal input                        |
| `src/integration_layer.rs`     | Agentic/REST/gRPC stubs                 |
| `src/aureus_bridge.rs`         | Reflexion/reasoning loop                |
| `src/vision_encoder.rs`        | Simple image to embedding converter     |
| `tests/`                       | Integration and property tests          |
| `benches/`                     | Criterion benchmarks                    |
| `examples/`                    | Minimal runnable example                |
| `docs/`                        | Architecture, usage, integration, roadmap|
| `.github/`                     | PR/Issue templates for collaboration    |
| `.vscode/`                     | VS Code developer environment           |

---

## 🚀 Quickstart

```sh
git clone <your-repo>
cd hipcortex
cargo build
cargo test        # Run all tests
cargo run         # Run the CLI demo
cargo bench       # Run benchmarks
```

See examples/quickstart.rs for a minimal programmatic usage demo.
Detailed data model and extended architecture diagrams are available in [docs/data_model.md](docs/data_model.md) and [docs/architecture.md](docs/architecture.md).

## 🛠️ Use Cases

HipCortex can serve a variety of scenarios:

- **Agent Memory Engine:** manage conversation context and reasoning traces for single or multi-agent systems.
- **Edge Workflow Execution:** run on resource-constrained hardware thanks to Rust's performance and small footprint.
- **Search & Retrieval:** build knowledge graphs or retrieval-augmented generation pipelines using the SymbolicStore and TemporalIndexer.
- **Real-Time Interfaces:** expose REST/gRPC APIs and upcoming CLI/web dashboards via the IntegrationLayer.
- **Knowledge Export:** connect to Notion, PDF, or other backends for long-term persistence (planned).
## 🧪 Test & Automation
- **Run all tests:**  
  `cargo test`

- **Run benchmarks:**  
  `cargo bench`

- **Test suite:**  
  - Unit and integration tests: `/tests/integration_tests.rs`
  - Property-based/fuzz tests: integrated using [proptest](https://docs.rs/proptest)
  - Add new test files to `/tests/` as needed

- **CI/CD Ready:**  
  You can use GitHub Actions or any CI provider—add `.github/workflows/ci.yml` (see Rust starter templates) to run on every PR or push.

- **VS Code Integration:**  
  Open with VS Code. Test & bench tasks are already available via `.vscode/tasks.json` (Ctrl+Shift+B).

- **Best Practices:**  
  - Always write failing tests first (TDD)
  - Ensure all modules have coverage before merge
  - Add benchmarks for any new algorithm or data structure



## Summary Table 
| Doc                  | Purpose                                               |
| -------------------- | ----------------------------------------------------- |
| README.md            | Project overview, structure, TDD, quickstart, roadmap |
| src/lib.rs           | Library entry (export modules)                        |
| docs/architecture.md | System design, extensibility, diagram                 |
| docs/business_context.md | Business requirements and use cases |
| docs/data_model.md | MemoryRecord schema and API notes |
| docs/usage.md        | Build, test, bench, example, import                   |
| docs/integration.md  | Protocol/API plans, extension points                  |
| docs/roadmap.md      | Completed, active, planned modules                    |
| docs/contributing.md | Contribution guide, code/test policy                  |
| LICENSE              | MIT License                                           |
