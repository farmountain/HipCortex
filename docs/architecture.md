# HipCortex Architecture

HipCortex is a modular AI memory engine with these key principles:

- **Temporal Memory:** Short-term and long-term memory, managed with decay and LRU.
- **Procedural Memory:** FSM-driven, agentic "reasoning/action" traces, for procedural or regenerative workflows.
- **Symbolic Memory:** Graph-based, human-interpretable key-value and concept memory.
- **Perception Adapter:** Handles multimodal input (text, embeddings, agent messages, vision—future).
- **Aureus Bridge:** Reflexion and reasoning integration (for AUREUS, chain-of-thought, and agent feedback).
- **Integration Layer:** Ready for REST/gRPC/agent protocols (OpenManus, MCP, etc).

## Module Interaction Diagram

```mermaid
flowchart TD
    Percept[PerceptionAdapter] --> Trace[Memory Trace/Concept]
    Trace --> STM[TemporalIndexer]
    Trace --> Symb[SymbolicStore]
    STM & Symb --> FSM[ProceduralCache (FSM)]
    FSM --> Reason[AureusBridge]
    Reason --> API[IntegrationLayer]
