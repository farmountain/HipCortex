# HipCortex

A modular, agentic AI memory engine for reasoning, edge, and multi-agent workflows.
Built in Rust, integrating temporal, procedural, and symbolic memory with agentic and multimodal APIs.

## � Quick Start

Get HipCortex running in minutes:

### Docker/Podman (Recommended)
```bash
# Clone and start with Docker Compose
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex
./quick-start.sh  # Linux/Mac
# or
.\quick-start.ps1  # Windows
```

### VS Code Extension
1. Download the latest `.vsix` file from `vscode-extension/`
2. Install via VS Code: `Ctrl+Shift+P` → "Extensions: Install from VSIX..."
3. Configure your API endpoint in settings

**📖 [Complete Quick Start Guide](QUICK_START_GUIDE.md)** | **📚 [Full Documentation](docs/)**

---

## �🔍 Problem
AI agents often lack persistent, contextual memory. Without a unified engine it
is difficult to reason over time or across modalities.

## 🎯 Mission
HipCortex aims to provide a memory engine that blends symbolic reasoning,
temporal relevance, procedural logic and perception in one modular package.
**HipCortex Memory: Math, Logic, Symbolic Guarantees** – reasoning steps follow proven models with logic checks. See [docs/memory_design.md](docs/memory_design.md).

## 🌐 Universal Positioning (AI Era)
HipCortex is designed to be the cognitive layer for every device—laptop, mobile, smart glass, autonomous OS—delivering local, private, explainable memory and reasoning.

### Key Features & Moats
- Local, encrypted, user-owned memory (privacy-first)
- Contextual, multimodal, explainable memory and reasoning
- Universal API, plugin SDK, device/OS integration
- Edge AI, federated learning, adaptive inference
- Resilience, performance, fault tolerance, offline-first
- Developer ecosystem, interoperability, open standards

### Strategic Gaps
- Federated learning/adaptive edge AI
- Mobile/AR/automotive SDKs
- Visual explainability for users
- Plugin marketplace/ecosystem
- Open schema/standards for context/memory
- Unified privacy/user control dashboard

### 🔬 Memory Design Extension
HipCortex now ships with a mathematically proven memory layer. Logical
predicates validate each write, symbolic graphs track context and property-based
tests confirm graph connectivity and FSM reachability. Every module begins with
a "Chain-of-Thought" comment summarizing its reasoning flow.

HipCortex treats each learned latent map as a serializable world model. Maps are
versioned over time and checked for alignment across procedural, temporal and
symbolic layers.


## 📘 Business Context
HipCortex enables persistent memory and reasoning for bots and edge automation. It can operate as a lightweight library, a REST microservice or a desktop app. See [docs/business_context.md](docs/business_context.md) for details.

---

## ✨ Features

HipCortex is built from modular building blocks so you can mix and match memory
and reasoning components.

- **AuditLog:** Hash-chained entries provide tamper-evident persistence for all
  memory writes.
- **Temporal Indexer:** Segmented ring buffer with per-trace decay factors and
  LRU pruning for short/long-term memory.
- **Procedural FSM Cache:** Regenerative memory driven by finite state logic for workflows and actions. Supports batch advancement of traces.
- **TemporalFSMBackend:** optional in-memory backend storing FSM traces with rollback and batch transitions.
- **Symbolic Store:** Graph-based concept store with semantic key/value pairs.
  Caches recent label lookups with an LRU cache. Backed by a pluggable
  `GraphDatabase` trait for in-memory or persistent graphs.
- **PetGraph Backend:** In-memory graph backend (default) - no external dependencies required.
- **Sled Backend:** Embedded key-value database - compile with `--features rocksdb-backend`.
- **Neo4j/Postgres Backends:** External database support - enable `neo4j_backend` or `postgres_backend` features to store graphs in Neo4j or Postgres (requires external libraries).
- **Perception Adapter:** Multimodal input handler (text, embeddings, agent
  messages, vision). Includes a simple VisionEncoder for image embeddings.
- **Semantic Compression:** Reduce embedding dimensionality with `semantic_compression::compress_embedding` for efficient storage.
- **Semantic Cache:** in-memory LRU store with embedding similarity lookups.
- **Aureus Bridge:** Reflexion and reasoning hook for chain-of-thought engines.
- **Integration Layer:** bridges OpenManus and MCP protocols to REST/gRPC endpoints.
- **MCP Server:** run both REST and gRPC endpoints to orchestrate symbolic context for multiple agents.
- **Math & Logic Guarantees:** memory operations validated with formal proofs and symbolic checks.
- **Fully Test-Driven:** Extensive unit tests and Criterion benchmarks.
- **Optional Web Server:** compile with `--features "web-server,petgraph_backend"` for an Axum REST API.
- **Optional GUI:** compile with `--features "gui,petgraph_backend"` to launch a Tauri desktop client.
- **Database Backends:** 
  - `--features "petgraph_backend"` for in-memory graphs (no external deps)
  - `--features "sqlite_backend"` for SQLite support (requires SQLite libraries)
  - `--features "postgres_backend"` for PostgreSQL support (requires PostgreSQL libraries)
  - `--features "neo4j_backend"` for Neo4j support (requires Neo4j server)
- **RocksDB Backend:** compile with `--features rocksdb-backend` and use `MemoryStore::new_rocksdb` for an embedded key-value database.
- **WASM Plugin Host:** compile with `--features "plugin,petgraph_backend"` to run custom WebAssembly extensions via `PluginHost`.
- **Effort Evaluator & Confidence Regulator:** monitor reasoning effort and confidence to avoid collapse.
- **Hypothesis Manager:** maintain multiple reasoning paths and a quantized state tree for backtracking.
- **Latent Map World Model:** learned latent maps are stored as versioned world models with safety guardrails.
- **Enhancement Advisor:** analyze module metrics and recommend improvements for human review.
- **Puzzle Benchmark Suite:** validates complex planning algorithms like Tower of Hanoi and 8-puzzle.
### Component Usage Examples

**GraphDatabase Backends (Neo4j/Postgres)**
```rust
use hipcortex::backends::{Neo4jBackend, PostgresGraphBackend};
// enable with --features neo4j_backend or postgres_backend
```

**TemporalFSMBackend**
```rust
use hipcortex::backends::temporal_backend::TemporalFSMBackend;
let mut backend = TemporalFSMBackend::new();
```

**IntegrationLayer Bridges**
```rust
use hipcortex::modules::integration_layer::IntegrationLayer;
let mut layer = IntegrationLayer::new();
layer.handle_openmanus("key", "{\"text\":\"hi\"}");
```

**SemanticCache**
```rust
use hipcortex::semantic_cache::SemanticCache;
let mut cache = SemanticCache::new(4);
cache.put_embedding("foo".into(), vec![0.1,0.2]);
```

**MonitoringService**
```sh
cargo run --example mcp_server --features web-server
# visit /metrics for JSON or open the GUI for HTML dashboard
```

**LLM connectors (Mistral/Falcon/DeepSeek)**
```sh
cargo run -- llm-generate --model mistral "Hello"
```

## Safety & Guardrail

HipCortex enforces runtime policies through the `SafetyGuardrail` module.
Operations across the graph store, FSM backend and LLM connectors call
`check_precondition` before mutating state. Violations are logged and can
trigger rollbacks. Use the CLI below to view recent audit snapshots:

```sh
cargo run -- safety-audit
```

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
| `src/mcp_server.rs`            | Combined REST + gRPC MCP server         |
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

### Minimal Setup (Recommended)
For quick setup without external database dependencies:
```sh
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex
cargo build --no-default-features --features "petgraph_backend"
cargo run --example quickstart --no-default-features --features "petgraph_backend"
cargo test --no-default-features --features "petgraph_backend" --lib
```

### Full Setup
For complete functionality with all features:
```sh
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex
cargo build --all-features  # Requires external database libraries
cargo test                  # Run all tests
cargo run                   # Run the CLI demo
cargo bench                 # Run benchmarks
```

### Feature Combinations
```sh
# Web server with REST API
cargo build --features "web-server,petgraph_backend"

# GUI application
cargo build --features "gui,petgraph_backend"

# With database backends (requires external libraries)
cargo build --features "petgraph_backend,sqlite_backend,postgres_backend"
```

**Note**: For detailed setup instructions including database configuration, see `Hipcortex_Env_Setup_Guide.md`.

Launch the combined MCP server (REST + gRPC) with:

```sh
cargo run --example mcp_server --features "web-server,grpc-server"
```

Open http://localhost:3000/metrics to view monitoring data.
How to Test as User:
https://github.com/farmountain/HipCortex/blob/main/How%20to%20Test%20as%20a%20User

If you encounter Codex container timeouts, run `scripts/codex_startup.sh` before heavy builds to prefetch dependencies and perform a quick `cargo check --all-features`.

See `examples/quickstart.rs` for a minimal programmatic usage demo.
`examples/world_model_example.rs` demonstrates the persistent world model API.
The new `examples/rag_export.rs` shows retrieving content via the RAG adapter and exporting it to PDF.
For WebAssembly extension, see `examples/plugin_host.rs` and run:
`cargo run --example plugin_host --features plugin`.
Detailed data model and extended architecture diagrams are available in [docs/data_model.md](docs/data_model.md) and [docs/architecture.md](docs/architecture.md).

## LLM & World Model Connectors

HipCortex ships with lightweight connectors for popular open-source models.
- Mistral, Falcon, DeepSeek and custom local LLMs
- World Model connector (JEPA style or mock implementation)

Example usage:

```sh
cargo run -- llm-generate "Tell me a story"
cargo run -- worldmodel-predict '{"state":"robot","action":"move"}'
```

## 🛠️ Use Cases


- **Agentic AI via OpenManus:** manage conversation context and reasoning traces for single or multi-agent systems.
- **AUREUS Reflexion loops:** integrate chain-of-thought feedback for deeper reasoning.
- **Edge Workflow Execution:** run on resource-constrained hardware thanks to Rust's performance and small footprint.
- **Multimodal learning or smart glasses:** use the PerceptionAdapter to capture images and text.
- **Real-Time Automation:** expose REST/gRPC APIs and upcoming CLI/web dashboards via the IntegrationLayer.
- **Knowledge Export:** use `rag_adapter` with `PdfExporter` or `NotionExporter` for long-term persistence.

## 👥 Key User Roles
- **AI Agent** – stores traces and retrieves context.
- **Developer** – integrates the engine via REST/gRPC or protocol adapters.
- **Architect** – designs workflows and multi-agent systems using the modules.
- **Researcher** – experiments with new memory types or reasoning loops.

## 🗺️ Use Case Map
1. **Store reasoning trace** through the PerceptionAdapter and TemporalIndexer.
2. **Query symbols** from the SymbolicStore.
3. **Update state** via the ProceduralCache or AureusBridge.
4. **Visualize world model** using real-time CLI and web dashboards.
## 🧪 Test & Automation
- **Run all tests:**  
  `cargo test`

- **Run benchmarks:**  
  `cargo bench`

- **Test suite:**
  - Unit and integration tests: `/tests/integration_tests.rs`
  - Property-based/fuzz tests: integrated using [proptest](https://docs.rs/proptest)
  - Add new test files to `/tests/` as needed
  - Additional examples cover multimodal smart-glasses and humanoid robotics perception traces
  - Recent perception tests: `multimodal_perception_tests.rs`, `smart_glasses_sit.rs`, `humanoid_perception_uat.rs`

- **CI/CD Ready:**  
  You can use GitHub Actions or any CI provider—add `.github/workflows/ci.yml` (see Rust starter templates) to run on every PR or push.

- **VS Code Integration:**  
  Open with VS Code. Test & bench tasks are already available via `.vscode/tasks.json` (Ctrl+Shift+B).

- **Best Practices:**
  - Always write failing tests first (TDD)
  - Ensure all modules have coverage before merge
  - Add benchmarks for any new algorithm or data structure

## 🏆 Project Success Criteria

HipCortex aims to remain stable and extensible as the ecosystem grows. The core
success criteria include:

- **Technical Architecture** – all modules compile cleanly and interoperate as
  described in the architecture diagram.
- **Data Integrity & Consistency** – no reasoning traces or symbolic graphs are
  lost or corrupted across sessions.
- **Scalability & Performance** – memory usage and runtime must support edge
  constraints while scaling horizontally on servers.
- **Extensibility** – pluggable perception encoders, symbolic stores and caches
  should be swappable without modifying core logic.
- **Observability & Debugging** – real-time logging and dashboards provide a
  clear view of every state transition.
- **Math & Statistical Soundness** – temporal indexes, concept graphs and FSM
  transitions follow well-defined models validated by tests or simulation.
- **Integration with LLMs** – connectors and protocols handle context without
  hallucination drift.
- **Documentation & Community** – README, architecture docs and examples remain
  up to date for contributors.

## 📊 Critical Data & Math Foundation

Each value stream collects metrics that align with solid statistical models.
Examples include:

- *PerceptionAdapter* – input token entropy and PCA/ICA statistics.
- *TemporalIndexer* – trace lifetimes modeled with Markov chains.
- *SymbolicStore* – graph degree variance and clustering coefficients.
- *ProceduralCache* – FSM state transition matrices and ergodicity checks.
- *AureusBridge* – Bayesian inference metrics for reasoning loops.
- *IntegrationLayer* – API latency and queuing statistics.

See [docs/architecture.md](docs/architecture.md) for the complete mapping of
| docs/memory_design.md | Math, logic and symbolic reasoning extension |
value stream activities to data collection targets and mathematical foundations.

## \ud83d\udccb Roadmap

The [roadmap document](docs/roadmap.md) lists completed modules and upcoming work.
Highlights include semantic compression, RAG adapters, persistent world memory,
real-time CLI/web tools, and expanded LLM connectors.



## Summary Table 
| Doc                  | Purpose                                               |
| -------------------- | ----------------------------------------------------- |
| README.md            | Project overview, structure, TDD, quickstart, roadmap |
| src/lib.rs           | Library entry (export modules)                        |
| docs/architecture.md | System design, extensibility, diagram                 |
| docs/memory_design.md | Math, logic and symbolic reasoning extension |
| docs/business_context.md | Business requirements and use cases |
| docs/data_model.md | MemoryRecord schema and API notes |
| docs/usage.md        | Build, test, bench, example, import                   |
| docs/integration.md  | Protocol/API plans, extension points                  |
| docs/roadmap.md      | Completed, active, planned modules                    |
| docs/contributing.md | Contribution guide, code/test policy                  |
| docs/agent.md        | Codex agent workflow and contribution guide           |
| LICENSE              | Apache License 2.0                                           |
