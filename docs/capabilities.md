# HipCortex capability matrix

Honest surface coverage as of **product v0.5.1 / VSIX v0.5.7** (code-grounded, not aspirational).

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
| rollout | Y | Y | Y | Y | Y | N | N | N | REST `POST /worldmodel/rollout`; MCP: `rollout` (dirichlet/mcts/ensemble + goal_state); VS Code LM: `hipcortex_rollout` |
| purge | bg | Y | N | N | N | N | N | N | REST: background TTL eviction only (no dedicated route). MCP: `purge_expired` |
| search_code | Y | Y | N | N | N | N | N | N | REST `GET /graph/search`; MCP: `search_code` |
| graph_ppr / topo | Y | Y | N | N | Y | N | N | N | REST `/topo/*`; MCP: `graph_ppr`, `deconstruct_hypothesis`, `check_topo_edge`; LM: `hipcortex_topo_ppr`, `hipcortex_deconstruct`, `hipcortex_check_edge` |
| can_execute | Y | Y | partial | N | Y | N | N | N | REST self/health gate; MCP + LM: `can_execute` / `hipcortex_can_execute` |

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
| `graph_ppr` | graph_ppr / topo | Y |
| `deconstruct_hypothesis` | graph_ppr / topo | Y |
| `check_topo_edge` | graph_ppr / topo | Y |
| `rollout` | rollout | Y |
| `can_execute` | can_execute | Y |

**Not in MCP `TOOLS` today (honest gaps):** dedicated `health` tool (use REST `/health` or VS Code LM `hipcortex_health`), dedicated `query` tool (use `search_memory` / `get_live_beliefs`).

MCP serverInfo version in `sdk/mcp/server.py`: **0.5.1**. VS Code extension package: **0.5.7** (10 LM tools).

## Known surface gaps (priority)

1. **MCP health** — REST + VS Code LM have first-class health; MCP uses `get_stats` only.
2. **Python/TS SDK** — `live_beliefs` / `reflect` / `predict` / `rollout` parity present; topo (`graph_ppr` / deconstruct / check_edge) still MCP/REST-first.
3. **LangChain / CrewAI** — memory chat adapters only (add/query/forget style); no graph/WM/intelligence ops.
4. **VS Code LM** — 10 tools (search/health/predict/rollout/graph/causal/topo suite/can_execute); no first-class add/forget/delete LM tools (commands cover add/query).

## How to re-check

```sh
python scripts/check_capabilities.py
python scripts/check_capabilities.py --check-mcp
```

Exit 0 if every MCP `TOOLS` name appears in this doc and MCP version string is present. Exit 1 on drift.

## Framework package-first

LangChain: `HipCortexMemory.from_settings()`. CrewAI: `make_memory_tools()`. See `examples/adapters/`.

