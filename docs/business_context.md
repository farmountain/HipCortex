# Business Context

HipCortex provides a modular foundation for building agent memory and reasoning services. It can be embedded in edge devices or scaled out as a microservice.

## Problem
AI agents lack persistent, modular and contextual memory.

## Mission
Build a memory engine that merges symbolic reasoning, temporal relevance, procedural logic and perception.

Typical use cases include:

- Agentic AI via OpenManus or similar protocols.
- AUREUS Reflexion loops and chain-of-thought reasoning.
- Multimodal learning or smart glasses capturing perception traces.
- Workflow automation on resource constrained hardware.
- LLM connectors (OpenAI, Claude, Ollama) enable prompt-based reflexion.
- Knowledge graph or retrieval‑augmented pipelines.
- Research platforms exploring symbolic and procedural memory.

The project is written in Rust to ensure performance and safety, with optional web and desktop frontends.

## Key User Roles
- **AI Agent** – interacts with the engine to store and retrieve traces.
- **Developer** – integrates HipCortex via REST/gRPC or custom adapters.
- **Architect** – designs workflows and multi-agent systems.
- **Researcher** – experiments with new memory types or reasoning loops.

## High-Level Use Case Map
1. **Store reasoning trace** – capture input via the PerceptionAdapter and persist it with the TemporalIndexer and SymbolicStore.
2. **Query symbols** – explore the graph-based SymbolicStore for concepts and relationships.
3. **Update state** – manage FSM transitions or Reflexion loops with the ProceduralCache and AureusBridge.
4. **Visualize world model** – real-time CLI and web dashboards show stored state and reasoning traces.
5. **Review suggestions** – the Enhancement Advisor analyzes metrics from every component and recommends improvements that users can approve.
