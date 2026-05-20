# Plan: HipCortex SaaS Features (P0 + P1)

## Summary
Add SDK/integration surface (Python SDK, LangChain, LlamaIndex, AutoGen, CrewAI) and Rust REST
endpoints (GDPR forget, coherence status) to position HipCortex as a deployable AI-memory SaaS.

## User Story
As an AI developer, I want to plug HipCortex into LangChain/AutoGen/CrewAI via pip install and
hit `/memory/forget/:actor` for GDPR compliance, so that HipCortex becomes the drop-in memory
backend for agentic SaaS products.

## Metadata
- **Complexity**: Large
- **Estimated Files**: 12

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src/memory_store.rs` | UPDATE | Add `delete_by_actor` (GDPR) |
| `src/web_server.rs` | UPDATE | Add forget + coherence endpoints |
| `sdk/python/hipcortex/__init__.py` | CREATE | Package root |
| `sdk/python/hipcortex/client.py` | CREATE | HTTP client |
| `sdk/python/hipcortex/langchain_memory.py` | CREATE | LangChain BaseChatMemory subclass |
| `sdk/python/hipcortex/llamaindex_storage.py` | CREATE | LlamaIndex StorageContext wrapper |
| `sdk/python/hipcortex/adapters/__init__.py` | CREATE | Adapters package |
| `sdk/python/hipcortex/adapters/autogen.py` | CREATE | AutoGen ConversableAgent hook |
| `sdk/python/hipcortex/adapters/crewai.py` | CREATE | CrewAI BaseTool subclass |
| `sdk/python/setup.py` | CREATE | pip-installable package |
| `benchmarks/python_benchmark.py` | CREATE | Latency benchmark vs Mem0 |

## NOT Building
- Persistent CoherenceChecker state across requests (fresh checker per request is P1 sufficient)
- GDPR deletion propagation into graph DB backends (neo4j/postgres — feature-gated, not default)
- Full LlamaIndex KVStore implementation (wrapper pattern is sufficient)
- Authentication/authorization on forget endpoint

## Acceptance Criteria
- [ ] `cargo build --no-default-features --features "petgraph_backend"` passes (no web-server needed for core)
- [ ] `cargo build --no-default-features --features "web-server,petgraph_backend"` passes
- [ ] `DELETE /memory/forget/alice` removes all alice records + audit entry
- [ ] `GET /coherence/status` returns JSON with `coherence_score`
- [ ] `pip install -e sdk/python/` succeeds
- [ ] LangChain `HipCortexMemory` usable as drop-in memory in LangChain chain
- [ ] Benchmark outputs latency table to stdout
