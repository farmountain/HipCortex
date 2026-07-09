# Design: SDK & Extension Surface Update for Tiered Memory Foundation (TMF)

## Design Decision #1 — Version Bump Strategy

### Recommendation: `0.3.x → 0.4.0` (Minor Bump) Across All SDK/Extension Surfaces

**Rationale:**
- All 6 new endpoints are **purely additive**. No existing method signatures, return types, or behaviours change.
- Semver mandates a **minor version** bump (`MINOR`) for backwards-compatible new functionality.
- `0.3.3 → 0.4.0` normalizes the VSIX version with the SDK versions (previously diverged: VSIX was `0.3.3`, SDKs were `0.3.0`). A shared minor version across all surfaces makes the changelog story clean: *"v0.4 = TMF graph features"*.
- `Cargo.toml` (`0.1.0`) is **not changed**. The Rust server semver tracks internal crate stability, not the public SDK contract. Bumping it would trigger cargo.lock drift and is unnecessary.

**What is NOT a breaking change here (confirmed):**
- `StatsResponse` gaining `active_records` — existing consumers receive an extra key; they don't break.
- `query_memory(include_expired=False)` — default is `False`, so all existing calls without the param behave identically.

---

## Design Decision #2 — VSIX Extension Scope: What to Add vs. Skip

### Recommendation: Add `hipcortex_graph_search` LM Tool + `@hipcortex /link` Chat Command Only

The VS Code extension exposes HipCortex to Copilot via two surfaces: **`languageModelTools`** (called by the LLM proactively) and **`chatParticipants.commands`** (user-invoked slash commands).

**Add** `hipcortex_graph_search` as a `languageModelTools` entry:
- Wraps `GET /memory/search/related?seed_id=<id>&limit=<n>`
- The LLM can call this proactively after a `hipcortex_search` to expand the retrieval set using PPR-ranked related memories.
- This unlocks the single highest-value TMF feature for Copilot workflows.

**Add** `@hipcortex /link` as a chat command:
- Allows users to type `@hipcortex /link <record-id-1> <record-id-2> <relation>` to manually create graph edges.
- Implemented as `handle_link_memory()` in `extension.ts`, calling `POST /memory/link`.

**Do NOT add** palette commands for delete or neighbor traversal:
- `DELETE /memory/:id` is a destructive power-user operation — exposing it as a palette command without a confirmation UI would be reckless. It belongs in the SDK, not the extension UX.
- `GET /memory/neighbors/:id` is a raw graph API. It has no natural interactive prompt flow. It's better surfaced via the SDK for programmatic use.

---

## Design Decision #3 — MCP Server Tool Scope

### Recommendation: Add `link_memories`, `get_neighbors`, `search_related`, `delete_memory` as MCP Tools

The MCP server (`sdk/mcp/server.py`) is a JSON-RPC stdio bridge that exposes HipCortex to all MCP-compatible AI coding assistants (Claude Code, Cursor, Windsurf, Zed AI). It currently has 4 tools: `add_memory`, `search_memory`, `forget_actor`, `get_stats`, `search_code`.

All 4 new TMF endpoints translate directly to MCP tools. The `include_expired` param is a detail to fold into the `search_memory` tool's optional fields rather than a new tool.

**Tool input schemas:**

```
link_memories:
  required: [source_id, target_id]
  optional: [relation (default: "related")]
  → POST /memory/link {source_id, target_id, relation}

get_neighbors:
  required: [record_id]
  optional: [limit (default: 10)]
  → GET /memory/neighbors/{record_id}?limit=...

search_related:
  required: [seed_id]
  optional: [limit (default: 10)]
  → GET /memory/search/related?seed_id={seed_id}&limit=...

delete_memory:
  required: [record_id]
  → DELETE /memory/{record_id}
```

---

## Design Decision #4 — Python SDK Method Signatures (No Ambiguity)

### `client.py` (sync) and `async_client.py` (async) — Identical signatures, different call style

```python
# Sync (client.py):
def link_memories(self, source_id: str, target_id: str, relation: str = "related") -> Dict[str, Any]:
    """Create a directed graph edge between two memory records.
    POST /memory/link
    Returns: {"success": bool, "source_id": str, "target_id": str, "relation": str}
    """

def get_neighbors(self, record_id: str, limit: int = 10) -> List[Dict[str, Any]]:
    """Return neighboring memory records linked via the CausalTopoGraph.
    GET /memory/neighbors/{record_id}
    Returns: list of MemoryRecord dicts
    """

def search_related(self, seed_id: str, limit: int = 10) -> List[Dict[str, Any]]:
    """PPR-ranked related memory search seeded from a given record ID.
    GET /memory/search/related?seed_id={seed_id}&limit={limit}
    Returns: list of {"record": {...}, "score": float} dicts
    """

def delete_memory(self, record_id: str) -> Dict[str, Any]:
    """Delete a single memory record by UUID.
    DELETE /memory/{record_id}
    Returns: {"success": bool, "record_id": str}
    """

# query_memory signature extension (backwards-compatible, default False):
def query_memory(
    self, ..., include_expired: bool = False
) -> List[Dict[str, Any]]: ...
```

`async_client.py` mirrors all of the above with `async def` and `await self._client.<method>(...)`.

---

## Design Decision #5 — TypeScript SDK Types (No Ambiguity)

New interfaces to add to `types.ts`:

```typescript
// For POST /memory/link
export interface LinkMemoriesRequest {
  source_id: string;
  target_id: string;
  relation?: string;   // default: "related"
}
export interface LinkMemoriesResponse {
  success: boolean;
  source_id: string;
  target_id: string;
  relation: string;
}

// For GET /memory/neighbors/:id
export interface NeighborsResponse {
  records: MemoryRecord[];
  total: number;
}

// For GET /memory/search/related
export interface RelatedSearchParams {
  seed_id: string;
  limit?: number;
}
export interface RelatedSearchResponse {
  results: SearchResult[];   // reuses existing SearchResult {score, record}
}

// For DELETE /memory/:id
export interface DeleteMemoryResponse {
  success: boolean;
  record_id: string;
}
```

Extend `StatsResponse`:
```typescript
export interface StatsResponse {
  // ...existing fields...
  active_records: number;   // new — non-expired records count
}
```

Extend `QueryParams`:
```typescript
export interface QueryParams {
  // ...existing fields...
  include_expired?: boolean;
}
```

New methods on `HipCortexClient` in `client.ts`:
```typescript
async linkMemories(req: LinkMemoriesRequest): Promise<LinkMemoriesResponse>
async getNeighbors(recordId: string, limit?: number): Promise<NeighborsResponse>
async searchRelated(seedId: string, limit?: number): Promise<RelatedSearchResponse>
async deleteMemory(recordId: string): Promise<DeleteMemoryResponse>
```

---

## Build & Release Pipeline (No Ambiguity)

### Python SDK

```bash
cd sdk/python
# 1. Bump version in pyproject.toml: 0.3.0 → 0.4.0
# 2. Build sdist + wheel:
python -m build
# 3. Upload to PyPI (requires PYPI_API_TOKEN):
twine upload dist/hipcortex-0.4.0*
```

### TypeScript / npm SDK

```bash
cd sdk/typescript
# 1. Bump version in package.json: 0.3.0 → 0.4.0
# 2. Build compiled output:
npm run build
# 3. Publish (requires npm login):
npm publish
```

### VS Code Extension (VSIX)

```bash
cd vscode-extension
# 1. Bump version in package.json: 0.3.3 → 0.4.0
# 2. Webpack bundle:
npm run package
# 3. Package VSIX:
npx vsce package --no-dependencies
# Output: hipcortex-memory-0.4.0.vsix
# 4. Publish to marketplace:
npx vsce publish --no-dependencies
```

### MCP Server

The MCP server is bundled inside the `hipcortex` pip package (via `sdk/python/`). No separate publish step — it ships with the Python SDK v0.4.0.

---

## Verification Plan

| Step | Command | Pass Criterion |
|:---|:---|:---|
| Python unit tests | `cd sdk/python && python -m pytest tests/` | 0 failures |
| TS SDK build check | `cd sdk/typescript && npm run build` | 0 type errors |
| Extension compile check | `cd vscode-extension && npm run compile` | 0 TS errors |
| Smoke: link + neighbors | `curl -X POST :3030/memory/link && curl :3030/memory/neighbors/<id>` | HTTP 200 with valid JSON |
| Smoke: search/related | `curl ":3030/memory/search/related?seed_id=<id>"` | HTTP 200 with `results` array |
| Smoke: delete | `curl -X DELETE :3030/memory/<id>` | HTTP 200 `{"success":true}` |
| Smoke: include_expired | `curl ":3030/memory/query?include_expired=true"` | returns expired records |
