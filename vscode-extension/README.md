# HipCortex Memory Engine & Cognitive OS for VS Code & Antigravity IDE (`v0.9.0`)

[![Version](https://img.shields.io/badge/version-v0.9.0-blue.svg)](package.json)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
![Latency](https://img.shields.io/badge/write_p50-0.48ms__--__0.61ms-brightgreen.svg)
![Token Savings](https://img.shields.io/badge/token_savings-59%25__--__88%25-blueviolet.svg)

**Give your AI coding assistant persistent, cross-session causal memory with a full cognitive OS substrate — transactional belief revision, multi-agent workspaces, world-model rollout, DigitalTwin simulation, and topological graph tools.**

Product server / pip / npm are **0.9.0**. 837 tests pass (339 lib + 320 unit + 128 integration + 50 property). See [docs/channels.md](../docs/channels.md).

---

## Zero-config onboarding (no Rust or Cargo required)

Install from Marketplace / Open VSX / GitHub release VSIX. Extension **starts a local Rust webserver** under `~/.hipcortex-vscode/bin/` (or uses `hipcortex.apiUrl`).

- **Zero external DB / Docker** for default petgraph path
- **Local-first** storage under `~/.hipcortex-vscode/storage`
- **Auto-recovery**: restarts server before queries when down
- **Executable bundled bins**: `chmod 0755` applied on macOS/Linux (fixes spawn `EACCES`)
- **Passive capture**: saves code edits and terminal output automatically when `hipcortex.passiveCapture` is `true`

```bash
code --install-extension hipcortex-memory-0.9.0.vsix
```

---

## What's new in v0.9.0 — Continuous Substrate

v0.9.0 adds a **continuous dynamical simulation layer** on top of the v0.8.0 Cognitive OS substrate.

### DigitalTwin (RK4 + HybridRollout)
- `POST /v1/twin` — create a DigitalTwin fork with configurable state dimension, `dt`, and max covariance
- `POST /v1/twin/:id/step` — advance one step via RK4 integrator; returns continuous state vector
- `POST /v1/twin/:id/rollout` — multi-action HybridRollout; returns `continuous_trajectory` + `continuous_sigma_norm`
- `GET /v1/twin/:id` — inspect trajectory depth and record count
- VS Code commands: **Create DigitalTwin**, **DigitalTwin: Step**, **DigitalTwin: Rollout**, **DigitalTwin: Show State**

### ExperienceStore (3-tier pyramid)
- **Raw** tier — unprocessed `Temporal` records
- **Episode** tier — `Skill`/`Belief` records with evidence links
- **Abstract** tier — consolidated Temporal records (`action="consolidated"`)
- `AutoConsolidate` achieves ≥ 90% hot-set reduction while preserving full provenance
- `GET /v1/experience/:actor/tiers` — tier counts + compression ratio + raw pressure flag
- `POST /v1/experience/:actor/search` — semantic search across all tiers
- VS Code command: **Show Experience Tier Stats**

### Python SDK: `HipCortexSubstrate`
```python
from hipcortex import HipCortexSubstrate
sub = HipCortexSubstrate("http://localhost:3030")
twin_id = sub.create_twin(dim=4, dt=0.1)
state   = sub.twin_step(twin_id, "move_forward")
tiers   = sub.experience_tiers("my-agent")
```

### MCP: 5 new tools + 4 new resources (42 total / 7 resources)
- `twin_create`, `twin_step`, `twin_rollout`, `twin_get`, `experience_tiers`
- Resource: `hipcortex://experience/tiers` auto-injected at session start

---

## What's new in v0.8.0 — Cognitive OS Substrate

v0.8.0 reframes HipCortex from a memory store into a **closed dynamical cognitive substrate**. All state evolution flows through a single transactional operator family (`CognitiveDelta`).

### Transactional Core
- `POST /v1/cognitive/transact` — atomic delta application with CoherenceChecker + TxLog
- `GET /v1/cognitive/snapshot` — typed snapshot with `tx_cursor`, beliefs, goals
- `POST /v1/state/diff` — causal `ΔS` between any two tx indices

### JTMS Belief Revision
- Doyle-style justification-based truth maintenance
- `RetractBelief` / `AssertJustification` — cascade retraction through dependency graph
- No belief stays IN without a valid justification

### Causal Motif Compactor
- Frequent causal path mining → `SkillPayload` + high-confidence `Belief` induction
- `AutoConsolidate { min_frequency }` — sub-linear memory growth on long trajectories

### Multi-Agent Workspaces
- `WorkspaceOpen { mode: Private | Shared }` + `WorkspaceMerge` deltas
- OR-Set CRDT merge for concurrent mutations; no silent contamination

### Rollout + Drift Alarms
- Multi-step Kalman covariance expansion (discrete Lyapunov recursion)
- Continuous goal-distance drift alarm on `POST /v1/fork/:id/rollout`

---

## `@hipcortex` chat commands

Open Copilot / Antigravity chat and type `@hipcortex`:

- `@hipcortex health` — server status, calibration score, epistemic entropy
- `@hipcortex add <content>` — store decision / preference / constraint
- `@hipcortex query <query>` — semantic + topological retrieval
- `@hipcortex status` — Headroom vs Caveman mode and savings

---

## Language Model Tools (10)

Extension registers **10** tools with `vscode.lm` (requires host LM tool API):

| Tool | Purpose |
|------|---------|
| `hipcortex_search` | Semantic + live-belief-aware search |
| `hipcortex_health` | Health + calibration + capability gate |
| `hipcortex_predict` | WorldModel single-step `P(s'|s,a)` |
| `hipcortex_rollout` | Multi-step Kalman rollout with drift alarm |
| `hipcortex_graph_search` | PPR / related memories from seed UUID |
| `hipcortex_causal` | Causal attribution |
| `hipcortex_topo_ppr` | Topological Personalized PageRank |
| `hipcortex_deconstruct` | Hypothesis → candidate causal edges |
| `hipcortex_check_edge` | Contradiction / cycle check before link |
| `hipcortex_can_execute` | SelfModel ExecutionGate |

---

## VS Code Commands (15)

| Command | Action |
|---------|--------|
| `hipcortex.addMemory` | Add memory record |
| `hipcortex.queryMemory` | Query memory records |
| `hipcortex.healthCheck` | System health check |
| `hipcortex.predictState` | Predict next state |
| `hipcortex.systemHealth` | Calibrated health + ECE |
| `hipcortex.stateDiff` | Causal state diff (tx range) |
| `hipcortex.cognitiveHealth` | Cognitive health status |
| `hipcortex.cognitiveSnapshot` | Cognitive snapshot |
| `hipcortex.twinCreate` | **NEW** Create DigitalTwin |
| `hipcortex.twinStep` | **NEW** DigitalTwin: Step |
| `hipcortex.twinRollout` | **NEW** DigitalTwin: Rollout |
| `hipcortex.twinGet` | **NEW** DigitalTwin: Show State |
| `hipcortex.experienceTiers` | **NEW** Show Experience Tier Stats |
| `hipcortex.restartServer` | Restart server |
| `hipcortex.testExtension` | Test extension |

---

## Phase-5 Operator Methods (8)

Available as `HipCortexClient` TypeScript methods and wired to VS Code commands:

| Method | REST endpoint |
|--------|--------------|
| `cognitiveTransact` | `POST /v1/cognitive/transact` |
| `computeStateDiff` | `POST /v1/state/diff` |
| `simulateRollout` | `POST /v1/fork/:id/rollout` |
| `workspaceOpen` | transact `WorkspaceOpen` delta |
| `workspaceMerge` | transact `WorkspaceMerge` delta |
| `retractBelief` | transact `RetractBelief` delta |
| `triggerConsolidation` | transact `AutoConsolidate` delta (MCP: `consolidate_memory`) |
| `getLiveBeliefs` | `GET /v1/beliefs/live` |
| `getStateExport` | `GET /v1/state/export` — versioned `schema_version=0.9.0` snapshot (MCP: `get_state_export`) |

---

## MCP Integration (42 tools, 7 resources)

MCP hosts (Claude Code, Cursor, Windsurf, …) use the Python MCP server via `hipcortex install`.  
42 tools + 7 auto-injected resources:

- `hipcortex://context/relevant` — top-k semantically relevant memories
- `hipcortex://beliefs/current` — active belief records
- `hipcortex://context/conversation` — recent temporal traces
- `hipcortex://experience/tiers` — **NEW** ExperienceStore tier stats for current actor

Register in `.mcp.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "type": "stdio",
      "command": "python",
      "args": ["/path/to/hipcortex/sdk/mcp/server.py"],
      "env": { "HIPCORTEX_URL": "http://localhost:3030" }
    }
  }
}
```

---

## Headroom & Caveman (token savings)

- **Headroom (Top-5)**: ~59–84% token reduction vs full history dump
- **Caveman (Top-3)**: ~70–88% in tight debug loops

---

## Configuration (`settings.json`)

```json
{
  "hipcortex.apiUrl": "http://127.0.0.1:3030",
  "hipcortex.apiKey": "",
  "hipcortex.autoStart": true,
  "hipcortex.optimizationMode": "headroom",
  "hipcortex.passiveCapture": true
}
```

---

## Local development & packaging

```bash
cd vscode-extension
npm install
npm run compile
npm test
npx @vscode/vsce package --no-dependencies
```

Produces `hipcortex-memory-0.9.0.vsix` (version from `package.json`).

---

## Related

- Channel honesty: [docs/channels.md](../docs/channels.md) · `hipcortex channels`
- Capability matrix: [docs/capabilities.md](../docs/capabilities.md)
- Host wizards: [docs/hosts/README.md](../docs/hosts/README.md)
- Architecture: [docs/architecture.md](../docs/architecture.md)
