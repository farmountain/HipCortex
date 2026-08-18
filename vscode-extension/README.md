# HipCortex Memory Engine & Cognitive OS for VS Code & Antigravity IDE (`v0.8.0`)

[![Version](https://img.shields.io/badge/version-v0.8.0-blue.svg)](package.json)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
![Latency](https://img.shields.io/badge/write_p50-0.48ms__--__0.61ms-brightgreen.svg)
![Token Savings](https://img.shields.io/badge/token_savings-59%25__--__88%25-blueviolet.svg)

**Give your AI coding assistant persistent, cross-session causal memory with a full cognitive OS substrate — transactional belief revision, multi-agent workspaces, world-model rollout, and topological graph tools.**

Product server / pip / npm are **0.8.0**. 82/82 E2E scenarios pass. See [docs/channels.md](../docs/channels.md).

---

## Zero-config onboarding (no Rust or Cargo required)

Install from Marketplace / Open VSX / GitHub release VSIX. Extension **starts a local Rust webserver** under `~/.hipcortex-vscode/bin/` (or uses `hipcortex.apiUrl`).

- **Zero external DB / Docker** for default petgraph path
- **Local-first** storage under `~/.hipcortex-vscode/storage`
- **Auto-recovery**: restarts server before queries when down
- **Executable bundled bins**: `chmod 0755` applied on macOS/Linux (fixes spawn `EACCES`)
- **Passive capture**: saves code edits and terminal output automatically when `hipcortex.passiveCapture` is `true`

```bash
code --install-extension hipcortex-memory-0.8.0.vsix
```

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
| `triggerConsolidation` | transact `AutoConsolidate` delta |
| `getLiveBeliefs` | `GET /v1/beliefs/live` |

---

## MCP Integration (36 tools)

MCP hosts (Claude Code, Cursor, Windsurf, …) use the Python MCP server via `hipcortex install`.  
36 tools + 3 auto-injected resources:

- `hipcortex://context/relevant` — top-k semantically relevant memories
- `hipcortex://beliefs/current` — active belief records
- `hipcortex://context/conversation` — recent temporal traces

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

Produces `hipcortex-memory-0.8.0.vsix` (version from `package.json`).

---

## Related

- Channel honesty: [docs/channels.md](../docs/channels.md) · `hipcortex channels`
- Capability matrix: [docs/capabilities.md](../docs/capabilities.md)
- Host wizards: [docs/hosts/README.md](../docs/hosts/README.md)
- Architecture: [docs/architecture.md](../docs/architecture.md)
