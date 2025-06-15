# Roadmap & Future Modules

## Completed
- Modular memory architecture
- Temporal indexer (STM/LTM)
- FSM procedural cache
- Symbolic key-value and graph store
- Multimodal perception adapter
- Vision encoder module
- Reflexion/agent integration stubs
- Initial LLM clients (OpenAI, Claude, Ollama)
- TDD, benchmarks, VS Code dev config

## In Progress / Planned
- Semantic cache/compression
- Local inference via Ollama or custom backends

## Completed Enhancements
- Persistent world model memory
- Real-time agentic CLI and Web UI
- Expanded open-source LLM connectors (Llama, DeepSeek, etc.)
- EffortEvaluator & ConfidenceRegulator for collapse resistance metrics
- HypothesisManager and quantized state tree for multi-path reasoning
- Procedural backtracking and fallback logic
- Puzzle benchmark harness for algorithmic planning tasks

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

1. **Document All Data Models** – provide schemas and diagrams for each memory structure.
2. **Implement Runtime Validators** – check FSM reachability and graph connectivity automatically.
3. **Add Property-Based Tests** – stress-test symbolic and temporal modules with proptest.
4. **Pilot Statistical Monitoring** – collect moving averages and standard deviation for key metrics.
5. **Automate Observability Dashboards** – integrate logs and metrics in the web dashboard.
6. **Deploy Enhancement Advisor** – surface reasoning-based suggestions for users to approve and refine.

PRs for new modules and improvements are highly encouraged!
