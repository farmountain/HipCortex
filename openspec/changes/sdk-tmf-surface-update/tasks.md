# Implementation Tasks: SDK & Extension Surface Update for Tiered Memory Foundation (TMF)

## Phase 1 — Version Bumps

- [x] 1.1 Bump `sdk/python/pyproject.toml` → `version = "0.4.0"`
- [x] 1.2 Bump `sdk/typescript/package.json` → `"version": "0.4.0"`
- [x] 1.3 Bump `vscode-extension/package.json` → `"version": "0.4.0"`

---

## Phase 2 — Python SDK (`sdk/python/hipcortex/`)

### 2a — `client.py` (sync)
- [x] 2.1 Add `link_memories(source_id, target_id, relation="related")` → `POST /memory/link`
- [x] 2.2 Add `get_neighbors(record_id, limit=10)` → `GET /memory/neighbors/{record_id}`
- [x] 2.3 Add `search_related(seed_id, limit=10)` → `GET /memory/search/related?seed_id=...&limit=...`
- [x] 2.4 Add `delete_memory(record_id)` → `DELETE /memory/{record_id}`
- [x] 2.5 Extend `query_memory(include_expired: bool = False)` — add `include_expired` param to query string when `True`

### 2b — `async_client.py` (async)
- [x] 2.6 Mirror all 4 new methods as `async def` with `await self._client.<method>(...)` — identical signatures to sync client
- [x] 2.7 Extend `query_memory` with `include_expired` param same as sync

---

## Phase 3 — TypeScript SDK (`sdk/typescript/src/`)

### 3a — `types.ts`
- [x] 3.1 Add `LinkMemoriesRequest`, `LinkMemoriesResponse` interfaces
- [x] 3.2 Add `NeighborsResponse` interface
- [x] 3.3 Add `RelatedSearchParams`, `RelatedSearchResponse` interfaces
- [x] 3.4 Add `DeleteMemoryResponse` interface
- [x] 3.5 Extend `StatsResponse` with `active_records: number`
- [x] 3.6 Extend `QueryParams` with `include_expired?: boolean`

### 3b — `client.ts`
- [x] 3.7 Add `linkMemories(req: LinkMemoriesRequest)` → `POST /memory/link`
- [x] 3.8 Add `getNeighbors(recordId, limit?)` → `GET /memory/neighbors/{recordId}`
- [x] 3.9 Add `searchRelated(seedId, limit?)` → `GET /memory/search/related?seed_id=...`
- [x] 3.10 Add `deleteMemory(recordId)` → `DELETE /memory/{recordId}`
- [x] 3.11 Extend `queryMemory()` to pass `include_expired` param when set
- [x] 3.12 Run `npm run build` — verify 0 TypeScript errors ✓

---

## Phase 4 — MCP Server (`sdk/mcp/server.py`)

- [x] 4.1 Add `link_memories` tool to `TOOLS` list with `source_id`, `target_id`, `relation` input schema
- [x] 4.2 Add `get_neighbors` tool with `record_id`, `limit` input schema
- [x] 4.3 Add `search_related` tool with `seed_id`, `limit` input schema
- [x] 4.4 Add `delete_memory` tool with `record_id` input schema
- [x] 4.5 Add `handle_link_memories()`, `handle_get_neighbors()`, `handle_search_related()`, `handle_delete_memory()` handler functions
- [x] 4.6 Wire new tools into the `elif tool_name == ...` dispatch block in `handle_call_tool()`

---

## Phase 5 — VS Code Extension (`vscode-extension/`)

### 5a — `package.json`
- [x] 5.1 Add `hipcortex_graph_search` entry to `languageModelTools` array (PPR search LM tool)
- [x] 5.2 Add `/link` entry to `chatParticipants[0].commands` array

### 5b — `extension.ts`
- [x] 5.3 Add `handle_link_memory()` async function calling `POST /memory/link`
- [x] 5.4 Add `handle_graph_search()` async function calling `GET /memory/search/related`
- [x] 5.5 Register `hipcortex_graph_search` as a `vscode.lm.registerTool` handler
- [x] 5.6 Wire `/link` chat command into the existing `chatHandler` switch/if block
- [x] 5.7 Run `npm run compile` — verify 0 TypeScript errors ✓

---

## Phase 6 — Build & Package

- [x] 6.1 Build Python sdist + wheel: `cd sdk/python && python -m build`
- [x] 6.2 Build npm package: `cd sdk/typescript && npm run build`
- [x] 6.3 Bundle VSIX: `cd vscode-extension && npm run package && npx vsce package --no-dependencies`
- [x] 6.4 Verify VSIX file created: `hipcortex-memory-0.4.0.vsix` ✓

---

## Phase 7 — Publish

- [ ] 7.1 Publish Python to PyPI: `twine upload sdk/python/dist/hipcortex-0.4.0*`
- [ ] 7.2 Publish npm: `cd sdk/typescript && npm publish`
- [ ] 7.3 Publish VSIX to VS Marketplace: `cd vscode-extension && npx vsce publish --no-dependencies`

---

## Phase 8 — Smoke Verification

- [ ] 8.1 `pip install hipcortex==0.4.0` + call `link_memories()` against running server
- [ ] 8.2 `npm install hipcortex@0.4.0` + call `linkMemories()` in a test script
- [ ] 8.3 Install `hipcortex-memory-0.4.0.vsix` locally and exercise `@hipcortex /link` in chat
- [ ] 8.4 Verify `GET /stats` response includes `active_records` in `StatsResponse` type

