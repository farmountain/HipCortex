# HipCortex v0.4.9 Documentation Rewrite & Codebase Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite `README.md`, `BENCHMARK.md`, `vscode-extension/README.md`, `vscode-extension/package.json`, `sdk/mcp/README.md`, `sdk/python/README.md`, and `sdk/typescript/README.md` to establish HipCortex `v0.4.9` as the Autonomous Cognitive OS with verified Headroom & Caveman token savings and the rigorous Caveman Comparison Matrix (`0.48–0.61 ms local vs 15–35 ms local vector vs 142 ms cloud`).

**Architecture:** Adopt **Approach A: The Rigorous Caveman & Headroom Engineering Matrix** approved in `docs/superpowers/specs/2026-07-11-rewrite-docs-v049-design.md`, establishing unassailable factual context across all developer surfaces (GitHub, VS Code Marketplace, Antigravity IDE, 12-Agent Universal MCP ecosystem, and SDK client packages).

**Tech Stack:** Markdown, JSON, Git, Python/TypeScript SDK snippets, open-source documentation standards.

## Global Constraints

1. All version badges and text references MUST state `v0.4.9` (`0.4.9`).
2. All latency comparisons MUST include the **Caveman Comparison Matrix** (`0.48–0.61 ms local Rust loopback` vs `15–35 ms local self-hosted Python vector` vs `142 ms cloud vector API`). Never state `300× faster` in isolation without explaining the network round-trip and vector embedding inference difference.
3. Token reduction claims MUST use the exact `tiktoken cl100k_base` benchmark results from `token_reduction_benchmark.py`: `Headroom Mode (Top-5)` (`-59% steady state, -84% at 50 turns`) vs `Caveman Mode (Top-3)` (`-70% steady state, -88% at 50 turns`) vs `Proactive Substrate` (`-93% steady state`).
4. `sdk/mcp/README.md` MUST provide exact, copy-pasteable JSON configuration snippets across all 12 target autonomous agents: Claude Code, Cursor, Windsurf, Grok Code, Hermes, OpenClaw, Cline, RooCode, OpenAI Codex CLI, Aider, Gemini CLI, and Amazon Q Developer.

---

### Task 1: Root Repository `README.md` & `BENCHMARK.md` Rewrite (`/opsx-apply` Tasks 1.1 & 1.2)

**Files:**
- Modify: `README.md:1-120`
- Modify: `BENCHMARK.md:20-68`

**Interfaces:**
- Consumes: Verified latency results (`2.05 ms` p50 Windows desktop loopback, `0.61 ms` Linux x86_64, `0.48 ms` early sub-ms bare loopback) and token reduction numbers from `token_reduction_benchmark.py`.
- Produces: Root `README.md` and `BENCHMARK.md` adhering to the Caveman Comparison Matrix and 6-layer architecture specification.

- [ ] **Step 1: Write `README.md` with verified `v0.4.9` Cognitive OS narrative and Caveman Comparison Matrix**

Modify `README.md` to start with:
```markdown
# HipCortex — The Autonomous Cognitive OS & Persistent Causal Substrate (`v0.4.9`)

![Version](https://img.shields.io/badge/version-v0.4.9-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Rust](https://img.shields.io/badge/rust-1.95%2B-orange.svg)
![Latency](https://img.shields.io/badge/write_p50-0.48ms__--__0.61ms-brightgreen.svg)
![Token Savings](https://img.shields.io/badge/token_savings-59%25__--__88%25-blueviolet.svg)

**Persistent causal topological memory, recursive Bayesian world-model prediction (`/worldmodel/rollout`), and automatic FSM skill compilation for autonomous AI agents.**

Runs locally as a **single `4 MB` zero-dependency compiled Rust binary (`webserver.exe`)** with sub-millisecond writes (`0.48–0.61 ms p50`), SHA-256 Merkle audit chains, and adaptive context budgeting (`WorkingSetBroker`).

---

## ⚡ The Caveman Comparison Matrix (`Fact vs. Cloud & Local Vectors`)

We believe in **100% rigorous, unassailable engineering benchmarks** (`Headroom & Caveman mode audits`). When comparing memory engines, transport layer and embedding computation model matter:

| System / Substrate | Write Median (`add_p50`) | Write 95th (`add_p95`) | Query Median (`query_p50`) | Architectural & Transport Reality |
| :--- | :---: | :---: | :---: | :--- |
| **HipCortex Local Rust (`v0.4.9` Linux)** | **`0.61 ms`** (`0.48 ms` bare) | **`1.1 ms`** | **`0.23 ms`** | Compiled `4 MB` Rust binary over local HTTP (`127.0.0.1`). Zero public network RTT. Indexes causal topological relationships (`petgraph`) + SHA-256 Merkle audit chains without heavy dense vector inference bottlenecks. |
| **HipCortex Local Rust (`v0.4.9` Windows)** | **`2.05 ms`** | **`3.67 ms`** | **`0.52 ms`** | Same compiled Rust binary measured over Windows loopback (`127.0.0.1`). |
| **Self-Hosted Local Vector Store (`Mem0/Python`)** | `~15–35 ms` | `~50–80 ms` | `~10–25 ms` | Local Python process + embedding model inference (`~10–25 ms`) + local vector index upsert (`Qdrant/Chroma`). *HipCortex is ~15× to 30× faster than local vector stores.* |
| **Cloud Vector Memory API (`Mem0 Cloud US-East`)** | `~142 ms` | `~310 ms` | `~89 ms` | Public HTTPS round-trip across internet + cloud embedding calculation + remote vector DB upsert. *HipCortex local binary is ~230× to 300× faster than cloud APIs.* |

> [!IMPORTANT]
> **Why HipCortex is sub-millisecond:** We replace expensive dense vector calculation on critical write paths with **precise topological causal graph indexing (`petgraph`) and Dirichlet-Multinomial transition counters**, ensuring zero network I/O and zero LLM embedding delays when saving memory state.

---

## 🧠 Headroom vs. Caveman Mode Token Optimization (`59% – 88% Savings`)

In long autonomous coding sessions (`Claude Code`, `Copilot`, `Antigravity IDE`), full conversation history injection causes **context stuffing**, degraded reasoning, and astronomical token bills.

HipCortex (`WorkingSetBroker` + `TemporalIndexer`) solves this with **Topological Context Budgeting**, verified via `benchmarks/token_reduction_benchmark.py` (`tiktoken cl100k_base`):

| Context Strategy | Input Tokens (Turn 20) | Steady-State Savings (Turns 11–20) | Projected 50-Turn Session Savings | When to Use |
| :--- | :---: | :---: | :---: | :--- |
| **Full History Injection** | `8,861 tokens` | Baseline (`0%`) | Baseline (`~2,308 tok/turn`) | ❌ Default Copilot/Claude behavior |
| **Rolling-10 Window** | `6,772 tokens` | `-23.6%` | `-17.0%` | ⚠️ Forgets early architectural rules |
| **Headroom Mode (`Top-5`)** | **`4,160 tokens`** | **`-62.7%` (`-59% average`)** | **`-84.0%`** | ✅ **Standard balance:** Retains broad context with huge budget headroom |
| **Caveman Mode (`Top-3`)** | **`2,737 tokens`** | **`-69.1%` (`-70% average`)** | **`-88.0%`** | ⚡ **Strict optimization:** Ultra-lean context for high-frequency loops |
| **Proactive Substrate (`live_beliefs`)** | **`700 tokens`** | **`-93.0%`** | **`-96.0%`** | 🤖 **Substrate-as-Mind:** Agent queries pre-merged `CausalTopoGraph` directly |

---

## 🏗️ 6-Layer Cognitive Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        CLIENT / AGENT LAYER                            │
│   (Claude Code, Antigravity IDE, Cursor, Grok Code, Hermes, OpenClaw)  │
└───────────────────────────────────▲────────────────────────────────────┘
                                    │  MCP / HTTP JSON-RPC (`Tier 0` Session)
┌───────────────────────────────────▼────────────────────────────────────┘
│ LAYER 1: WORKING SET BROKER (`WorkingSetBroker` / `SessionContext`)    │
│          Pages active context into Tier 0; manages token budget        │
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 2: TEMPORAL INDEXER (`TemporalIndexer` — 5 Memory Tiers)         │
│          WorkingSet ──► ShortTerm ──► LongTerm ──► Causal ──► Procedural│
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 3: CAUSAL TOPOLOGICAL GRAPH (`CausalTopoGraph` / `petgraph`)     │
│          Directed acyclic & cyclic causal links, Backdoor Adjustment   │
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 4: WORLD MODEL & SIMULATOR (`WorldModelEnhanced` / `MctsSimulator`)│
│          Dirichlet-Multinomial transitions, MCTS `POST /worldmodel/rollout`│
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 5: OMEGA LOOP ENGINE (`LoopEngine` / `SelfModel`)                │
│          Bayesian attribution, surprise calculation, FSM skill compile │
├────────────────────────────────────────────────────────────────────────┤
│ LAYER 6: GRAPH & AUDIT STORAGE (`GraphDatabase` / Merkle SHA-256)      │
│          Tamper-evident Merkle hash chain, durable local SQLite/JSON   │
└────────────────────────────────────────────────────────────────────────┘
```
```

- [ ] **Step 2: Update `BENCHMARK.md` with explicit Caveman Comparison Matrix disclaimers**

Verify and update `BENCHMARK.md` lines `31-43` to ensure exact wording parity with our Caveman Comparison Matrix, explicitly pointing readers to `python benchmarks/token_reduction_benchmark.py` and `python benchmarks/python_benchmark.py --url http://127.0.0.1:3030 -n 200`.

- [ ] **Step 3: Run benchmark verification check**

Run: `python benchmarks/python_benchmark.py --url http://127.0.0.1:3030 -n 50`
Expected: Output showing local `add_p50_ms` between `0.5 ms` and `2.5 ms` and `query_p50_ms` under `1.0 ms`.

- [ ] **Step 4: Commit Task 1 changes**

Run: `git add README.md BENCHMARK.md ; git commit -m "docs(readme): rewrite root README and BENCHMARK with Caveman Comparison Matrix and v0.4.9 architecture"`

---

### Task 2: VS Code Extension & Antigravity IDE Marketplace (`/opsx-apply` Tasks 2.1 & 2.2)

**Files:**
- Modify: `vscode-extension/package.json:1-50`
- Modify: `vscode-extension/README.md:1-150`

**Interfaces:**
- Consumes: `webserver.exe` binary management logic from `vscode-extension/src/server.ts` and `@hipcortex` Copilot chat participant registered in `vscode-extension/src/extension.ts`.
- Produces: Marketplace-ready metadata (`package.json`) and clear Antigravity IDE / VS Code user onboarding documentation (`vscode-extension/README.md`).

- [ ] **Step 1: Update `vscode-extension/package.json` metadata**

Modify `vscode-extension/package.json` top-level fields:
```json
{
  "name": "hipcortex-memory",
  "displayName": "HipCortex Memory Engine & Cognitive OS",
  "description": "Persistent causal topological memory, world model prediction (/worldmodel/rollout), and Headroom token optimization (59-88% savings) for Copilot & Antigravity IDE.",
  "version": "0.4.9",
  "publisher": "farmountain",
  "keywords": [
    "memory",
    "causal",
    "cognitive-os",
    "copilot",
    "antigravity",
    "mcp",
    "ai",
    "agent"
  ]
}
```

- [ ] **Step 2: Rewrite `vscode-extension/README.md` for Zero-Config Onboarding**

Rewrite `vscode-extension/README.md` with:
```markdown
# HipCortex Memory Engine & Cognitive OS for VS Code & Antigravity IDE (`v0.4.9`)

**Give your AI coding assistant (VS Code Copilot & Antigravity IDE) persistent, cross-session causal memory and world-model prediction.**

---

## 🚀 Zero-Config Onboarding (No Rust or Cargo Required!)

When you install `HipCortex Memory Engine & Cognitive OS`, the extension **automatically downloads and runs the standalone `v0.4.9` local Rust binary (`webserver.exe`) in the background** (`~/.hipcortex-vscode/bin/`).

- **Zero dependencies**: No external databases, no Python required, no Docker needed.
- **Local-first & private**: All memories and causal graphs are stored locally on your machine (`~/.hipcortex-vscode/storage`). Zero telemetry.
- **Sub-millisecond speed**: `0.48–0.61 ms p50` write latency over local loopback (`http://127.0.0.1:3030`).

---

## 💬 Interactive `@hipcortex` Chat Commands

Open your GitHub Copilot or Antigravity Chat panel and type `@hipcortex`:

- `@hipcortex health`: Inspect local server status, memory tier counts (`WorkingSet`, `ShortTerm`, `LongTerm`), and Merkle chain audit integrity.
- `@hipcortex add <content>`: Store an explicit architectural decision, user preference, or project constraint into persistent memory.
- `@hipcortex query <query>`: Perform semantic + topological graph retrieval across your workspace history.
- `@hipcortex status`: Display current token optimization mode (`Headroom` vs `Caveman`) and active memory savings.

---

## 🛠️ Language Model Tools (`LM Tools`)

HipCortex automatically registers 6 native tools with the VS Code & Antigravity Language Model API:
1. `hipcortex_search`: Semantic and causal graph search over past project sessions.
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
```

- [ ] **Step 3: Verify extension build syntax**

Run: `cd vscode-extension ; npm run compile`
Expected: Clean compilation with zero TypeScript errors.

- [ ] **Step 4: Commit Task 2 changes**

Run: `git add vscode-extension/package.json vscode-extension/README.md ; git commit -m "docs(extension): rewrite marketplace README for zero-config v0.4.9 onboarding and LM tools"`

---

### Task 3: Universal MCP & Multi-Agent Setup Guide (`/opsx-apply` Task 3.1)

**Files:**
- Modify: `sdk/mcp/README.md:1-250`

**Interfaces:**
- Consumes: Python MCP server implementation from `sdk/python/hipcortex/mcp/server.py` and `cli.py`.
- Produces: Comprehensive multi-agent JSON-RPC configuration blocks across 12 target AI orchestrators.

- [ ] **Step 1: Rewrite `sdk/mcp/README.md` with the 12-Agent Setup Matrix**

Modify `sdk/mcp/README.md` to include:
```markdown
# HipCortex Universal MCP Server & Multi-Agent Setup Guide (`v0.4.9`)

HipCortex exposes a native **Model Context Protocol (MCP)** server (`hipcortex.mcp.server`) and REST API (`http://127.0.0.1:3030`) that gives autonomous AI agents multi-tier causal memory, world-model rollout prediction, and `Headroom Mode` token reduction (`59–88% savings`).

---

## 🚀 Instant CLI Setup (`hipcortex setup`)

If you have Python installed, our CLI automatically detects your installed agents and writes the exact MCP configurations:

```bash
pip install hipcortex
hipcortex setup --mode headroom --url http://127.0.0.1:3030
```

---

## 📋 Exact Copy-Paste Configurations (12 Autonomous Agents)

### 1. Claude Code (`claude mcp add`)
Run in your terminal:
```bash
claude mcp add hipcortex python -m hipcortex.mcp.server --mode headroom
```
Or edit `~/.claude/mcp.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 2. Cursor IDE (`.cursor/mcp.json`)
Create or edit `.cursor/mcp.json` in your workspace root (or globally in `~/.cursor/mcp.json`):
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 3. Windsurf IDE (`~/.codeium/windsurf/mcp_config.json`)
Add to `~/.codeium/windsurf/mcp_config.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 4. Grok Code (`~/.grok/mcp.json`)
Add to your Grok Code configuration directory `~/.grok/mcp.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030", "OPTIMIZATION_MODE": "headroom" }
    }
  }
}
```

### 5. Hermes Agent (`~/.hermes/mcp_config.json`)
Add to `~/.hermes/mcp_config.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 6. OpenClaw Orchestrator (`~/.openclaw/mcp.json`)
Add to `~/.openclaw/mcp.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 7. Cline / RooCode (VS Code Extension MCP Settings)
Open VS Code $\rightarrow$ `Cline Settings` (or `RooCode Settings`) $\rightarrow$ `MCP Servers` $\rightarrow$ Add New:
- **Server Name**: `hipcortex`
- **Command**: `python`
- **Arguments**: `["-m", "hipcortex.mcp.server", "--mode", "headroom"]`
- **Environment**: `{"HIPCORTEX_URL": "http://127.0.0.1:3030"}`

### 8. OpenAI Codex CLI (`codex --mcp-server`)
Pass via command line or `~/.codex/config.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"]
    }
  }
}
```

### 9. Aider AI Pair Programmer (`--mcp-server`)
Launch Aider with HipCortex MCP:
```bash
aider --mcp-server "python -m hipcortex.mcp.server --mode headroom"
```

### 10. Gemini CLI & Antigravity IDE (`~/.gemini/antigravity-ide/mcp/`)
Place `hipcortex.json` into your Antigravity IDE `mcp` server directory:
```json
{
  "command": "python",
  "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
  "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
}
```

### 11. Amazon Q Developer (`~/.amazonq/mcp.json`)
Add to `~/.amazonq/mcp.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 12. Direct HTTP JSON-RPC / REST Mode
For lightweight custom harnesses, call the local server directly:
```bash
curl -X POST http://127.0.0.1:3030/memory/add \
  -H "Content-Type: application/json" \
  -d '{"actor": "agent", "action": "decided", "target": "use sqlite over postgres", "record_type": "Symbolic"}'
```
```

- [ ] **Step 2: Commit Task 3 changes**

Run: `git add sdk/mcp/README.md ; git commit -m "docs(mcp): rewrite universal setup guide across 12 autonomous coding agents"`

---

### Task 4: Python and TypeScript SDK Documentation Parity (`/opsx-apply` Tasks 4.1 & 4.2)

**Files:**
- Modify: `sdk/python/README.md:1-120`
- Modify: `sdk/typescript/README.md:1-120`

**Interfaces:**
- Consumes: Python client classes `Client` and `AsyncClient` from `sdk/python/hipcortex/__init__.py` and TypeScript client from `sdk/typescript/src/index.ts`.
- Produces: SDK README files demonstrating multi-tier ingestion (`record_type`), `can_execute()` health checks, and `POST /worldmodel/rollout` trajectory prediction.

- [ ] **Step 1: Rewrite `sdk/python/README.md`**

Update `sdk/python/README.md` with:
```python
from hipcortex import Client

client = Client(base_url="http://127.0.0.1:3030")

# 1. Multi-Tier Memory Ingestion (5 verified memory tiers)
client.add_memory(
    actor="agent",
    action="configured",
    target="jwt_token_ttl=3600",
    record_type="Working",  # Mapped natively to Temporal tier
    confidence=1.0
)

client.add_memory(
    actor="agent",
    action="established",
    target="All database migrations must run inside transactions",
    record_type="Semantic", # Mapped natively to Symbolic/LongTerm tier
    confidence=1.0
)

# 2. Check SelfModel Execution Capacity Gates before running risky actions
if client.can_execute("rollout"):
    print("Health check passed — engine ready for simulation")

# 3. World Model Trajectory Rollout Prediction (POST /worldmodel/rollout)
rollout_result = client.rollout(
    initial_state={"db_status": "locked", "active_tx": 1},
    actions=["rollback_tx", "release_lock", "retry_migration"]
)
print("Predicted outcome:", rollout_result)
```

- [ ] **Step 2: Rewrite `sdk/typescript/README.md`**

Update `sdk/typescript/README.md` with matching TypeScript examples:
```typescript
import { HipCortexClient } from 'hipcortex-sdk';

const client = new HipCortexClient({ baseUrl: 'http://127.0.0.1:3030' });

async function main() {
  // 1. Multi-Tier Memory Ingestion
  await client.addMemory({
    actor: 'agent',
    action: 'compiled',
    target: 'src/web_server.rs',
    record_type: 'Procedural',
    confidence: 1.0,
  });

  // 2. SelfModel Execution Gate Check
  const canRun = await client.canExecute('rollout');
  if (canRun) {
    // 3. Multi-step World Model Rollout
    const prediction = await client.rollout({
      initial_state: { memory_tier: 'WorkingSet', token_budget: 4000 },
      actions: ['evict_stale', 'compact_graph', 'query_top3'],
    });
    console.log('MCTS Trajectory Prediction:', prediction);
  }
}
main();
```

- [ ] **Step 3: Commit Task 4 changes**

Run: `git add sdk/python/README.md sdk/typescript/README.md ; git commit -m "docs(sdk): add multi-tier ingestion, execution gates, and rollout examples to python and typescript docs"`

---

### Task 5: Verification, Git Push & Dissemination Checklist (`/opsx-apply` Tasks 5.1, 5.2, & 5.3)

**Files:**
- Check: All modified markdown files (`README.md`, `BENCHMARK.md`, `vscode-extension/README.md`, `sdk/mcp/README.md`, `sdk/python/README.md`, `sdk/typescript/README.md`).

**Interfaces:**
- Consumes: All completed documentation commits from Tasks 1–4.
- Produces: Remote repository sync (`git push origin main`) and clean multi-channel publishing checklist.

- [ ] **Step 1: Run comprehensive git diff check and verify working tree status**

Run: `git status -s`
Expected: Clean status (all documentation changes committed).

- [ ] **Step 2: Push all commits to remote GitHub `main` branch**

Run: `git push origin main`
Expected: Output showing successful push to `origin/main` (`main -> main`).

- [ ] **Step 3: Output Multi-Channel Dissemination & Publishing Checklist**

Produce the exact terminal summary table for user dissemination across platforms:
1. **GitHub Repository**: Updated automatically via `git push origin main`.
2. **PyPI (`hipcortex` v0.4.9)**: Run `cd sdk/python && python -m build && twine upload dist/*`.
3. **npm (`hipcortex-sdk` v0.4.9)**: Run `cd sdk/typescript && npm run build && npm publish --access public`.
4. **VS Code Marketplace / Open VSIX (`hipcortex-memory` v0.4.9)**: Run `cd vscode-extension && vsce package && vsce publish`.

- [ ] **Step 4: Mark `rewrite-docs-v049` tasks completed and suggest `/opsx-archive`**

Update `openspec/changes/rewrite-docs-v049/tasks.md` marking all checkboxes `[x]` and output completion summary.
