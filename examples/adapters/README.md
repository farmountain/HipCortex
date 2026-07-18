# Framework adapter snippets

Thin demos for wiring HipCortex into popular agent frameworks and HTTP automation tools.
Requires a running server (default `http://127.0.0.1:3030`) and the Python SDK:

```bash
pip install hipcortex
# or: pip install -e sdk/python
cargo run --bin webserver --no-default-features --features "web-server,petgraph_backend"
```

These match the starter files produced by `hipcortex install` for agent frameworks.
Copy a file into your project and uncomment the framework-specific wiring.

| File | Framework | What it shows |
|------|-----------|---------------|
| [`hipcortex_langchain.py`](hipcortex_langchain.py) | LangChain | Drop-in `HipCortexMemory` / `AsyncHipCortexMemory` |
| [`hipcortex_crewai.py`](hipcortex_crewai.py) | CrewAI | Remember / Recall / Forget tools |
| [`hipcortex_autogen.py`](hipcortex_autogen.py) | AutoGen 0.4 | `HipCortexAutoGenMemory` (+ 0.3 hooks) |
| [`hipcortex_llamaindex.py`](hipcortex_llamaindex.py) | LlamaIndex | `HipCortexChatStore` + storage context |
| [`hipcortex_pydantic_ai.py`](hipcortex_pydantic_ai.py) | Pydantic AI | `remember` / `recall` as agent tools |
| [`hipcortex_dspy.py`](hipcortex_dspy.py) | DSPy | Save/load optimization traces via memory |
| [`hipcortex_n8n_curl.sh`](hipcortex_n8n_curl.sh) | n8n / Make / Zapier | HTTP ingest + flat search curl examples |

## Notes

- Point `HipCortexClient(...)` / `url=` at your server or managed tier.
- Full adapters live in `sdk/python/hipcortex/` (`langchain_memory`, `adapters/*`, `llamaindex_storage`).
- Rust examples (quickstart, world model, etc.) stay in [`../`](../).
