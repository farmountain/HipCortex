## Approach

Pure TypeScript change to `vscode-extension/src/extension.ts`. No Rust server changes required — both `POST /memory/search` and the causal routes are already fully implemented and auth-whitelisted.

## Design Decisions

### D1: Search handler returns scored results, query handler returns raw records

`handleSearchMemory` formats `{score, record}` tuples from `POST /memory/search`. Score is shown as a percentage (e.g. `[87.3%]`). `handleQueryMemory` continues returning grouped-by-date raw records. The two UX flows are intentionally different.

### D2: `searchMemory()` added to `HipCortexAPI` class (not the extension class)

Keeps the API layer clean. All HTTP calls live in `HipCortexAPI`. The chat handler only calls API methods, never axios directly.

### D3: `hipcortex_causal` uses a `mode` discriminator

Single tool with `mode: "graph" | "counterfactual" | "intervention"` is cleaner than three separate tools. Copilot agents can describe what they want in natural language and the tool routes internally.

### D4: Causal tool accesses `baseUrl` via the existing `HipCortexAPI` instance

The LM tool creates a `new HipCortexAPI()` (same pattern as other tools in the file) and calls the `predictState`-style pattern for causal — using `axios` directly inside the invoke closure, same as `hipcortex_predict` does for reflect.

## Component Design

### `HipCortexAPI.searchMemory()` (new method, ~12 lines)

```
Input: query: string, limit: number = 10
Output: Promise<{ results: Array<{ score: number; record: MemoryRecord }>; total: number }>
HTTP: POST /memory/search   body: { query, limit }
Headers: Authorization Bearer if apiKey set
```

### Dispatch split at extension.ts:434 (modification)

```
Before:
  command.startsWith('query') || command.startsWith('search') → handleQueryMemory

After:
  command.startsWith('query')  → handleQueryMemory      (GET /memory/query, filter)
  command.startsWith('search') → handleSearchMemory     (POST /memory/search, scored)
```

### `handleSearchMemory()` (new private method, ~35 lines)

```
Input: request.prompt stripped of leading "search "
Parse: optional "limit: N" from prompt
Call:  this.api.searchMemory(cleanQuery, limit)
Output:
  - Header: "🔍 Semantic search results (N results)"
  - Per result: "[87.3%] [action] target (actor)"
  - Empty: "No semantically similar memories found."
  - Token savings footer via tokenTracker
```

### `hipcortex_causal` LM tool (new registerTool, ~45 lines)

```
modelDescription: "Run causal/counterfactual query over WorldModel causal graph.
  mode=graph: inspect graph structure
  mode=counterfactual: {intervention: string, query: string}
  mode=intervention: {variable: string, value: any}"

invoke:
  mode === 'counterfactual' → POST /worldmodel/causal/counterfactual {intervention, query}
  mode === 'intervention'   → POST /worldmodel/causal/intervention   {variable, value}
  default (graph)           → GET  /worldmodel/causal
  return JSON.stringify(result.data, null, 2)
```

Registration: push to `context.subscriptions` alongside existing 4 tools at line 885.

## File Map

```
vscode-extension/src/extension.ts
  ├─ HipCortexAPI class
  │   └─ + searchMemory(query, limit)           ← ~line 405 (after searchRelated)
  ├─ HipCortexChatParticipant class
  │   ├─ dispatch at line 434: split query/search
  │   └─ + handleSearchMemory(request, stream)  ← after handleQueryMemory (~line 645)
  └─ LM tool registration block
      └─ + hipcortex_causal tool                ← after graphSearchTool (~line 884)
```

## Verification

1. `cd vscode-extension && npm run compile` → 0 errors
2. `@HipCortex query actor:user` → still calls GET /memory/query
3. `@HipCortex search recent decisions` → calls POST /memory/search, shows scored output
4. LM tool list includes `hipcortex_causal` in extension host logs
