# Roadmap & Future Modules

## Completed
- ✅ DONE: Modular memory architecture
- ✅ DONE: Temporal indexer (STM/LTM)
- ✅ DONE: FSM procedural cache
- ✅ DONE: Symbolic key-value and graph store
- ✅ DONE: Multimodal perception adapter
- ✅ DONE: Vision encoder module
- ✅ DONE: Reflexion/agent integration stubs
- ✅ DONE: Initial LLM clients (OpenAI, Claude, Ollama)
- ✅ DONE: TDD, benchmarks, VS Code dev config

- ✅ DONE: Semantic cache/compression
## Completed Enhancements
- ✅ DONE: Persistent world model memory
- ✅ DONE: Real-time agentic CLI and Web UI
- ✅ DONE: Expanded open-source LLM connectors (Llama, DeepSeek, etc.)
- ✅ DONE: EffortEvaluator & ConfidenceRegulator for collapse resistance metrics
- ✅ DONE: HypothesisManager and quantized state tree for multi-path reasoning
- ✅ DONE: Procedural backtracking and fallback logic
- ✅ DONE: Puzzle benchmark harness for algorithmic planning tasks

## Roadmap Highlights
- **Vision encoder**: Integrate image/embedding modules for visual reasoning.
- **Semantic compression**: Memory-efficient summary/compression for long-term storage.
- **RAG/Notion export**: Retrieval adapters and Notion/PDF exporters implemented.
- **World model memory**: Store agent/environment state and simulate context.
- **Real-time CLI/Web**: Manage, debug, and visualize agentic memory interactively.
- **Collapse metrics**: EffortEvaluator and ConfidenceRegulator measure reasoning fatigue and collapse_score.
- **Puzzle benchmark suite**: Validate complex planning tasks like Tower of Hanoi for regression testing.

---

## Next Steps

The following actions reinforce the math-driven data foundation:
- **Document All Data Models** – provide schemas and diagrams for each memory structure.
- **Implement Runtime Validators** – check FSM reachability and graph connectivity automatically.
- **Add Property-Based Tests** – stress-test symbolic and temporal modules with proptest.
- **Pilot Statistical Monitoring** – collect moving averages and standard deviation for key metrics.
- **Automate Observability Dashboards** – integrate logs and metrics in the web dashboard using the new `MonitoringService` and Tauri UI.
- **Deploy Enhancement Advisor** – surface reasoning-based suggestions for users to approve and refine.
- **Local inference via Ollama or custom backends**

## Post-MVP Ideas
- Persistence for FSM backend
- Advanced LLM plugin hosting
- Semantic cache eviction policies
- Multi-dashboard views for admins
