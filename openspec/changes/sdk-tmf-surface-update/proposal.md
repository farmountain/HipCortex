# Change Proposal: SDK & Extension Surface Update for Tiered Memory Foundation (TMF)

## Why

The `tiered-memory-foundation` branch (merged as commit `3126373`) shipped **+992 lines across 10 files**, introducing 6 new REST endpoints and 2 enhanced response fields on the HipCortex server. None of these capabilities are yet exposed in any of the four distribution surfaces:

| Distribution Surface | Package Name | Current Version | Publisher |
|:---|:---|:---|:---|
| Python SDK (`sdk/python/`) | `hipcortex` (PyPI) | `0.3.0` | `pip install hipcortex` |
| TypeScript SDK (`sdk/typescript/`) | `hipcortex` (npm) | `0.3.0` | `npm install hipcortex` |
| VS Code Extension (`vscode-extension/`) | `hipcortex-memory` (Marketplace) | `0.3.3` | `farmountain` publisher |
| MCP Server (`sdk/mcp/`) | `hipcortex` (pip-bundled) | inline in SDK | stdio transport |

Until these surfaces are updated, any agent or user who installs the published packages cannot access:
- **Graph-based memory linking** (`POST /memory/link`)
- **Neighborhood traversal** (`GET /memory/neighbors/:id`)
- **Personalized PageRank search** (`GET /memory/search/related`)
- **Per-record deletion** (`DELETE /memory/:id`)
- **Expired record visibility** (`include_expired` query param on `/memory/query`)
- **Active record count** (now present in `/stats` response)

## What Changes

### New API Methods Across All SDK Surfaces

| Server Endpoint | Python client.py | Python async_client.py | TS SDK client.ts | MCP server.py | VSIX Extension |
|:---|:---|:---|:---|:---|:---|
| `POST /memory/link` | `link_memories()` | `link_memories()` | `linkMemories()` | `link_memories` tool | no (see design) |
| `GET /memory/neighbors/:id` | `get_neighbors()` | `get_neighbors()` | `getNeighbors()` | `get_neighbors` tool | no (see design) |
| `GET /memory/search/related` | `search_related()` | `search_related()` | `searchRelated()` | `search_related` tool | `hipcortex_graph_search` LM tool |
| `DELETE /memory/:id` | `delete_memory()` | `delete_memory()` | `deleteMemory()` | `delete_memory` tool | no (see design) |
| `include_expired` param | `query_memory(include_expired=False)` | same | `queryMemory({includeExpired})` | n/a (MCP abstracts) | n/a |
| `active_records` in `/stats` | auto (dict pass-through) | auto | `StatsResponse.active_records` | auto (string output) | auto |

### Version Bumps

All four surfaces bump to **`0.4.0`**. See Design Decision #1 for full rationale.

| File | Field | Old | New |
|:---|:---|:---|:---|
| `sdk/python/pyproject.toml` | `version` | `0.3.0` | `0.4.0` |
| `sdk/typescript/package.json` | `version` | `0.3.0` | `0.4.0` |
| `vscode-extension/package.json` | `version` | `0.3.3` | `0.4.0` |
| `Cargo.toml` | `version` | `0.1.0` | `0.1.0` (no change — server semver is independent) |

### VS Code Extension

Two targeted additions to `package.json` and `extension.ts`:
1. Add `hipcortex_graph_search` as a new `languageModelTools` entry (PPR-powered contextual search).
2. Add `@hipcortex /link` as a new `chatParticipants.commands` entry.

No new VS Code commands (palette) are added — graph link and delete are power-user features that don't benefit from interactive prompts.

## Capabilities Affected

- **New**: `tiered-memory-graph` — SDK surface for memory-to-memory graph edges, neighborhood traversal, and PPR-ranked related memory search.
- **Modified**: `memory-query` — adds `include_expired` optional parameter.
- **Modified**: `memory-stats` — `active_records` field now exposed in TS types.
