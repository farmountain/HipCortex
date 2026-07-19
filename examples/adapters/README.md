# Framework adapter snippets

Thin demos for wiring HipCortex into popular agent frameworks and HTTP automation tools.
Requires a running server (default `http://127.0.0.1:3030`) and the Python SDK:

```bash
pip install hipcortex
# or: pip install -e sdk/python
cargo run --bin webserver --no-default-features --features "web-server,petgraph_backend"
```

These match package templates under `sdk/python/hipcortex/install/templates/` (written by
`hipcortex install --scaffold`). Prefer package APIs (`from_settings` / `make_memory_tools` /
`client_from_settings`); no giant inline copies.

| File | Framework | What it shows |
|------|-----------|---------------|
| [`hipcortex_langchain.py`](hipcortex_langchain.py) | LangChain | `HipCortexMemory.from_settings()` |
| [`hipcortex_crewai.py`](hipcortex_crewai.py) | CrewAI | `make_memory_tools()` |
| [`hipcortex_autogen.py`](hipcortex_autogen.py) | AutoGen 0.4 | `HipCortexAutoGenMemory.from_settings()` |
| [`hipcortex_llamaindex.py`](hipcortex_llamaindex.py) | LlamaIndex | `client_from_settings` + chat store |
| [`hipcortex_pydantic_ai.py`](hipcortex_pydantic_ai.py) | Pydantic AI | `remember` / `recall` tools via client |
| [`hipcortex_dspy.py`](hipcortex_dspy.py) | DSPy | Save/load optimization traces |
| [`hipcortex_n8n_curl.sh`](hipcortex_n8n_curl.sh) | n8n / Make / Zapier | HTTP ingest + flat search curl |

## Notes

- Config: `HIPCORTEX_URL` or `.hipcortex/config.toml` (explicit URL still OK).
- Full adapters: `sdk/python/hipcortex/` (`langchain_memory`, `adapters/*`, `llamaindex_storage`).
- Scaffold source of truth: `sdk/python/hipcortex/install/templates/*.tmpl`.
- Rust examples stay in [`../`](../).
