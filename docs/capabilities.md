# HipCortex capability matrix

Honest surface coverage as of **v0.5.0** (code-grounded, not aspirational).

Legend:

| Mark | Meaning |
|------|---------|
| **Y** | First-class API / tool / method on this surface |
| **N** | Not exposed |
| **partial** | Indirect / session-only / subset of full op |
| **planned** | Intended next (see Known surface gaps) |
| **bg** | Server-side background only (no request API) |

Columns:

| Surface | Source of truth |
|---------|-----------------|
| REST | `src/web_server.rs` routes |
| MCP | `sdk/mcp/server.py` `TOOLS` |
| Python SDK | `sdk/python/hipcortex/client.py` |
| TS SDK | `sdk/typescript/src/client.ts` |
| VS Code LM | `languageModelTools` + `vscode.lm.registerTool` |
| VS Code Cmd | `package.json` `contributes.commands` + chat participant |
| LangChain | `sdk/python/hipcortex/langchain_memory.py` |
| CrewAI | `sdk/python/hipcortex/adapters/crewai.py` |

## Core ops matrix

| Op | REST | MCP | Python SDK | TS SDK | VS Code LM | VS Code Cmd | LangChain | CrewAI | Notes |
|----|------|-----|------------|--------|------------|-------------|-----------|--------|-------|
| add_memory | Y | Y | Y | Y | N | Y | partial | Y | LangChain: `save_context` → human/ai messages only |
| query | Y | partial | Y | Y | N | Y | partial | partial | MCP: `search_memory` falls back to `GET /memory/query`; CrewAI recall = conversation history |
| search | Y | Y | Y | Y | Y | Y | N | N | MCP name: `search_memory`; VS Code LM: `hipcortex_search` |
| forget | Y | Y | Y | Y | N | N | Y | Y | MCP: `forget_actor`; LangChain/CrewAI: session/agent clear |
| live_beliefs | Y | Y | Y | Y | partial | N | N | N | MCP: `get_live_beliefs`; TS: `liveBeliefs()`; VS Code LM search path may merge beliefs |
| link | Y | Y | Y | Y | N | Y | N | N | MCP: `link_memories`; chat `/link` |
| neighbors | Y | Y | Y | Y | N | N | N | N | MCP: `get_neighbors` |
| search_related | Y | Y | Y | Y | Y | N | N | N | MCP: `search_related`; VS Code LM: `hipcortex_graph_search` |
| delete | Y | Y | Y | Y | N | N | N | N | MCP: `delete_memory` (single id); not GDPR forget |
| stats | Y | Y | Y | Y | N | N | N | N | MCP: `get_stats` |
| health | Y | N | Y | Y | Y | Y | N | N | MCP gap; VS Code LM: `hipcortex_health` |
| reflect | Y | Y | Y | Y | partial | N | N | N | REST `POST /memory/reflect`. MCP/Python/TS: `reflect`. VS Code API client has `reflect()` used internally |
| predict | Y | Y | Y | Y | Y | Y | N | N | REST `POST/GET /worldmodel/predict`. MCP/Python/TS: `predict` (state+action). VS Code LM: `hipcortex_predict` |
| rollout | Y | N | Y | Y | Y | N | N | N | REST `POST /worldmodel/rollout`; VS Code LM: `hipcortex_rollout` |
| purge | bg | Y | N | N | N | N | N | N | REST: background TTL eviction only (no dedicated route). MCP: `purge_expired` |
| search_code | Y | Y | N | N | N | N | N | N | REST `GET /graph/search`; MCP: `search_code` |

## MCP tools inventory (must stay in sync)

Checker `scripts/check_capabilities.py` greps these names from `TOOLS` and requires each string to appear in this file.

| MCP tool name | Matrix op | Status |
|---------------|-----------|--------|
| `add_memory` | add_memory | Y |
| `search_memory` | search | Y |
| `forget_actor` | forget | Y |
| `get_stats` | stats | Y |
| `search_code` | search_code | Y |
| `link_memories` | link | Y |
| `get_neighbors` | neighbors | Y |
| `search_related` | search_related | Y |
| `delete_memory` | delete | Y |
| `get_live_beliefs` | live_beliefs | Y |
| `purge_expired` | purge | Y |
| `reflect` | reflect | Y |
| `predict` | predict | Y |

**Not in MCP `TOOLS` today (honest gaps):** `rollout`, `health`, dedicated `query` tool.

MCP serverInfo version in `sdk/mcp/server.py`: **0.5.0**.

## Known surface gaps (priority)

1. **MCP health / rollout** — available on REST + SDKs + VS Code LM (rollout/health) but not MCP.
2. **Python/TS SDK** — `live_beliefs` / `reflect` / `predict` parity done (v0.5.0+); adapters still lack graph/WM/intelligence ops.
3. **LangChain / CrewAI** — memory chat adapters only (add/query/forget style); no graph/WM/intelligence ops.
4. **VS Code LM** — strong on search/health/predict/rollout/graph; no first-class add/forget/delete LM tools (commands cover add/query).

## How to re-check

```sh
python scripts/check_capabilities.py
python scripts/check_capabilities.py --check-mcp
```

Exit 0 if every MCP `TOOLS` name appears in this doc and MCP version string is present. Exit 1 on drift.

## Framework package-first

LangChain: `HipCortexMemory.from_settings()`. CrewAI: `make_memory_tools()`. See `examples/adapters/`.

