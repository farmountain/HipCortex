
---

## **docs/integration.md**

```markdown
# Integration Guide

This document describes how to integrate HipCortex with agent frameworks, APIs, and external systems.

---

## Integration Readiness

### REST/gRPC API
- IntegrationLayer (`src/integration_layer.rs`) exposes message handling that can
  be wired to REST or gRPC endpoints.
- To spin up the built-in REST server, compile with the `web-server` feature and
  call `web_server::run(addr)` or `run_with_store` from your application.
- To run the gRPC service, compile with the `grpc-server` feature and call
  `grpc_server::serve(addr, store).await`.
  This provides a basic `MemoryService` for adding and listing memory records.

### Agent Protocols (Planned)
- **OpenManus** and **MCP** adapters will translate their native message formats into the internal `PerceptInput` structure. A small protocol bridge will live in `IntegrationLayer` so agents can talk to HipCortex directly over these protocols.
- Agent messages flow **into** the engine through `PerceptionAdapter` which normalizes text, embeddings or structured agent events.
- `IntegrationLayer` then exposes the results back over the same channel (or via REST/gRPC) so the calling agent receives the action or trace response.

### Chain-of-Thought & Reflexion
- AureusBridge connects to agentic/LLM reasoning modules.
- Supports embedding AUREUS or other CoT frameworks for feedback loops.

### LLM Connectors
- Basic clients exist for OpenAI, Claude, and Ollama.
- Planned support for additional open-source models and local inference.
- Expose prompt-based reflexion by creating a client and passing it to
  `AureusBridge`:

```rust
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::llm_clients::{openai::OpenAIClient, claude::ClaudeClient, ollama::OllamaClient};

// Choose a client. Environment variables can store credentials.
let openai = OpenAIClient::new(std::env::var("OPENAI_API_KEY")?, "gpt-3.5-turbo");
let claude = ClaudeClient::new(std::env::var("CLAUDE_API_KEY")?, "claude-3-sonnet");
let ollama = OllamaClient::new(std::env::var("OLLAMA_URL")?, "llama3");

// Attach to AureusBridge for reflexion loops
let mut bridge = AureusBridge::with_client(Box::new(openai));
// bridge.set_client(Box::new(claude)); // swap if desired
```

The CLI uses `OPENAI_API_KEY` automatically for the `prompt` command. Other
connectors can be configured similarly in your application.

### RAG & Notion/PDF Export
- `rag_adapter` provides local and HTTP retrieval adapters.
- `knowledge_export` includes `NotionExporter` and `PdfExporter` for memory tracing and reporting.

### Real-Time CLI/Web (Planned)
- CLI and web server interface planned for live agentic memory debugging and management.

---

## How To Integrate

1. Implement your REST/gRPC server in `integration_layer.rs`.
2. Use PerceptionAdapter to receive inputs from your agent, CLI, or web API.
3. Process memory/logic as needed (STM, FSM, symbolic, etc.).
4. Use IntegrationLayer to return results, traces, or actions.
5. Extend with protocol adapters for OpenManus, MCP, or other frameworks.

---

For help, examples, or API changes, submit issues or PRs with your use-case!
