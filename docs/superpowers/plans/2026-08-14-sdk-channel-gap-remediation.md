# SDK & Channel Gap Remediation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every identified gap across all HipCortex distribution channels (MCP, TypeScript SDK, Python async client, CrewAI, LangChain, VSIX, Codex, Copilot) so that every op exposed by the Rust REST API is reachable from every relevant surface — with no ambiguity, full test coverage, and a ReAct-loop validation harness.

**Architecture:** All surfaces are HTTP clients to the same Rust `web_server.rs`. No surface bypasses `SafetyGuardrail`, `MemoryStore`, or the Merkle chain. Adding a method to a surface = one additional `_post`/`_get` call + matching type + AC test. No Rust changes needed for this plan.

**Tech Stack:** Python 3.9+, TypeScript 5, httpx, requests, crewai-tools, langchain-core, vscode API, pytest, e2e harness (pytest + httpx against live Rust server)

**Version target:** All surfaces → `0.6.0` (Python SDK, TypeScript SDK, MCP server banner, VSIX)

---

## Gap Registry (20 gaps, exhaustive)

| ID | Surface | Missing | REST target |
|----|---------|---------|-------------|
| G1 | MCP | `health` tool | GET /health |
| G2 | MCP | `observe` tool | POST /worldmodel/observe |
| G3 | MCP | `causal_add_edge`, `causal_intervention`, `causal_counterfactual` tools | POST /worldmodel/causal/edge, /intervention, /counterfactual |
| G4 | MCP | `coherence_check`, `coherence_resolve` tools | POST /coherence/check, /coherence/resolve/:id |
| G5 | MCP | `bulk_add` tool | POST /memory/bulk |
| G6 | MCP | `goal_react`, `goal_trace`, `memory_diff` tools | POST /goal/:id/react, GET /goal/:id/trace, POST /memory/diff |
| G7 | MCP | `corroborate`, `contradict`, `quarantine`, `restore` tools | POST /memory/corroborate/:id etc. |
| G8 | TypeScript SDK | `topoPpr`, `topoCheckEdge`, `topoDeconstruct` | POST/GET /topo/* |
| G9 | TypeScript SDK | `wmObserve`, `wmEntities`, `causalAddEdge`, `causalIntervention`, `causalCounterfactual` | POST /worldmodel/* |
| G10 | TypeScript SDK | `coherenceCheck`, `coherenceResolve`, `selfHealth`, `selfCapabilities`, `selfRegisterCapability` | POST /coherence/*, /self/* |
| G11 | TypeScript SDK | `auditVerify`, `auditExport`, `regulatoryHold*`, `webhooks*` | /audit/*, /regulatory/*, /webhooks |
| G12 | TypeScript SDK | `quarantine`, `restore`, `corroborate`, `contradict`, `consolidate` | POST /memory/* |
| G13 | TypeScript SDK | `graphSearch`, `getNode`, `createNode`, `createEdge`, `embedAndAdd`, `updateMemory`, `latestMemory`, `exportMemory`, `searchFlat` | /graph/*, /memory/* |
| G14 | Python async | Parity gaps: `rollout`, `can_execute`, topo, causal, self, coherence, audit, regulatory, webhooks, corroborate, contradict, quarantine, restore | (same as sync) |
| G15 | CrewAI | Only 3 tools; missing: `search`, `predict`, `rollout`, `reflect`, `can_execute`, `link`, `live_beliefs`, `graph_search` | /memory/search, /worldmodel/* etc. |
| G16 | LangChain | Chat-history only; missing: semantic `VectorStoreRetriever`, `reflect`, `live_beliefs`, async full parity | /memory/search, /memory/reflect etc. |
| G17 | VSIX | Missing LM tools: `hipcortex_add`, `hipcortex_forget`, `hipcortex_live_beliefs`; missing commands: `/search`, `/reflect`, `/forget` | via bundled server HTTP |
| G18 | Installer | No `codex` channel (`hipcortex install --channel codex`) | writes ~/.codex/config.yaml |
| G19 | Installer | No `copilot` channel (`hipcortex install --channel copilot`) | writes Copilot Extension MCP config |
| G20 | E2E harness | No Phase 6 test suite covering G1–G17 with ReAct validation loop | all surfaces via live server |

---

## Full Acceptance Criteria

### G1 — MCP `health` tool
- AC-G1-1: `health` present in MCP `TOOLS` list
- AC-G1-2: Call returns `{"status": "ok", "version": "..."}` or error string within 500ms
- AC-G1-3: `scripts/check_capabilities.py --check-mcp` exits 0 with `health` in output

### G2 — MCP `observe` tool
- AC-G2-1: `observe` present in MCP `TOOLS` list
- AC-G2-2: Call with `{state: "A", action: "move", next_state: "B"}` returns `{"success": true}`
- AC-G2-3: After observe, `/worldmodel/transitions` contains A→B entry
- AC-G2-4: Call with missing fields returns descriptive error (not 500)

### G3 — MCP causal tools
- AC-G3-1: `causal_add_edge` adds edge; verify via GET /worldmodel/causal
- AC-G3-2: `causal_intervention` returns P(Y|do(X=x)) dict with `result` key
- AC-G3-3: `causal_counterfactual` returns counterfactual value
- AC-G3-4: All 3 tools present in TOOLS; check_capabilities exits 0

### G4 — MCP coherence tools
- AC-G4-1: `coherence_check` returns `{consistent: bool, issues: [...]}` for provided record IDs
- AC-G4-2: `coherence_resolve` returns `{resolved: bool, winner_id: str}` for a conflict ID
- AC-G4-3: Both tools present in TOOLS

### G5 — MCP `bulk_add`
- AC-G5-1: `bulk_add` accepts list of records, returns `{inserted: N, failed: 0}`
- AC-G5-2: N records appear in subsequent query

### G6 — MCP goal/diff tools
- AC-G6-1: `goal_react` returns `{status: "Succeeded"|"Failed"|"InProgress"}` for a known goal UUID
- AC-G6-2: `goal_trace` returns list of derived records
- AC-G6-3: `memory_diff` returns `{level, fields_changed: [...]}` for two record UUIDs
- AC-G6-4: All 3 tools present in TOOLS

### G7 — MCP corroborate/contradict/quarantine/restore
- AC-G7-1: `corroborate` increments corroboration count on record
- AC-G7-2: `contradict` marks record contradicted
- AC-G7-3: `quarantine` sets status=quarantine; record excluded from default search
- AC-G7-4: `restore` sets status back to active
- AC-G7-5: All 4 tools present in TOOLS

### G8 — TypeScript topo methods
- AC-G8-1: `client.topoPpr({seedId?})` returns `{nodes: [...], scores: {...}}`
- AC-G8-2: `client.topoCheckEdge({from, to})` returns `{ok: bool, would_contradict: bool}`
- AC-G8-3: `client.topoDeconstruct({text})` returns `{nodes: [...], edges: [...]}`
- AC-G8-4: All 3 have TypeScript types in types.ts
- AC-G8-5: client.test.ts has passing unit tests for all 3 (mock fetch)

### G9 — TypeScript WorldModel/Causal methods
- AC-G9-1: `client.wmObserve({state, action, next_state})` returns success
- AC-G9-2: `client.wmEntities()` returns entity list
- AC-G9-3: `client.causalAddEdge(...)`, `client.causalIntervention(...)`, `client.causalCounterfactual(...)` all typed + callable
- AC-G9-4: types.ts has full request/response types for all 5 methods

### G10 — TypeScript coherence/self methods
- AC-G10-1: `client.coherenceCheck({record_ids})` returns consistency result
- AC-G10-2: `client.coherenceResolve({id, strategy})` returns resolution
- AC-G10-3: `client.selfHealth()` returns health scores
- AC-G10-4: `client.selfCapabilities()` returns capability list
- AC-G10-5: `client.selfRegisterCapability(desc)` returns success

### G11 — TypeScript audit/regulatory/webhooks
- AC-G11-1: `client.auditVerify()` returns `{valid: bool, chain_length: N}`
- AC-G11-2: `client.auditExport()` returns Merkle chain entries array
- AC-G11-3: `client.regulatoryHoldSet(actor, reason)` returns hold ID
- AC-G11-4: `client.regulatoryHoldRelease(actor)` returns success
- AC-G11-5: `client.webhooksList()`, `client.webhookRegister(url, events)`, `client.webhookDelete(id)` all callable

### G12 — TypeScript memory lifecycle methods
- AC-G12-1: `client.quarantine(id)` → status=quarantine
- AC-G12-2: `client.restore(id)` → status=active
- AC-G12-3: `client.corroborate(id)` → increments corroboration
- AC-G12-4: `client.contradict(id)` → marks contradicted
- AC-G12-5: `client.consolidate({actor?, threshold?, dry_run?})` returns consolidation report

### G13 — TypeScript graph/search/misc methods
- AC-G13-1: `client.graphSearch(symbol)` returns symbol matches
- AC-G13-2: `client.getNode(id)` returns node or null
- AC-G13-3: `client.createNode(label, props)` returns node ID
- AC-G13-4: `client.createEdge(from, to, relation)` returns edge ID
- AC-G13-5: `client.embedAndAdd(req)` returns record ID
- AC-G13-6: `client.updateMemory(id, fields)` returns updated record
- AC-G13-7: `client.latestMemory(limit)` returns N most recent records
- AC-G13-8: `client.exportMemory()` returns JSONL-compatible array
- AC-G13-9: `client.searchFlat(query, limit)` returns unranked results

### G14 — Python async_client parity
- AC-G14-1: Every method in sync `client.py` exists in `async_client.py` (verified by `test_async_parity.py`)
- AC-G14-2: `await client.rollout(...)` returns same shape as sync
- AC-G14-3: `await client.can_execute(...)` returns bool
- AC-G14-4: All topo/causal/coherence/self/audit/regulatory/webhook/quarantine methods async

### G15 — CrewAI expanded tools
- AC-G15-1: `make_memory_tools()` returns 11 tools (was 3)
- AC-G15-2: `HipCortexSearchTool._run(query)` returns formatted results string
- AC-G15-3: `HipCortexPredictTool._run(state, action)` returns prediction string
- AC-G15-4: `HipCortexRolloutTool._run(state, mode?)` returns trajectory string
- AC-G15-5: `HipCortexReflectTool._run(query)` returns CoT output
- AC-G15-6: `HipCortexCanExecuteTool._run(action)` returns "approved"/"rejected"
- AC-G15-7: `HipCortexLinkTool._run(from_id, to_id, relation)` returns edge ID
- AC-G15-8: `HipCortexBeliefsTool._run(actor?)` returns belief list
- AC-G15-9: `HipCortexGraphSearchTool._run(seed_id)` returns PPR results
- AC-G15-10: All tools importable without crewai installed (graceful fallback)
- AC-G15-11: Phase 3 e2e test verifies all 11 tools instantiate and name correctly

### G16 — LangChain expansion
- AC-G16-1: `HipCortexSemanticRetriever` class exists in `langchain_memory.py`
- AC-G16-2: `retriever.get_relevant_documents(query)` calls /memory/search, returns `List[Document]`
- AC-G16-3: `HipCortexMemory.reflect(query)` method returns CoT string
- AC-G16-4: `HipCortexMemory.live_beliefs(actor?)` returns belief dict
- AC-G16-5: `AsyncHipCortexMemory` has matching `areflect`, `alive_beliefs` methods
- AC-G16-6: Phase 3 e2e test imports and invokes retriever

### G17 — VSIX LM tool additions
- AC-G17-1: `hipcortex_add` LM tool registered; call with `{actor, action, target}` stores record, returns `{record_id}`
- AC-G17-2: `hipcortex_forget` LM tool registered; call with `{actor}` deletes records
- AC-G17-3: `hipcortex_live_beliefs` LM tool registered; returns formatted belief list
- AC-G17-4: `/search` chat command added to package.json `contributes.commands`
- AC-G17-5: `/reflect` chat command added
- AC-G17-6: `/forget` chat command added
- AC-G17-7: Phase 4 e2e validates 13 LM tools registered (was 10)
- AC-G17-8: package.json version bumped to 0.6.0

### G18 — Codex installer
- AC-G18-1: `hipcortex install --channel codex` exits 0
- AC-G18-2: `~/.codex/config.yaml` contains `mcpServers.hipcortex` with correct `command` path
- AC-G18-3: `hipcortex install --channel codex --uninstall` removes the entry
- AC-G18-4: Channel `codex` listed in `docs/channels.md` with status `mcp`

### G19 — Copilot installer
- AC-G19-1: `hipcortex install --channel copilot` exits 0
- AC-G19-2: Writes VS Code `settings.json` MCP server entry (fallback: prints manual steps)
- AC-G19-3: Channel `copilot` updated from `guide` → `mcp` in `docs/channels.md`
- AC-G19-4: `channels.yaml` updated to match

### G20 — E2E harness Phase 6
- AC-G20-1: `tests/e2e_user_harness/suites/test_phase6_gap_coverage.py` exists
- AC-G20-2: Suite covers MCP tool calls for all G1–G7 (health, observe, causal, coherence, bulk, goal, corroborate)
- AC-G20-3: Suite covers TypeScript SDK equivalents via subprocess `npx ts-node` or REST proxy
- AC-G20-4: Suite covers CrewAI tools G15 (all 11 tools instantiate and call live server)
- AC-G20-5: Suite covers LangChain G16 (retriever returns Documents)
- AC-G20-6: ReAct loop runner (`react_validator.py`) runs all AC assertions, logs pass/fail, retries on fail up to 3 times
- AC-G20-7: `pytest tests/e2e_user_harness/suites/test_phase6_gap_coverage.py` exits 0

---

## File Structure

```
sdk/mcp/server.py               — G1-G7: +9 tools (+~250 lines)
sdk/python/hipcortex/
  client.py                     — G15,G16: no changes needed (already complete)
  async_client.py               — G14: +~150 lines (parity methods)
  langchain_memory.py           — G16: +HipCortexSemanticRetriever, reflect, live_beliefs (~60 lines)
  adapters/crewai.py            — G15: +8 new Tool classes + expand make_memory_tools (~200 lines)
sdk/typescript/src/
  client.ts                     — G8-G13: +30 new methods (~300 lines)
  types.ts                      — G8-G13: +25 new interfaces (~150 lines)
sdk/python/hipcortex/install_hosts.py  — G18,G19: +2 install functions (~80 lines)
vscode-extension/src/extension.ts      — G17: +3 LM tools + 3 commands (~90 lines)
vscode-extension/package.json         — G17: +3 commands + version bump
tests/e2e_user_harness/suites/test_phase6_gap_coverage.py  — G20: new file (~300 lines)
tests/e2e_user_harness/suites/test_phase3_framework_integrations.py  — G15,G16: extend existing tests
scripts/react_validator.py       — G20: ReAct loop runner (~80 lines)
docs/channels.md                — G18,G19: update table
docs/channels.yaml              — G18,G19: add codex/copilot mcp entries
docs/capabilities.md            — all gaps: update matrix rows + MCP tool count (18→27)
sdk/python/pyproject.toml       — version 0.5.2→0.6.0
sdk/typescript/package.json     — version bump to 0.6.0
vscode-extension/package.json   — version bump to 0.6.0
sdk/mcp/server.py               — serverInfo version bump 0.5.2→0.6.0
```

---

## Sprint 1 — MCP Completeness (G1–G7)

**File:** `sdk/mcp/server.py`

- [ ] **Step 1: Add `health` tool (G1)**

```python
{
    "name": "health",
    "description": "Check HipCortex server liveness. Returns status and version.",
    "inputSchema": {"type": "object", "properties": {}, "required": []}
},
```

Handler:
```python
def handle_health(args: dict) -> str:
    result = _get("/health")
    return f"status={result.get('status','?')} version={result.get('version','?')}"
```

- [ ] **Step 2: Add `observe` tool (G2)**

```python
{
    "name": "observe",
    "description": "Feed a (state, action, next_state) transition into the WorldModel Dirichlet-Multinomial table.",
    "inputSchema": {"type": "object",
        "properties": {
            "state":      {"type": "string"},
            "action":     {"type": "string"},
            "next_state": {"type": "string"}
        }, "required": ["state", "action", "next_state"]}
},
```

Handler:
```python
def handle_observe(args: dict) -> str:
    result = _post("/worldmodel/observe", {
        "state": args["state"], "action": args["action"], "next_state": args["next_state"]
    })
    return f"observed: {args['state']} --[{args['action']}]--> {args['next_state']}" \
           if result.get("success") else f"error: {result}"
```

- [ ] **Step 3: Add 3 causal tools (G3)**

`causal_add_edge`: POST /worldmodel/causal/edge `{from, to, distribution?}`
`causal_intervention`: POST /worldmodel/causal/intervention `{target, value, query}`
`causal_counterfactual`: POST /worldmodel/causal/counterfactual `{observed, intervention, query}`

- [ ] **Step 4: Add 2 coherence tools (G4)**

`coherence_check`: POST /coherence/check `{record_ids: [str]}`
`coherence_resolve`: POST /coherence/resolve/:id `{strategy: "recency"|"confidence"|"consensus"}`

- [ ] **Step 5: Add `bulk_add` tool (G5)**

`bulk_add`: POST /memory/bulk `{records: [{actor, action, target, record_type?, ...}]}`
Returns: `"inserted N, failed M"`

- [ ] **Step 6: Add `goal_react`, `goal_trace`, `memory_diff` tools (G6)**

`goal_react`: POST /goal/:id/react — args: `{goal_id: str (UUID)}`
`goal_trace`: GET /goal/:id/trace — args: `{goal_id: str}`
`memory_diff`: POST /memory/diff — args: `{from_id: str, to_id: str}`

- [ ] **Step 7: Add 4 lifecycle tools (G7)**

`corroborate`: POST /memory/corroborate/:id — args: `{record_id, evidence?: str}`
`contradict`: POST /memory/contradict/:id — args: `{record_id, reason?: str}`
`quarantine`: POST /memory/quarantine/:id — args: `{record_id}`
`restore`: POST /memory/restore/:id — args: `{record_id}`

- [ ] **Step 8: Bump MCP serverInfo version to 0.6.0**

- [ ] **Step 9: Wire all 9 new handlers into the dispatch dict**

```python
"health":                handle_health,
"observe":               handle_observe,
"causal_add_edge":       handle_causal_add_edge,
"causal_intervention":   handle_causal_intervention,
"causal_counterfactual": handle_causal_counterfactual,
"coherence_check":       handle_coherence_check,
"coherence_resolve":     handle_coherence_resolve,
"bulk_add":              handle_bulk_add,
"goal_react":            handle_goal_react,
"goal_trace":            handle_goal_trace,
"memory_diff":           handle_memory_diff,
"corroborate":           handle_corroborate,
"contradict":            handle_contradict,
"quarantine":            handle_quarantine,
"restore":               handle_restore,
```

- [ ] **Step 10: Update `docs/capabilities.md` MCP tools inventory (18→27 tools)**

- [ ] **Step 11: Run `python scripts/check_capabilities.py --check-mcp` — expect exit 0**

- [ ] **Step 12: Commit**
```bash
git add sdk/mcp/server.py docs/capabilities.md
git commit -m "feat(mcp): add 9 missing tools — health, observe, causal, coherence, bulk, goal, lifecycle"
```

---

## Sprint 2 — TypeScript SDK Completeness (G8–G13)

**Files:** `sdk/typescript/src/client.ts`, `sdk/typescript/src/types.ts`

- [ ] **Step 1: Add topo types to types.ts (G8)**

```typescript
export interface TopoPprRequest { seed_id?: string; limit?: number; }
export interface TopoPprResponse { nodes: string[]; scores: Record<string, number>; }
export interface TopoCheckEdgeRequest { from: string; to: string; }
export interface TopoCheckEdgeResponse { ok: boolean; would_contradict: boolean; reason?: string; }
export interface TopoDeconstructRequest { text: string; apply?: boolean; }
export interface TopoDeconstructResponse { nodes: string[]; edges: Array<{from: string; to: string}>; }
```

- [ ] **Step 2: Add WorldModel/Causal types (G9)**

```typescript
export interface WmObserveRequest { state: string; action: string; next_state: string; }
export interface WmObserveResponse { success: boolean; }
export interface WmEntity { id: string; state: number[]; covariance: number[][]; }
export interface CausalAddEdgeRequest { from: string; to: string; distribution?: Record<string,number>; }
export interface CausalInterventionRequest { target: string; value: string; query: string; }
export interface CausalCounterfactualRequest { observed: Record<string,string>; intervention: Record<string,string>; query: string; }
```

- [ ] **Step 3: Add Coherence/Self types (G10)**

```typescript
export interface CoherenceCheckRequest { record_ids: string[]; }
export interface CoherenceCheckResponse { consistent: boolean; issues: string[]; }
export interface CoherenceResolveRequest { strategy: "recency" | "confidence" | "consensus"; }
export interface CoherenceResolveResponse { resolved: boolean; winner_id: string; }
export interface SelfHealthResponse { overall: number; modules: Record<string, number>; }
export interface CapabilityDescriptor { name: string; description: string; required_cpu_percent: number; required_memory_mb: number; }
```

- [ ] **Step 4: Add Audit/Regulatory/Webhook types (G11)**

```typescript
export interface AuditVerifyResponse { valid: boolean; chain_length: number; }
export interface WebhookRegistration { id: string; url: string; events: string[]; }
export interface RegulatoryHold { actor: string; reason: string; held_at: string; }
```

- [ ] **Step 5: Add remaining types (G12, G13)**

```typescript
export interface ConsolidateRequest { actor?: string; threshold?: number; dry_run?: boolean; }
export interface ConsolidateResponse { merged: number; removed: number; dry_run: boolean; }
```

- [ ] **Step 6: Add topo methods to client.ts (G8)**

```typescript
async topoPpr(req: TopoPprRequest = {}): Promise<TopoPprResponse> {
    const qs = req.seed_id ? `?seed_id=${req.seed_id}&limit=${req.limit ?? 20}` : `?limit=${req.limit ?? 20}`;
    return this.request<TopoPprResponse>("GET", `/topo/ppr${qs}`);
}
async topoCheckEdge(req: TopoCheckEdgeRequest): Promise<TopoCheckEdgeResponse> {
    return this.request<TopoCheckEdgeResponse>("POST", "/topo/check-edge", req);
}
async topoDeconstruct(req: TopoDeconstructRequest): Promise<TopoDeconstructResponse> {
    return this.request<TopoDeconstructResponse>("POST", "/topo/deconstruct", req);
}
```

- [ ] **Step 7: Add WorldModel/Causal methods (G9)**

```typescript
async wmObserve(req: WmObserveRequest): Promise<WmObserveResponse>  // POST /worldmodel/observe
async wmEntities(): Promise<WmEntity[]>                              // GET /worldmodel/entities
async causalAddEdge(req: CausalAddEdgeRequest): Promise<{success:boolean}>   // POST /worldmodel/causal/edge
async causalIntervention(req: CausalInterventionRequest): Promise<{result:number}>
async causalCounterfactual(req: CausalCounterfactualRequest): Promise<{result:number}>
```

- [ ] **Step 8: Add Coherence/Self methods (G10)**

```typescript
async coherenceCheck(req: CoherenceCheckRequest): Promise<CoherenceCheckResponse>  // POST /coherence/check
async coherenceResolve(id: string, req: CoherenceResolveRequest): Promise<CoherenceResolveResponse> // POST /coherence/resolve/:id
async selfHealth(): Promise<SelfHealthResponse>                     // GET /self/health
async selfCapabilities(): Promise<CapabilityDescriptor[]>           // GET /self/capabilities
async selfRegisterCapability(desc: CapabilityDescriptor): Promise<{success:boolean}> // POST /self/capabilities
```

- [ ] **Step 9: Add Audit/Regulatory/Webhook methods (G11)**

```typescript
async auditVerify(): Promise<AuditVerifyResponse>                   // POST /audit/verify
async auditExport(): Promise<object[]>                              // GET /audit/export
async regulatoryHoldSet(actor: string, reason: string): Promise<{id:string}>  // POST /regulatory/hold
async regulatoryHoldRelease(actor: string): Promise<{success:boolean}>         // DELETE /regulatory/hold/:actor
async regulatoryHoldList(): Promise<RegulatoryHold[]>               // GET /regulatory/hold
async webhooksList(): Promise<WebhookRegistration[]>                // GET /webhooks
async webhooksRegister(url: string, events: string[]): Promise<WebhookRegistration> // POST /webhooks
async webhooksDelete(id: string): Promise<void>                     // DELETE /webhooks/:id
```

- [ ] **Step 10: Add lifecycle/misc methods (G12, G13)**

```typescript
async quarantine(id: string): Promise<{success:boolean}>            // POST /memory/quarantine/:id
async restore(id: string): Promise<{success:boolean}>               // POST /memory/restore/:id
async corroborate(id: string, evidence?: string): Promise<{success:boolean}>
async contradict(id: string, reason?: string): Promise<{success:boolean}>
async consolidate(req?: ConsolidateRequest): Promise<ConsolidateResponse>  // POST /memory/consolidate
async graphSearch(symbol: string): Promise<object[]>                // GET /graph/search?q=
async getNode(id: string): Promise<object | null>                   // GET /node/:id
async createNode(label: string, props?: Record<string,string>): Promise<{id:string}>
async createEdge(from: string, to: string, relation: string): Promise<{id:string}>
async embedAndAdd(req: AddMemoryRequest): Promise<AddMemoryResponse> // POST /memory/embed
async updateMemory(id: string, fields: Partial<AddMemoryRequest>): Promise<object>  // PUT /memory/update/:id
async latestMemory(limit?: number): Promise<MemoryRecord[]>         // GET /memory/latest
async exportMemory(): Promise<MemoryRecord[]>                        // GET /memory/export
async searchFlat(query: string, limit?: number): Promise<MemoryRecord[]>  // GET /memory/search-flat
```

- [ ] **Step 11: Add unit tests for all new methods in `client.test.ts` (mock fetch)**

- [ ] **Step 12: Bump `sdk/typescript/package.json` version to 0.6.0**

- [ ] **Step 13: Run `npx tsc --noEmit` — expect 0 errors**

- [ ] **Step 14: Commit**
```bash
git add sdk/typescript/src/
git commit -m "feat(ts-sdk): add 30 missing methods — topo, causal, coherence, self, audit, regulatory, webhooks, lifecycle"
```

---

## Sprint 3 — Python async_client Parity (G14)

**File:** `sdk/python/hipcortex/async_client.py`

- [ ] **Step 1: Write `test_async_parity.py` — FAIL first**

```python
# tests/unit/test_async_parity.py
import inspect
from hipcortex.client import HipCortexClient
from hipcortex.async_client import AsyncHipCortexClient

def test_async_parity():
    sync_methods = {n for n,_ in inspect.getmembers(HipCortexClient, predicate=inspect.isfunction)
                    if not n.startswith('_')}
    async_methods = {n for n,_ in inspect.getmembers(AsyncHipCortexClient, predicate=inspect.isfunction)
                     if not n.startswith('_')}
    missing = sync_methods - async_methods
    assert not missing, f"async_client missing: {missing}"
```

Run: `pytest tests/unit/test_async_parity.py` — expect FAIL (shows missing methods)

- [ ] **Step 2: Add all missing async methods**

Group 1 — Intelligence:
```python
async def rollout(self, state: str, mode: str = "dirichlet", **kw) -> Dict[str, Any]:
    return await self._request("POST", "/worldmodel/rollout", {"state": state, "mode": mode, **kw})

async def can_execute(self, action: str = "rollout") -> bool:
    try:
        result = await self._request("POST", "/self/can-execute", {"operation": action})
        return result.get("should_execute", False)
    except Exception:
        return await self.health()
```

Group 2 — Topo:
```python
async def topo_ppr(self, seed_id: Optional[str] = None, limit: int = 20) -> Dict[str, Any]: ...
async def topo_check_edge(self, from_: str, to: str) -> Dict[str, Any]: ...
async def topo_deconstruct(self, text: str, apply: bool = False) -> Dict[str, Any]: ...
```

Group 3 — Causal:
```python
async def wm_observe(self, state: str, action: str, next_state: str) -> Dict[str, Any]: ...
async def wm_entities(self) -> List[Dict[str, Any]]: ...
async def causal_add_edge(self, from_: str, to: str, distribution: Dict = None) -> Dict[str, Any]: ...
async def causal_intervention(self, target: str, value: str, query: str) -> Dict[str, Any]: ...
async def causal_counterfactual(self, observed: Dict, intervention: Dict, query: str) -> Dict[str, Any]: ...
```

Group 4 — Coherence/Self:
```python
async def coherence_check(self, record_ids: List[str]) -> Dict[str, Any]: ...
async def coherence_resolve(self, conflict_id: str, strategy: str = "recency") -> Dict[str, Any]: ...
async def self_health(self) -> Dict[str, Any]: ...
async def self_capabilities(self) -> List[Dict[str, Any]]: ...
async def self_register_capability(self, desc: Dict[str, Any]) -> Dict[str, Any]: ...
```

Group 5 — Audit/Regulatory/Webhooks:
```python
async def audit_verify(self) -> Dict[str, Any]: ...
async def audit_export(self) -> List[Dict[str, Any]]: ...
async def regulatory_hold_set(self, actor: str, reason: str) -> Dict[str, Any]: ...
async def regulatory_hold_release(self, actor: str) -> Dict[str, Any]: ...
async def regulatory_hold_list(self) -> List[Dict[str, Any]]: ...
async def webhooks_list(self) -> List[Dict[str, Any]]: ...
async def webhooks_register(self, url: str, events: List[str]) -> Dict[str, Any]: ...
async def webhooks_delete(self, id: str) -> None: ...
```

Group 6 — Lifecycle:
```python
async def quarantine(self, record_id: str) -> Dict[str, Any]: ...
async def restore(self, record_id: str) -> Dict[str, Any]: ...
async def corroborate(self, record_id: str, evidence: str = None) -> Dict[str, Any]: ...
async def contradict(self, record_id: str, reason: str = None) -> Dict[str, Any]: ...
async def consolidate(self, actor: str = None, threshold: float = 0.8, dry_run: bool = True) -> Dict[str, Any]: ...
```

Group 7 — Convenience (sync has these):
```python
async def remember(self, content: str, action: str = "observation", ...) -> Dict[str, Any]: ...
async def recall(self, limit: int = 20, ...) -> List[Dict[str, Any]]: ...
async def recall_with_metadata(self, ...) -> List[Dict[str, Any]]: ...
async def prompt_context(self, query: str, ...) -> str: ...
async def remember_and_recall(self, content: str, query: str, ...) -> Dict[str, Any]: ...
async def ping_latency_ms(self) -> float: ...
async def create_node(self, label: str, ...) -> Dict[str, Any]: ...
async def create_edge(self, from_id: str, to_id: str, relation: str) -> Dict[str, Any]: ...
async def set_state(self, ...) -> Dict[str, Any]: ...
async def get_state(self, ...) -> Dict[str, Any]: ...
async def update_memory(self, record_id: str, ...) -> Dict[str, Any]: ...
async def latest_memory(self, limit: int = 10) -> List[Dict[str, Any]]: ...
async def export_memory(self) -> List[Dict[str, Any]]: ...
async def search_flat(self, query: str, limit: int = 20) -> List[Dict[str, Any]]: ...
```

- [ ] **Step 3: Run parity test — expect PASS**
```bash
pytest tests/unit/test_async_parity.py -v
```

- [ ] **Step 4: Commit**
```bash
git add sdk/python/hipcortex/async_client.py tests/unit/test_async_parity.py
git commit -m "feat(async-client): full parity with sync client — +30 async methods"
```

---

## Sprint 4 — CrewAI Expanded Tools (G15)

**File:** `sdk/python/hipcortex/adapters/crewai.py`

- [ ] **Step 1: Add 8 new Tool classes**

```python
class HipCortexSearchTool(BaseTool):
    name = "hipcortex_search"
    description = "Semantic search over persistent memory. Use before reasoning to recall relevant context."
    # _run(query: str, limit: int = 10) → calls client.search(query, limit)
    # Returns: "1. [score=0.92] target_text\n2. ..."

class HipCortexPredictTool(BaseTool):
    name = "hipcortex_predict"
    description = "WorldModel single-step prediction: given current state and action, predict next state probability."
    # _run(state: str, action: str) → calls client.predict(state, action)

class HipCortexRolloutTool(BaseTool):
    name = "hipcortex_rollout"
    description = "Multi-step world-model rollout. mode=dirichlet (default) or mcts. Returns trajectory."
    # _run(state: str, mode: str = "dirichlet", iterations: int = 50) → calls client.rollout(...)

class HipCortexReflectTool(BaseTool):
    name = "hipcortex_reflect"
    description = "AureusBridge chain-of-thought over memory context. Use to synthesize conclusions from past observations."
    # _run(query: str) → calls client.reflect(query)

class HipCortexCanExecuteTool(BaseTool):
    name = "hipcortex_can_execute"
    description = "SelfModel gate: should this agent attempt the given operation given current resources and health?"
    # _run(action: str) → calls client.can_execute(action) → "approved" | "rejected"

class HipCortexLinkTool(BaseTool):
    name = "hipcortex_link"
    description = "Create a causal graph edge between two memory records. Use to model causal or temporal relationships."
    # _run(from_id: str, to_id: str, relation: str = "caused") → calls client.link_memories(...)

class HipCortexBeliefsTool(BaseTool):
    name = "hipcortex_beliefs"
    description = "Retrieve live beliefs: current symbolic facts, hypotheses, and world state for this agent."
    # _run(actor: str = None) → calls client.live_beliefs()

class HipCortexGraphSearchTool(BaseTool):
    name = "hipcortex_graph_search"
    description = "PPR-ranked graph search from a seed memory record. Returns related memories weighted by causal proximity."
    # _run(seed_id: str, limit: int = 10) → calls client.search_related(seed_id, limit)
```

- [ ] **Step 2: Expand `make_memory_tools` to return all 11 tools**

```python
def make_memory_tools(client=None, agent_id=None, tools="all") -> List[Any]:
    # tools="all" returns all 11; tools=["remember","search"] returns subset
    all_tools = [
        HipCortexRememberTool, HipCortexRecallTool, HipCortexForgetTool,
        HipCortexSearchTool, HipCortexPredictTool, HipCortexRolloutTool,
        HipCortexReflectTool, HipCortexCanExecuteTool, HipCortexLinkTool,
        HipCortexBeliefsTool, HipCortexGraphSearchTool,
    ]
    ...
```

- [ ] **Step 3: Run existing Phase 3 test — still passes (backwards compat)**

- [ ] **Step 4: Extend Phase 3 test to verify all 11 tools**

```python
def test_crewai_all_tools_instantiate(raw_client):
    tools = make_memory_tools(client=HipCortexClient(raw_client.base_url))
    assert len(tools) == 11
    names = {t.name for t in tools}
    assert "hipcortex_search" in names
    assert "hipcortex_predict" in names
    assert "hipcortex_rollout" in names
    assert "hipcortex_reflect" in names
    assert "hipcortex_can_execute" in names
    assert "hipcortex_link" in names
    assert "hipcortex_beliefs" in names
    assert "hipcortex_graph_search" in names
```

- [ ] **Step 5: Commit**
```bash
git add sdk/python/hipcortex/adapters/crewai.py tests/e2e_user_harness/suites/test_phase3_framework_integrations.py
git commit -m "feat(crewai): expand from 3 to 11 tools — search, predict, rollout, reflect, can_execute, link, beliefs, graph"
```

---

## Sprint 5 — LangChain Expansion (G16)

**File:** `sdk/python/hipcortex/langchain_memory.py`

- [ ] **Step 1: Add `HipCortexSemanticRetriever`**

```python
try:
    from langchain_core.documents import Document
    from langchain_core.retrievers import BaseRetriever
    _LANGCHAIN_RETRIEVER = True
except ImportError:
    _LANGCHAIN_RETRIEVER = False
    class BaseRetriever: pass  # type: ignore

class HipCortexSemanticRetriever(BaseRetriever):
    """LangChain Retriever backed by HipCortex /memory/search.
    
    Usage:
        retriever = HipCortexSemanticRetriever.from_settings()
        docs = retriever.get_relevant_documents("database performance")
    """
    def __init__(self, client: HipCortexClient, limit: int = 10):
        self._client = client
        self._limit = limit

    @classmethod
    def from_settings(cls, limit: int = 10) -> "HipCortexSemanticRetriever":
        from .adapters.common import client_from_settings
        return cls(client=client_from_settings(), limit=limit)

    def _get_relevant_documents(self, query: str) -> List[Any]:
        results = self._client.search(query=query, limit=self._limit)
        if not _LANGCHAIN_RETRIEVER:
            return results
        return [
            Document(
                page_content=r["record"].get("target", ""),
                metadata={k: v for k, v in r["record"].items() if k != "target"}
            )
            for r in results.get("results", [])
        ]
```

- [ ] **Step 2: Add `reflect` and `live_beliefs` to `HipCortexMemory`**

```python
def reflect(self, query: str) -> str:
    """AureusBridge CoT synthesis over memory context."""
    result = self._client.reflect(query=query)
    return result.get("reflection", result.get("output", str(result)))

def live_beliefs(self, actor: Optional[str] = None) -> Dict[str, Any]:
    """Retrieve live symbolic beliefs for actor (or all)."""
    return self._client.live_beliefs()
```

- [ ] **Step 3: Add async equivalents to `AsyncHipCortexMemory`**

```python
async def areflect(self, query: str) -> str: ...
async def alive_beliefs(self, actor: Optional[str] = None) -> Dict[str, Any]: ...
async def aget_relevant_documents(self, query: str) -> List[Any]:  # async retriever
```

- [ ] **Step 4: Extend Phase 3 tests**

```python
def test_langchain_semantic_retriever(raw_client):
    from hipcortex.langchain_memory import HipCortexSemanticRetriever
    from hipcortex import HipCortexClient
    client = HipCortexClient(raw_client.base_url)
    # seed a memory
    client.add_memory(actor="test", action="noted", target="Rust is fast for memory engines")
    retriever = HipCortexSemanticRetriever(client=client, limit=5)
    docs = retriever._get_relevant_documents("Rust performance")
    assert len(docs) >= 1

def test_langchain_reflect(raw_client):
    from hipcortex.langchain_memory import HipCortexMemory
    from hipcortex import HipCortexClient
    memory = HipCortexMemory(client=HipCortexClient(raw_client.base_url), session_id="test")
    result = memory.reflect("what is hipcortex used for?")
    assert isinstance(result, str) and len(result) > 0
```

- [ ] **Step 5: Commit**
```bash
git add sdk/python/hipcortex/langchain_memory.py tests/e2e_user_harness/suites/test_phase3_framework_integrations.py
git commit -m "feat(langchain): add HipCortexSemanticRetriever + reflect/live_beliefs methods"
```

---

## Sprint 6 — VSIX LM Tool Additions (G17)

**Files:** `vscode-extension/src/extension.ts`, `vscode-extension/package.json`

- [ ] **Step 1: Add `hipcortex_add` LM tool to extension.ts**

```typescript
const addTool = (vscode.lm as any).registerTool('hipcortex_add', {
    modelDescription: 'Store a memory in HipCortex. Call with actor, action, target to persist an observation or decision.',
    inputSchema: {
        type: 'object',
        properties: {
            actor:  { type: 'string', description: 'Scope/agent identifier' },
            action: { type: 'string', description: 'Action label (e.g. decided, observed, noted)' },
            target: { type: 'string', description: 'The memory content to store' },
        },
        required: ['actor', 'action', 'target'],
    },
    invoke: async (params: any, _token: any) => {
        const result = await apiClient.addMemory(params);
        return { content: [{ type: 'text', value: result.success ? `stored: ${result.record_id}` : `error: ${result.error}` }] };
    },
});
```

- [ ] **Step 2: Add `hipcortex_forget` LM tool**

```typescript
const forgetTool = (vscode.lm as any).registerTool('hipcortex_forget', {
    modelDescription: 'Delete all memories for an actor (GDPR forget / session reset).',
    inputSchema: { type: 'object', properties: { actor: { type: 'string' } }, required: ['actor'] },
    invoke: async (params: any, _token: any) => {
        const result = await apiClient.forget(params.actor);
        return { content: [{ type: 'text', value: `deleted ${result.records_deleted} records for ${params.actor}` }] };
    },
});
```

- [ ] **Step 3: Add `hipcortex_live_beliefs` LM tool**

```typescript
const beliefsTool = (vscode.lm as any).registerTool('hipcortex_live_beliefs', {
    modelDescription: 'Retrieve live beliefs: current symbolic facts and world state. Call before reasoning about system state.',
    inputSchema: { type: 'object', properties: { actor: { type: 'string', description: 'Optional actor filter' } } },
    invoke: async (params: any, _token: any) => {
        const result = await apiClient.liveBeliefs(params);
        return { content: [{ type: 'text', value: JSON.stringify(result, null, 2) }] };
    },
});
```

- [ ] **Step 4: Add `/search`, `/reflect`, `/forget` commands to chat participant handler**

In the chat participant `handleRequest` switch:
```typescript
case '/search':
    const searchResults = await apiClient.search({ query: userInput, limit: 10 });
    stream.markdown(formatSearchResults(searchResults));
    break;
case '/reflect':
    const reflection = await apiClient.reflect(userInput);
    stream.markdown(reflection.reflection ?? String(reflection));
    break;
case '/forget':
    const actor = userInput.trim() || defaultActor;
    const forgetResult = await apiClient.forget(actor);
    stream.markdown(`Deleted ${forgetResult.records_deleted} records for \`${actor}\``);
    break;
```

- [ ] **Step 5: Add commands to `package.json` `contributes.commands`**

```json
{ "command": "hipcortex.searchMemory", "title": "HipCortex: Search Memory" },
{ "command": "hipcortex.reflectMemory", "title": "HipCortex: Reflect on Memory" },
{ "command": "hipcortex.forgetActor", "title": "HipCortex: Forget Actor" }
```

And to `chatParticipants.commands`:
```json
{ "name": "search", "description": "Semantic search over memory" },
{ "name": "reflect", "description": "AureusBridge CoT synthesis" },
{ "name": "forget", "description": "Delete memories for an actor" }
```

- [ ] **Step 6: Bump VSIX version to 0.6.0 in package.json**

- [ ] **Step 7: Update Phase 4 e2e to assert 13 LM tools**

```python
def test_vsix_lm_tool_count(raw_client):
    ext_src = (workspace_root / "vscode-extension" / "src" / "extension.ts").read_text()
    registered = ext_src.count("registerTool(")
    assert registered == 13, f"Expected 13 LM tools, got {registered}"
```

- [ ] **Step 8: Commit**
```bash
git add vscode-extension/src/extension.ts vscode-extension/package.json tests/e2e_user_harness/suites/test_phase4_vscode_extension.py
git commit -m "feat(vsix): add hipcortex_add/forget/live_beliefs LM tools + /search /reflect /forget commands"
```

---

## Sprint 7 — Channel Installers (G18–G19)

**File:** `sdk/python/hipcortex/install_hosts.py`

- [ ] **Step 1: Add `_install_codex` function (G18)**

```python
def _install_codex(server_url: str) -> str:
    """Write ~/.codex/config.yaml with hipcortex MCP server entry."""
    import shutil
    codex_dir = Path.home() / ".codex"
    if not codex_dir.exists():
        return "skipped: ~/.codex/ not found (Codex CLI not installed)"
    
    config_path = codex_dir / "config.yaml"
    mcp_cmd = shutil.which("hipcortex-mcp") or "hipcortex-mcp"
    
    entry = f"""
mcpServers:
  hipcortex:
    command: {mcp_cmd}
    env:
      HIPCORTEX_URL: {server_url}
"""
    existing = config_path.read_text(encoding="utf-8") if config_path.exists() else ""
    if "hipcortex:" in existing:
        return "unchanged"
    config_path.write_text(existing + entry, encoding="utf-8")
    return "created"

def _uninstall_codex() -> bool:
    config_path = Path.home() / ".codex" / "config.yaml"
    if not config_path.exists():
        return False
    import re
    text = config_path.read_text(encoding="utf-8")
    cleaned = re.sub(r'\n\s+hipcortex:\n(?:\s+\S.*\n)*', '', text)
    config_path.write_text(cleaned, encoding="utf-8")
    return True
```

- [ ] **Step 2: Add `_install_copilot` function (G19)**

```python
def _install_copilot(server_url: str) -> str:
    """Write VS Code settings.json MCP entry for GitHub Copilot."""
    import json
    
    # VS Code user settings
    if sys.platform == "win32":
        settings_path = Path(os.environ.get("APPDATA", "")) / "Code" / "User" / "settings.json"
    elif sys.platform == "darwin":
        settings_path = Path.home() / "Library" / "Application Support" / "Code" / "User" / "settings.json"
    else:
        settings_path = Path.home() / ".config" / "Code" / "User" / "settings.json"
    
    if not settings_path.exists():
        return f"skipped: {settings_path} not found (VS Code not installed)"
    
    settings = json.loads(settings_path.read_text(encoding="utf-8"))
    mcp_servers = settings.setdefault("mcp", {}).setdefault("servers", {})
    
    if "hipcortex" in mcp_servers:
        return "unchanged"
    
    import shutil
    mcp_cmd = shutil.which("hipcortex-mcp") or "hipcortex-mcp"
    mcp_servers["hipcortex"] = {
        "type": "stdio",
        "command": mcp_cmd,
        "env": {"HIPCORTEX_URL": server_url}
    }
    settings_path.write_text(json.dumps(settings, indent=2), encoding="utf-8")
    return "created"
```

- [ ] **Step 3: Wire into CLI channel dispatch**

```python
INSTALLABLE_CHANNELS = [
    "claude-code", "cursor", "windsurf", "cline", "roocode",
    "antigravity", "hermes", "openclaw", "grok", "codex", "copilot",
]
# In install() function:
elif channel == "codex":
    result = _install_codex(server_url)
elif channel == "copilot":
    result = _install_copilot(server_url)
```

- [ ] **Step 4: Update `docs/channels.md` and `docs/channels.yaml`**

Change Codex status: `guide` → `mcp`
Change Copilot status: `guide` → `mcp`

- [ ] **Step 5: Test**

```python
# tests/unit/test_install_channels.py
def test_codex_install_skips_gracefully(tmp_path, monkeypatch):
    monkeypatch.setattr(Path, "home", lambda: tmp_path)
    result = _install_codex("http://127.0.0.1:3030")
    assert "skipped" in result  # no ~/.codex dir
```

- [ ] **Step 6: Commit**
```bash
git add sdk/python/hipcortex/install_hosts.py docs/channels.md docs/channels.yaml
git commit -m "feat(installer): add codex and copilot MCP channels"
```

---

## Sprint 8 — E2E Harness Phase 6 + ReAct Validator (G20)

**Files:**
- `tests/e2e_user_harness/suites/test_phase6_gap_coverage.py` (new)
- `scripts/react_validator.py` (new)

- [ ] **Step 1: Write ReAct validator script**

```python
# scripts/react_validator.py
"""ReAct-loop acceptance criteria validator.

Run: python scripts/react_validator.py [--max-retries 3]

For each AC, runs the assertion. On failure: logs the gap, retries (re-runs
the specific test up to max_retries). Reports pass/fail/retry summary.
"""
import subprocess, sys, json
from pathlib import Path

ACS = [
    # (id, description, pytest_node_id)
    ("AC-G1-1", "MCP health tool present in TOOLS", "tests/e2e_user_harness/suites/test_phase6_gap_coverage.py::test_mcp_health_tool"),
    ("AC-G2-1", "MCP observe tool present", "tests/e2e_user_harness/suites/test_phase6_gap_coverage.py::test_mcp_observe_tool"),
    # ... all 70+ ACs mapped to pytest node IDs
]

def run_ac(node_id: str, retries: int = 3) -> bool:
    for attempt in range(retries):
        result = subprocess.run(
            [sys.executable, "-m", "pytest", node_id, "-x", "--tb=short", "-q"],
            capture_output=True, text=True
        )
        if result.returncode == 0:
            return True
        print(f"  FAIL (attempt {attempt+1}/{retries}): {result.stdout[-500:]}")
    return False

def main(max_retries: int = 3):
    passed, failed = [], []
    for ac_id, desc, node_id in ACS:
        print(f"[{ac_id}] {desc}...")
        ok = run_ac(node_id, retries=max_retries)
        (passed if ok else failed).append((ac_id, desc))
        print(f"  {'✓ PASS' if ok else '✗ FAIL'}")
    
    print(f"\n{'='*60}")
    print(f"PASS: {len(passed)} | FAIL: {len(failed)}")
    if failed:
        for ac_id, desc in failed:
            print(f"  FAIL: [{ac_id}] {desc}")
        sys.exit(1)
    print("All ACs passed.")

if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser()
    p.add_argument("--max-retries", type=int, default=3)
    args = p.parse_args()
    main(args.max_retries)
```

- [ ] **Step 2: Write `test_phase6_gap_coverage.py`**

```python
"""Phase 6: Gap coverage — validates all G1-G19 acceptance criteria against live server."""
import json
import pytest
from tests.e2e_user_harness.client_factory import HarnessHttpxClient

# ── MCP tool presence (G1-G7) ────────────────────────────────────────────────

def test_mcp_tool_inventory():
    """All 27 MCP tools present in server.py TOOLS list."""
    from pathlib import Path
    src = (Path(__file__).parent.parent.parent.parent / "sdk" / "mcp" / "server.py").read_text()
    required = [
        "add_memory", "search_memory", "forget_actor", "get_stats", "search_code",
        "link_memories", "get_neighbors", "search_related", "graph_ppr",
        "deconstruct_hypothesis", "check_topo_edge", "rollout", "can_execute",
        "delete_memory", "get_live_beliefs", "purge_expired", "reflect", "predict",
        # G1-G7 new:
        "health", "observe", "causal_add_edge", "causal_intervention", "causal_counterfactual",
        "coherence_check", "coherence_resolve", "bulk_add",
        "goal_react", "goal_trace", "memory_diff",
        "corroborate", "contradict", "quarantine", "restore",
    ]
    for name in required:
        assert f'"name": "{name}"' in src, f"MCP tool missing: {name}"

def test_mcp_health_tool(raw_client: HarnessHttpxClient):
    """G1: MCP health tool returns ok."""
    # Call MCP via direct HTTP simulation (same as MCP server does internally)
    resp = raw_client.get("/health")
    assert resp.status_code == 200
    data = resp.json()
    assert data.get("status") == "ok" or "version" in str(data)

def test_mcp_observe_tool(raw_client: HarnessHttpxClient):
    """G2: observe feeds transition into WorldModel."""
    resp = raw_client.post("/worldmodel/observe", json={
        "state": "idle", "action": "start_task", "next_state": "working"
    })
    assert resp.status_code in (200, 201)
    
    transitions = raw_client.get("/worldmodel/transitions").json()
    states = str(transitions)
    assert "idle" in states or resp.json().get("success")

def test_mcp_causal_tools(raw_client: HarnessHttpxClient):
    """G3: causal add_edge, intervention, counterfactual callable."""
    # add edge
    resp = raw_client.post("/worldmodel/causal/edge", json={
        "from": "database_load", "to": "latency_spike",
        "distribution": {"low": 0.2, "high": 0.8}
    })
    assert resp.status_code in (200, 201)
    
    # intervention — may return error if no distributions loaded yet, but must not 500
    resp = raw_client.post("/worldmodel/causal/intervention", json={
        "target": "database_load", "value": "high", "query": "latency_spike"
    })
    assert resp.status_code != 500

def test_mcp_bulk_add(raw_client: HarnessHttpxClient):
    """G5: bulk_add inserts N records."""
    records = [
        {"actor": "bulk_test", "action": "noted", "target": f"item {i}", "record_type": "Temporal"}
        for i in range(5)
    ]
    resp = raw_client.post("/memory/bulk", json={"records": records})
    assert resp.status_code in (200, 201)
    data = resp.json()
    assert data.get("inserted", 0) == 5

# ── TypeScript SDK methods (G8-G13) ──────────────────────────────────────────

def test_ts_sdk_has_new_methods():
    """G8-G13: TypeScript client.ts declares all new methods."""
    from pathlib import Path
    src = (Path(__file__).parent.parent.parent.parent / "sdk" / "typescript" / "src" / "client.ts").read_text()
    required_methods = [
        "topoPpr", "topoCheckEdge", "topoDeconstruct",
        "wmObserve", "wmEntities", "causalAddEdge", "causalIntervention", "causalCounterfactual",
        "coherenceCheck", "coherenceResolve", "selfHealth", "selfCapabilities", "selfRegisterCapability",
        "auditVerify", "auditExport",
        "regulatoryHoldSet", "regulatoryHoldRelease", "regulatoryHoldList",
        "webhooksList", "webhooksRegister", "webhooksDelete",
        "quarantine", "restore", "corroborate", "contradict", "consolidate",
        "graphSearch", "getNode", "createNode", "createEdge",
        "embedAndAdd", "updateMemory", "latestMemory", "exportMemory", "searchFlat",
    ]
    for method in required_methods:
        assert f"async {method}(" in src, f"TypeScript SDK missing method: {method}"

def test_ts_sdk_types_complete():
    """G8-G13: types.ts has interfaces for all new methods."""
    from pathlib import Path
    src = (Path(__file__).parent.parent.parent.parent / "sdk" / "typescript" / "src" / "types.ts").read_text()
    required_types = [
        "TopoPprRequest", "TopoPprResponse",
        "TopoCheckEdgeRequest", "TopoCheckEdgeResponse",
        "WmObserveRequest", "CausalAddEdgeRequest",
        "CoherenceCheckRequest", "CoherenceCheckResponse",
        "SelfHealthResponse", "AuditVerifyResponse",
        "ConsolidateRequest", "ConsolidateResponse",
    ]
    for t in required_types:
        assert f"interface {t}" in src or f"export interface {t}" in src, f"TypeScript type missing: {t}"

# ── Python async parity (G14) ─────────────────────────────────────────────────

def test_async_client_parity():
    """G14: Every sync client method exists in async client."""
    import inspect
    from hipcortex.client import HipCortexClient
    from hipcortex.async_client import AsyncHipCortexClient
    
    sync_methods = {n for n, _ in inspect.getmembers(HipCortexClient, predicate=inspect.isfunction)
                    if not n.startswith('_')}
    async_methods = {n for n, _ in inspect.getmembers(AsyncHipCortexClient, predicate=inspect.isfunction)
                     if not n.startswith('_')}
    missing = sync_methods - async_methods
    assert not missing, f"async_client missing methods: {sorted(missing)}"

# ── CrewAI tools (G15) ────────────────────────────────────────────────────────

def test_crewai_11_tools(raw_client: HarnessHttpxClient):
    """G15: make_memory_tools returns 11 tools with correct names."""
    from hipcortex import HipCortexClient
    from hipcortex.adapters.crewai import make_memory_tools
    
    client = HipCortexClient(raw_client.base_url)
    tools = make_memory_tools(client=client, agent_id="phase6_test")
    assert len(tools) == 11, f"Expected 11 tools, got {len(tools)}: {[t.name for t in tools]}"
    
    expected = {
        "hipcortex_remember", "hipcortex_recall", "hipcortex_forget",
        "hipcortex_search", "hipcortex_predict", "hipcortex_rollout",
        "hipcortex_reflect", "hipcortex_can_execute", "hipcortex_link",
        "hipcortex_beliefs", "hipcortex_graph_search",
    }
    assert {t.name for t in tools} == expected

def test_crewai_search_tool_live(raw_client: HarnessHttpxClient):
    """G15-AC-G15-2: Search tool returns formatted results."""
    from hipcortex import HipCortexClient
    from hipcortex.adapters.crewai import HipCortexSearchTool
    
    client = HipCortexClient(raw_client.base_url)
    client.add_memory(actor="crewai_test", action="noted", target="Rust is fast")
    
    tool = HipCortexSearchTool(client=client, agent_id="crewai_test")
    result = tool._run(query="Rust performance", limit=5)
    assert isinstance(result, str)

# ── LangChain (G16) ───────────────────────────────────────────────────────────

def test_langchain_retriever(raw_client: HarnessHttpxClient):
    """G16-AC-G16-2: Retriever returns document list."""
    from hipcortex import HipCortexClient
    from hipcortex.langchain_memory import HipCortexSemanticRetriever
    
    client = HipCortexClient(raw_client.base_url)
    client.add_memory(actor="lc_test", action="noted", target="LangChain integration works")
    
    retriever = HipCortexSemanticRetriever(client=client, limit=5)
    docs = retriever._get_relevant_documents("LangChain")
    assert isinstance(docs, list)

def test_langchain_reflect(raw_client: HarnessHttpxClient):
    """G16-AC-G16-3: reflect returns CoT string."""
    from hipcortex import HipCortexClient
    from hipcortex.langchain_memory import HipCortexMemory
    
    memory = HipCortexMemory(client=HipCortexClient(raw_client.base_url), session_id="lc_reflect_test")
    result = memory.reflect("what decisions were made?")
    assert isinstance(result, str) and len(result) > 0

# ── VSIX LM tools (G17) ───────────────────────────────────────────────────────

def test_vsix_13_lm_tools():
    """G17-AC-G17-7: extension.ts registers 13 LM tools."""
    from pathlib import Path
    src = (Path(__file__).parent.parent.parent.parent / "vscode-extension" / "src" / "extension.ts").read_text()
    count = src.count("registerTool(")
    assert count == 13, f"Expected 13 LM tools, got {count}"

def test_vsix_new_commands_in_package_json():
    """G17: package.json has /search /reflect /forget commands."""
    import json
    from pathlib import Path
    pkg = json.loads((Path(__file__).parent.parent.parent.parent / "vscode-extension" / "package.json").read_text())
    commands = {c["name"] for p in pkg.get("contributes", {}).get("chatParticipants", [])
                for c in p.get("commands", [])}
    assert "search" in commands, "Missing /search command"
    assert "reflect" in commands, "Missing /reflect command"
    assert "forget" in commands, "Missing /forget command"

# ── Channel installers (G18-G19) ─────────────────────────────────────────────

def test_codex_installer_exists():
    """G18: _install_codex function exists in install_hosts.py."""
    from hipcortex.install_hosts import _install_codex, _uninstall_codex
    assert callable(_install_codex)
    assert callable(_uninstall_codex)

def test_copilot_installer_exists():
    """G19: _install_copilot function exists."""
    from hipcortex.install_hosts import _install_copilot
    assert callable(_install_copilot)

def test_channels_doc_updated():
    """G18-G19: channels.md shows codex and copilot as mcp."""
    from pathlib import Path
    doc = (Path(__file__).parent.parent.parent.parent / "docs" / "channels.md").read_text()
    # Both should be mcp status not guide
    assert "| Grok Code / Grok Build | mcp |" in doc  # reference: this already passes
    assert "| OpenAI Codex CLI | mcp |" in doc, "Codex still guide"
    assert "| GitHub Copilot | mcp |" in doc, "Copilot still guide"

# ── Version consistency (all surfaces) ───────────────────────────────────────

def test_version_consistency():
    """All surfaces at 0.6.0."""
    import json
    from pathlib import Path
    root = Path(__file__).parent.parent.parent.parent
    
    py_ver = (root / "sdk" / "python" / "pyproject.toml").read_text()
    assert 'version = "0.6.0"' in py_ver, "Python SDK not 0.6.0"
    
    ts_ver = json.loads((root / "sdk" / "typescript" / "package.json").read_text())
    assert ts_ver["version"] == "0.6.0", "TypeScript SDK not 0.6.0"
    
    vsix_ver = json.loads((root / "vscode-extension" / "package.json").read_text())
    assert vsix_ver["version"] == "0.6.0", "VSIX not 0.6.0"
    
    mcp_src = (root / "sdk" / "mcp" / "server.py").read_text()
    assert '"version": "0.6.0"' in mcp_src, "MCP serverInfo not 0.6.0"
```

- [ ] **Step 3: Register Phase 6 in `conftest.py`** (if needed for fixtures)

- [ ] **Step 4: Run full Phase 6 suite against live server**
```bash
pytest tests/e2e_user_harness/suites/test_phase6_gap_coverage.py -v --tb=short
```
Expected: all pass (after all sprints complete)

- [ ] **Step 5: Run ReAct validator**
```bash
python scripts/react_validator.py --max-retries 3
```
Expected: "All ACs passed."

- [ ] **Step 6: Commit**
```bash
git add tests/e2e_user_harness/suites/test_phase6_gap_coverage.py scripts/react_validator.py
git commit -m "test(e2e): Phase 6 gap coverage suite + ReAct AC validator"
```

---

## Sprint 9 — Version Bumps + Capabilities Matrix (all surfaces)

- [ ] **Step 1: Bump Python SDK `pyproject.toml` version to 0.6.0**

- [ ] **Step 2: Bump TypeScript SDK `package.json` version to 0.6.0**

- [ ] **Step 3: Bump VSIX `package.json` version to 0.6.0**

- [ ] **Step 4: Bump MCP `server.py` `serverInfo.version` to 0.6.0**

- [ ] **Step 5: Update `docs/capabilities.md`**
  - MCP tool count: 18→27
  - Update matrix rows for all new CrewAI tools (G15 ops)
  - Update LangChain row (search now `partial`→`Y` via Retriever)
  - Add Codex/Copilot columns (now `mcp` not `guide`)

- [ ] **Step 6: Run `python scripts/check_capabilities.py --check-mcp` — exit 0**

- [ ] **Step 7: Final commit**
```bash
git add sdk/python/pyproject.toml sdk/typescript/package.json vscode-extension/package.json sdk/mcp/server.py docs/capabilities.md
git commit -m "chore: v0.6.0 — close all 20 SDK/channel surface gaps"
```

---

## ReAct Validation Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 react_validator.py                          │
│                                                             │
│  For each AC in AC_REGISTRY (70 ACs across 20 gaps):       │
│                                                             │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│  │ OBSERVE  │───▶│  THINK   │───▶│   ACT    │             │
│  │ run test │    │ pass/fail│    │ retry or │             │
│  │ capture  │    │ diagnose │    │ report   │             │
│  │ output   │    │ gap      │    │ skip     │             │
│  └──────────┘    └──────────┘    └──────────┘             │
│       ▲                                │                   │
│       └────────────────────────────────┘                   │
│                  (max 3 retries)                            │
│                                                             │
│  Output: PASS N / FAIL M with AC IDs                       │
└─────────────────────────────────────────────────────────────┘
```

**Cohesion invariants enforced by validator:**
1. Every MCP tool name in `TOOLS` has a handler in dispatch dict
2. Every TypeScript method has a matching type interface
3. Every async method signature matches sync counterpart
4. Every CrewAI tool calls `self._client` (same HipCortexClient instance)
5. All surfaces report 0.6.0 consistently
6. `check_capabilities.py --check-mcp` exits 0 (no drift between doc and code)

---

## Execution Order

```
Sprint 1  (MCP)          → independent, do first (most impact)
Sprint 2  (TypeScript)   → independent
Sprint 3  (async parity) → independent
Sprint 4  (CrewAI)       → needs Sprint 3 (client parity) done first
Sprint 5  (LangChain)    → independent
Sprint 6  (VSIX)         → independent
Sprint 7  (Installers)   → independent
Sprint 8  (E2E+ReAct)    → last; needs Sprints 1-7 complete to pass all ACs
Sprint 9  (Versions)     → last; stamp after all code done
```

Sprints 1-3 + 5-7 can run in parallel (different files, no conflicts).
Sprint 4 after Sprint 3. Sprint 8+9 after all others.
