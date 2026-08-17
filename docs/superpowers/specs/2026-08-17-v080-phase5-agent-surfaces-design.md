# Phase 5: AgentSurfaces Design

**v0.8.0 sub-project. Builds on Phase 4 (ExperienceStore). Final phase.**

---

## Goal

Expose all Phase 1-4 operations through MCP tools, Python SDK (`HipCortexClient`), and TypeScript SDK — achieving exact schema parity across all three surfaces. After Phase 5, any agent (Claude Code, CrewAI, LangChain, Codex, VS Code extension) can use the full cognitive substrate without writing raw HTTP.

---

## Surface Parity Matrix

| Operation | REST (Phase) | MCP Tool | Python SDK | TypeScript SDK |
|-----------|-------------|----------|------------|----------------|
| Transact (AddMemory) | `POST /v1/cognitive/transact` (P1) | `cognitive_transact` | `client.transact(delta, actor)` | `client.transact(delta, actor)` |
| Cognitive diff | `GET /v1/cognitive/diff` (P1) | `cognitive_diff` | `client.cognitive_diff(from_tx, to_tx)` | `client.cognitiveDiff(fromTx, toTx)` |
| Self health | `GET /v1/self/health` (P1) | `self_health` | `client.self_health()` | `client.selfHealth()` |
| Create fork | `POST /v1/fork` (P2) | `fork_create` | `client.fork_create()` | `client.forkCreate()` |
| Step fork | `POST /v1/fork/{id}/step` (P2) | `fork_step` | `client.fork_step(id, action)` | `client.forkStep(id, action)` |
| Fork snapshot | `GET /v1/fork/{id}/snapshot` (P2) | `fork_snapshot` | `client.fork_snapshot(id, actor)` | `client.forkSnapshot(id, actor)` |
| Delete fork | `DELETE /v1/fork/{id}` (P2) | `fork_delete` | `client.fork_delete(id)` | `client.forkDelete(id)` |
| Rollout | `POST /v1/fork/{id}/rollout` (P3) | `fork_rollout` | `client.fork_rollout(id, actions, sigma2_max)` | `client.forkRollout(id, actions, sigma2Max)` |
| Consolidate | via `POST /v1/cognitive/transact` (P4) | `consolidate_memory` | `client.consolidate(source_ids, summary)` | `client.consolidate(sourceIds, summary)` |
| Forget actor | via `POST /v1/cognitive/transact` (P4) | `forget_actor` | `client.forget_actor(actor_id)` | `client.forgetActor(actorId)` |
| Archive record | via `POST /v1/cognitive/transact` (P4) | `archive_record` | `client.archive_record(id)` | `client.archiveRecord(id)` |
| Cognitive snapshot | `GET /v1/cognitive/snapshot` (P0) | `cognitive_snapshot` | `client.cognitive_snapshot(actor)` | `client.cognitiveSnapshot(actor)` |

---

## MCP Server Changes (`sdk/mcp/server.py`)

Add 11 new tools to the existing MCP server (which already has 18 tools). Each tool maps directly to a REST call.

**Tool schema pattern:**

```python
@server.tool("cognitive_transact")
async def cognitive_transact(delta: dict, actor: str) -> dict:
    """Apply a CognitiveDelta to the cognitive substrate."""
    resp = await _post("/v1/cognitive/transact", {"delta": delta, "actor": actor})
    return resp  # {"ok": bool, "tx_cursor": int}
```

**All 11 new tools follow this pattern.** No business logic in MCP layer — pure HTTP proxy to REST.

**MCP resources (existing 3 unchanged):**
- `hipcortex://context/relevant`
- `hipcortex://beliefs/current`
- `hipcortex://context/conversation`

**Version bump:** `serverInfo.version` → `"0.8.0"` (via `scripts/stamp_versions.py --mcp`).

---

## Python SDK Changes (`sdk/python/hipcortex/`)

### `client.py` — new methods on `HipCortexClient`

```python
def transact(self, delta: dict, actor: str) -> dict:
    """POST /v1/cognitive/transact"""
    return self._post("/v1/cognitive/transact", {"delta": delta, "actor": actor})

def cognitive_diff(self, from_tx: int, to_tx: int) -> dict:
    """GET /v1/cognitive/diff"""
    return self._get("/v1/cognitive/diff", params={"from_tx": from_tx, "to_tx": to_tx})

def self_health(self) -> dict:
    """GET /v1/self/health"""
    return self._get("/v1/self/health")

def cognitive_snapshot(self, actor: str = "") -> dict:
    return self._get("/v1/cognitive/snapshot", params={"actor": actor})

def fork_create(self) -> dict:
    return self._post("/v1/fork", {})

def fork_step(self, fork_id: str, action: str) -> dict:
    return self._post(f"/v1/fork/{fork_id}/step", {"action": action})

def fork_snapshot(self, fork_id: str, actor: str = "") -> dict:
    return self._get(f"/v1/fork/{fork_id}/snapshot", params={"actor": actor})

def fork_delete(self, fork_id: str) -> dict:
    return self._delete(f"/v1/fork/{fork_id}")

def fork_rollout(self, fork_id: str, actions: list[str], sigma2_max: float = 0.25) -> dict:
    return self._post(f"/v1/fork/{fork_id}/rollout",
                      {"actions": actions, "sigma2_max": sigma2_max})

def consolidate(self, source_ids: list[str], summary: dict) -> dict:
    delta = {"type": "Consolidate", "source_ids": source_ids, "summary": summary}
    return self.transact(delta, actor="sdk")

def forget_actor(self, actor_id: str) -> dict:
    return self.transact({"type": "ForgetActor", "actor": actor_id}, actor="sdk")

def archive_record(self, record_id: str) -> dict:
    return self.transact({"type": "ArchiveRecord", "id": record_id}, actor="sdk")
```

`_delete` method added to `HipCortexClient` base (currently only `_get`, `_post`).

**pyproject.toml:** version → `"0.8.0"`.

---

## TypeScript SDK Changes (`sdk/typescript/` — new directory)

Minimal TypeScript client. No runtime dependencies beyond `fetch`.

### `src/index.ts`

```typescript
export class HipCortexClient {
  constructor(private baseUrl: string, private apiKey?: string) {}

  private async _get(path: string, params?: Record<string, string>) { ... }
  private async _post(path: string, body: unknown) { ... }
  private async _delete(path: string) { ... }

  async transact(delta: CognitiveDelta, actor: string): Promise<TransactResponse> { ... }
  async cognitiveDiff(fromTx: number, toTx: number): Promise<TxStateDiff> { ... }
  async selfHealth(): Promise<SelfHealthResponse> { ... }
  async cognitiveSnapshot(actor?: string): Promise<CognitiveSnapshot> { ... }
  async forkCreate(): Promise<ForkCreateResponse> { ... }
  async forkStep(forkId: string, action: string): Promise<ForkStepResponse> { ... }
  async forkSnapshot(forkId: string, actor?: string): Promise<CognitiveSnapshot> { ... }
  async forkDelete(forkId: string): Promise<{ ok: boolean }> { ... }
  async forkRollout(forkId: string, actions: string[], sigma2Max?: number): Promise<RolloutResult> { ... }
  async consolidate(sourceIds: string[], summary: MemoryRecord): Promise<TransactResponse> { ... }
  async forgetActor(actorId: string): Promise<TransactResponse> { ... }
  async archiveRecord(id: string): Promise<TransactResponse> { ... }
}
```

### Type definitions (`src/types.ts`)

Mirror Rust structs: `CognitiveDelta`, `TransactResponse`, `TxStateDiff`, `SelfHealthResponse`, `CognitiveSnapshot`, `RolloutResult`, `ForkCreateResponse`, `ForkStepResponse`, `MemoryRecord`.

### `package.json`

```json
{
  "name": "@hipcortex/client",
  "version": "0.8.0",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": { "build": "tsc", "test": "vitest run" }
}
```

---

## VS Code Extension (`vscode-extension/src/extension.ts`)

Add `HipCortexAPI` methods for new endpoints (same pattern as existing `/memory/add`, `/memory/query`):
- `cognitiveTransact(delta, actor)`
- `selfHealth()`
- `cognitiveDiff(fromTx, toTx)`

These are called from new VS Code commands:
- `hipcortex.cognitiveHealth` — calls `/v1/self/health`, shows in status bar
- `hipcortex.cognitiveSnapshot` — calls `/v1/cognitive/snapshot`, shows in webview

**package.json** version → `"0.8.0"`.

---

## E2E Tests

### `tests/e2e_user_harness/suites/test_phase7_passive_layer.py` additions

Add surface parity conformance tests (no live server needed — mock server):
- Python SDK methods map to correct endpoints
- MCP tool schemas advertised correctly

### `tests/e2e_user_harness/suites/test_phase8_substrate.py` additions

G5-1..G5-5 (live server):
- Python SDK `fork_rollout` returns same schema as REST
- MCP `cognitive_transact` tool call succeeds
- TypeScript SDK `transact` → correct JSON body
- Version `0.8.0` on all three surfaces (REST `/health`, MCP `initialize`, Python `client.VERSION`)

---

## Files Changed

| File | Change |
|------|--------|
| `sdk/mcp/server.py` | Add 11 new tools; bump version `0.8.0` |
| `sdk/python/hipcortex/client.py` | Add 12 new methods + `_delete` |
| `sdk/python/pyproject.toml` | Version `0.8.0` |
| `sdk/typescript/` | New directory: `src/index.ts`, `src/types.ts`, `package.json`, `tsconfig.json` |
| `vscode-extension/src/extension.ts` | Add 3 API methods + 2 commands |
| `vscode-extension/package.json` | Version `0.8.0`; register 2 new commands |
| `scripts/stamp_versions.py` | Handle `--ts` flag for TS SDK version |
| `VERSION` | `0.8.0` |
| `tests/e2e_user_harness/suites/test_phase8_substrate.py` | G5-1..G5-5 |
| `tests/e2e_user_harness/suites/test_phase7_passive_layer.py` | Surface parity conformance |

---

## Acceptance Gates

| Gate | Test |
|------|------|
| G5-1 | Python `client.transact({"type":"AddMemory",...}, "actor")` → `ok=True`, `tx_cursor` int |
| G5-2 | MCP `cognitive_transact` tool exists in `tools/list`; call succeeds with valid delta |
| G5-3 | Python `client.fork_rollout(id, ["a","b"], 0.25)` → `steps` list |
| G5-4 | TS SDK `client.selfHealth()` → object with `healthy` boolean |
| G5-5 | `/health`, MCP `initialize`, Python `client.VERSION` all return `"0.8.0"` |

---

## Non-Goals (Phase 5)

- Auto-generated SDK from OpenAPI spec (manual for v0.8.0)
- SDK authentication beyond API key header
- Streaming / WebSocket surfaces
- gRPC SDK wrappers
- npm publish / PyPI publish (manual for v0.8.0)
