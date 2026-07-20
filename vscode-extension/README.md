# HipCortex Memory Engine & Cognitive OS for VS Code & Antigravity IDE (`v0.5.7`)

[![Version](https://img.shields.io/badge/version-v0.5.7-blue.svg)](package.json)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
![Latency](https://img.shields.io/badge/write_p50-0.48ms__--__0.61ms-brightgreen.svg)
![Token Savings](https://img.shields.io/badge/token_savings-59%25__--__88%25-blueviolet.svg)

**Give your AI coding assistant (VS Code Copilot & Antigravity IDE) persistent, cross-session causal memory, world-model prediction/rollout, and topological graph tools.**

Product server / pip / npm remain **0.5.0**; this extension package is **0.5.7** (10 LM tools, dual `/health`, multi-OS bundled webserver). See [docs/channels.md](../docs/channels.md).

---

## Zero-config onboarding (no Rust or Cargo required)

Install from Marketplace / Open VSX / GitHub release VSIX. Extension **starts a local Rust `webserver`** under `~/.hipcortex-vscode/bin/` (or uses `hipcortex.apiUrl`).

- **Zero external DB / Docker** for default petgraph path
- **Local-first** storage under `~/.hipcortex-vscode/storage`
- **Dual `/health`**: accepts plain `ok` **and** JSON `{status,service}` (macOS-friendly ready poll)
- **Auto-recovery**: restarts server before queries when down
- **Release install**: [hipcortex-memory-0.5.7.vsix](https://github.com/farmountain/HipCortex/releases/download/v0.5.7/hipcortex-memory-0.5.7.vsix)

```bash
code --install-extension hipcortex-memory-0.5.7.vsix
```

---

## `@hipcortex` chat commands

Open Copilot / Antigravity chat and type `@hipcortex`:

- `@hipcortex health` — server status, tier counts, Merkle integrity
- `@hipcortex add <content>` — store decision / preference / constraint
- `@hipcortex query <query>` — semantic + topological retrieval
- `@hipcortex status` — Headroom vs Caveman mode and savings

---

## Language Model Tools (10)

Extension registers **10** tools with `vscode.lm` (requires host LM tool API):

| Tool | Purpose |
|------|---------|
| `hipcortex_search` | Semantic + live_beliefs-aware search |
| `hipcortex_health` | Health + capability gate |
| `hipcortex_predict` | WorldModel single-step `P(s'|s,a)` |
| `hipcortex_rollout` | Multi-step Dirichlet / MCTS rollout (`POST /worldmodel/rollout`) |
| `hipcortex_graph_search` | PPR / related memories from seed UUID |
| `hipcortex_causal` | Causal attribution |
| `hipcortex_topo_ppr` | Topo graph Personalized PageRank (`/topo/*`) |
| `hipcortex_deconstruct` | Hypothesis → candidate edges (`/topo/deconstruct`) |
| `hipcortex_check_edge` | Contradiction / cycle check before link |
| `hipcortex_can_execute` | SelfModel gate |

MCP hosts use the Python MCP server (18 tools) via `hipcortex install` — parallel path; see [sdk/mcp/README.md](../sdk/mcp/README.md).

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
  "hipcortex.optimizationMode": "headroom"
}
```

---

## Local development & packaging

```bash
cd vscode-extension
npm install
npm run compile
npm test
npx @vscode/vsce package --allow-missing-repository
```

Produces `hipcortex-memory-0.5.7.vsix` (version from `package.json`).

CI release workflow also packages multi-OS VSIX after matrix binary builds (tag `v0.5.7`).

---

## Related

- Channel honesty: [docs/channels.md](../docs/channels.md) · `hipcortex channels`
- Capability matrix: [docs/capabilities.md](../docs/capabilities.md)
- Host wizards: [docs/hosts/README.md](../docs/hosts/README.md)
