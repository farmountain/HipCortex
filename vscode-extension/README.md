# HipCortex Memory Engine & Cognitive OS for VS Code & Antigravity IDE (`v0.5.0`)

[![Version](https://img.shields.io/badge/version-v0.5.0-blue.svg)](package.json)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
![Latency](https://img.shields.io/badge/write_p50-0.48ms__--__0.61ms-brightgreen.svg)
![Token Savings](https://img.shields.io/badge/token_savings-59%25__--__88%25-blueviolet.svg)

**Give your AI coding assistant (VS Code Copilot & Antigravity IDE) persistent, cross-session causal memory and world-model prediction (`/worldmodel/rollout`).**

---

## 🚀 Zero-Config Onboarding (No Rust or Cargo Required!)

When you install `HipCortex Memory Engine & Cognitive OS` from the VS Code Marketplace or Open VSIX Registry, the extension **automatically downloads and runs the standalone `v0.5.0` local Rust binary (`webserver.exe`) in the background** (`~/.hipcortex-vscode/bin/`).

- **Zero dependencies**: No external databases, no Python required, no Docker needed.
- **Local-first & private**: All memories and causal graphs are stored locally on your machine (`~/.hipcortex-vscode/storage`). Zero telemetry.
- **Sub-millisecond speed**: `0.48–0.61 ms p50` write latency over local loopback (`http://127.0.0.1:3030`).
- **Auto-recovery & lifecycle**: If the server stops or crashes, the extension restarts it automatically before any query.

---

## 💬 Interactive `@hipcortex` Chat Commands

Open your GitHub Copilot or Antigravity Chat panel and type `@hipcortex`:

- `@hipcortex health`: Inspect local server status, memory tier counts (`WorkingSet`, `ShortTerm`, `LongTerm`), and Merkle chain audit integrity.
- `@hipcortex add <content>`: Store an explicit architectural decision, user preference, or project constraint into persistent memory (`Symbolic` / `Semantic` tier).
- `@hipcortex query <query>`: Perform semantic + topological graph retrieval (`petgraph` Personalized PageRank) across your workspace history.
- `@hipcortex status`: Display current token optimization mode (`Headroom` vs `Caveman`) and active memory savings.

---

## 🛠️ Language Model Tools (`LM Tools`)

HipCortex automatically registers 6 native tools with the VS Code & Antigravity Language Model API (`vscode.lm`):

1. `hipcortex_search`: Semantic and causal graph search over past project sessions (`live_beliefs`).
2. `hipcortex_health`: Server health check and capability gate evaluation (`can_execute`).
3. `hipcortex_predict`: Predict next state using `WorldModelEnhanced` Dirichlet-Multinomial transition matrix.
4. `hipcortex_rollout`: Execute multi-step Monte Carlo Tree Search (`MCTS`) trajectory rollouts (`POST /worldmodel/rollout`).
5. `hipcortex_graph_search`: Graph-theoretic breadth-first and topological traversal over `CausalTopoGraph`.
6. `hipcortex_causal`: Causal attribution analysis and Backdoor Adjustment calculation.

---

## 🧠 Headroom & Caveman Token Optimization (`59% – 88% Savings`)

Long coding sessions typically consume 15,000+ tokens per turn due to full history injection. HipCortex reduces Copilot token consumption while preserving structural awareness:

- **Headroom Mode (`Top-5`)**: Retrieves 5 causal neighbors into `Tier 0` (`SessionContext`), cutting token bills by **`59% to 84%`**.
- **Caveman Mode (`Top-3`)**: Retrieves 3 highest-priority causal neighbors, delivering **`70% to 88%`** token reduction during rapid debugging loops.

---

## ⚙️ Configuration (`settings.json`)

Open VS Code / Antigravity IDE Settings and search for `HipCortex`:

```json
{
  "hipcortex.apiUrl": "http://127.0.0.1:3030",
  "hipcortex.apiKey": "",
  "hipcortex.autoStart": true,
  "hipcortex.optimizationMode": "headroom"
}
```

---

## 🏗️ Local Development & VSIX Packaging

For local verification and extension packaging:
```bash
cd vscode-extension
npm install
npm run compile
npm test
npx @vscode/vsce package --allow-missing-repository
```
This builds `hipcortex-memory-0.5.0.vsix`, ready for `code --install-extension hipcortex-memory-0.5.0.vsix`.
