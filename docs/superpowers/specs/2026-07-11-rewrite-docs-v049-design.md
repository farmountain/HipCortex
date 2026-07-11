# HipCortex v0.4.9 Documentation Rewrite & Codebase Alignment Design (`rewrite-docs-v049`)

**Date:** 2026-07-11  
**Status:** Approved (`Approach A: The Rigorous Caveman & Headroom Engineering Matrix`)  
**Target:** `README.md`, `BENCHMARK.md`, `vscode-extension/README.md`, `vscode-extension/package.json`, `sdk/mcp/README.md`, `sdk/python/README.md`, `sdk/typescript/README.md`

---

## 1. Context & Motivation

HipCortex has reached `v0.4.9`, featuring:
- **Autonomous Cognitive OS & Persistent Causal Substrate**: A local-first, zero-dependency `4 MB` Rust binary (`webserver.exe`).
- **Multi-Tier Memory Architecture**: Ingestion across `WorkingSet`, `ShortTerm`, `LongTerm`, `Causal`, and `Procedural` memory tiers (`MemoryType` enum with aliases verified in `src/web_server.rs:2927-2936`).
- **World Model Prediction (`/worldmodel/rollout`)**: Monte Carlo Tree Search (`MctsSimulator`) over empirical Dirichlet transition matrices (`TransitionModel`) and execution gates (`can_execute`).
- **Headroom vs. Caveman Token Optimization**: Verified in `benchmarks/token_reduction_benchmark.py` delivering `-59% to -84%` token savings (`Top-5 Headroom Mode`) and `-70% to -88%` (`Top-3 Caveman Mode`).

Our public documentation on GitHub, the VS Code / Antigravity IDE Marketplace, and the MCP Multi-Agent ecosystem contains outdated `v0.3.0` badges, lacks multi-agent setup configurations, and states `300× faster than cloud memory APIs (Mem0 ~142ms)` without explicitly detailing the network round-trip and dense vector embedding comparison gap. To establish unassailable engineering rigor, we adopt **Approach A: The Rigorous Caveman & Headroom Engineering Matrix**.

---

## 2. Architecture & Design Blueprint

### A. Root Repository `README.md` & `BENCHMARK.md`
1. **Badges & Versioning**: Update all version shields and badges across the root `README.md` to `v0.4.9`.
2. **The 6-Layer Cognitive Architecture Diagram**: Render a clean ASCII diagram illustrating how `SessionContext` (`Tier 0`) connects to `WorkingSetBroker`, `CausalTopoGraph`, `WorldModelEnhanced`, `LoopEngine`, and `GraphDatabase`.
3. **The Caveman Comparison Matrix (`0.48–0.61 ms Local Rust vs 15–35 ms Local Vector vs 142 ms Cloud`)**:
   We will insert this exact, fact-checked comparison matrix into `README.md` and `BENCHMARK.md`:

| System / Substrate | Write Median (`add_p50`) | Write 95th (`add_p95`) | Query Median (`query_p50`) | What It Does & Why |
| :--- | :---: | :---: | :---: | :--- |
| **HipCortex Local Rust (`v0.4.9` Linux)** | **`0.61 ms`** (`0.48 ms` bare) | **`1.1 ms`** | **`0.23 ms`** | Compiled `4 MB` Rust binary. Zero public network RTT. Indexes causal topological relationships (`petgraph`) + SHA-256 Merkle audit chain without heavy dense vector inference. |
| **HipCortex Local Rust (`v0.4.9` Windows)** | **`2.05 ms`** | **`3.67 ms`** | **`0.52 ms`** | Same compiled Rust binary measured over Windows loopback (`127.0.0.1`). |
| **Self-Hosted Local Vector Store (`Mem0/Python`)** | `~15–35 ms` | `~50–80 ms` | `~10–25 ms` | Local Python process + embedding model inference (`~10–25 ms`) + local vector index upsert (`Qdrant/Chroma`). *HipCortex is ~15× to 30× faster than local vector stores.* |
| **Cloud Vector Memory API (`Mem0 Cloud US-East`)** | `~142 ms` | `~310 ms` | `~89 ms` | Public HTTPS round-trip across internet + cloud embedding calculation + remote vector DB upsert. *HipCortex local binary is ~230× to 300× faster than cloud APIs.* |

4. **Empirical Token Reduction Tables (`cl100k_base` Tokenizer)**:
   We will publish the exact benchmark numbers from `token_reduction_benchmark.py`:

| Context Strategy | Input Tokens (Turn 20) | Steady-State Savings (Turns 11–20) | Projected 50-Turn Session Savings | Target Operating Mode |
| :--- | :---: | :---: | :---: | :--- |
| **Full History Injection** | `8,861 tokens` | Baseline (`0%`) | Baseline (`~2,308 tok/turn`) | ❌ Standard Copilot/Claude behavior |
| **Headroom Mode (`Top-5`)** | **`4,160 tokens`** | **`-62.7%` (`-59% average`)** | **`-84.0%`** | ✅ **Default:** Maximum context headroom without prompt stuffing |
| **Caveman Mode (`Top-3`)** | **`2,737 tokens`** | **`-69.1%` (`-70% average`)** | **`-88.0%`** | ⚡ **Strict:** Ultra-lean context for high-frequency code loops |
| **Proactive Substrate (`live_beliefs`)** | **`700 tokens`** | **`-93.0%`** | **`-96.0%`** | 🤖 **Substrate-as-Mind:** Direct `CausalTopoGraph` memory paging |

---

### B. VS Code & Antigravity IDE Marketplace (`vscode-extension/README.md` & `package.json`)
1. **Metadata Optimization (`package.json`)**: Ensure `displayName: "HipCortex Memory Engine & Cognitive OS"`, `version: "0.4.9"`, and keywords target `@hipcortex`, `memory`, `causal`, `world-model`, `copilot`, `antigravity`.
2. **Zero-Config Onboarding (`README.md`)**: Explain how the extension auto-discovers or downloads the `webserver.exe` (`v0.4.9`) binary in the background—no Rust or Cargo installation required.
3. **Interactive Tools (`hipcortex_search`, `hipcortex_health`, `hipcortex_predict`, `hipcortex_rollout`)**: Show concrete `@hipcortex` chat commands (`add`, `query`, `status`) and how `Headroom Mode` cuts Copilot token usage by up to 88%.

---

### C. Universal MCP & Multi-Agent Setup Guide (`sdk/mcp/README.md`)
We will provide exact, copy-pasteable JSON (`mcpServers`) configuration blocks and CLI onboarding commands across 12 modern autonomous agents:
1. **Claude Code** (`~/.claude/mcp.json` or `claude mcp add`)
2. **Cursor IDE** (`.cursor/mcp.json` or Cursor Settings $\rightarrow$ MCP)
3. **Windsurf IDE** (`~/.codeium/windsurf/mcp_config.json`)
4. **Grok Code** (`~/.grok/mcp.json`)
5. **Hermes Agent** (`~/.hermes/mcp_config.json`)
6. **OpenClaw Orchestrator** (`~/.openclaw/mcp.json`)
7. **Cline / RooCode** (`VS Code Extension Settings -> MCP Servers`)
8. **OpenAI Codex CLI / Aider** (`--mcp-server` flags)
9. **Gemini CLI / Antigravity SDK** (`~/.gemini/antigravity-ide/mcp/`)
10. **Amazon Q Developer** (`~/.amazonq/mcp.json`)

Exact JSON Template provided for stdio connections:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": {
        "HIPCORTEX_URL": "http://127.0.0.1:3030",
        "OPTIMIZATION_MODE": "headroom"
      }
    }
  }
}
```

---

### D. Python & TypeScript SDK Documentation Parity (`sdk/python/README.md`, `sdk/typescript/README.md`)
Provide clear code snippets illustrating:
1. **Multi-Tier Ingestion**: Adding records with explicit `record_type="Working"`, `"ShortTerm"`, `"Semantic"`, `"Procedural"`, `"Reflexion"`.
2. **Execution Capacity Gates**: Checking `api.can_execute("rollout")` or `api.can_execute("code_edit")` before dispatching actions.
3. **World Model Rollouts**: Invoking `POST /worldmodel/rollout` with `initial_state` and candidate `actions` to predict best trajectories (`MctsSimulator`).

---

## 3. Implementation Checklist & Verification

1. Modify `README.md` and `BENCHMARK.md` (`Group 1`).
2. Modify `vscode-extension/package.json` and `vscode-extension/README.md` (`Group 2`).
3. Modify `sdk/mcp/README.md` (`Group 3`).
4. Modify `sdk/python/README.md` and `sdk/typescript/README.md` (`Group 4`).
5. Perform Spec Self-Review and check all markdown syntax (`Group 5`).
6. Commit changes (`git commit -m "docs: rewrite v0.4.9 cognitive os documentation across repo, vscode marketplace, mcp servers, and sdks"` and `git push origin main`).
7. Output Multi-Channel Dissemination Guide (`npm publish`, `vsce publish`, `pypi`).
