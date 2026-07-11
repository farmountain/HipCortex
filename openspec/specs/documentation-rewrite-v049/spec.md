# documentation-rewrite-v049 Specification

## Purpose
Defines the specifications, benchmarks, exact token reduction metrics, latency matrix, and universal onboarding documentation for HipCortex v0.4.9 as an Autonomous Cognitive OS across all repositories, IDEs, MCP servers, and SDKs.

## Requirements

### Requirement: Root Repository README Rigorous Narrative and Caveman Matrix
The root `README.md` file MUST accurately reflect the `v0.4.9` Autonomous Cognitive OS architecture and present the **Caveman Comparison Matrix** (`0.48–0.61 ms local Rust vs 15–35 ms local Python vector vs 142 ms cloud API`) and exact `tiktoken cl100k_base` benchmark tables (`Headroom Mode Top-5 at -59% to -84%`, `Caveman Mode Top-3 at -70% to -88%`).

#### Scenario: Developer reviews root README on GitHub
- **WHEN** a developer or automated agent navigates to the root repository `README.md`
- **THEN** they see version `v0.4.9` badges, the 6-layer Cognitive Architecture ASCII diagram, the empirical token savings tables (`Headroom` and `Caveman` modes), and the explicit Caveman Comparison Matrix disclaimers explaining local vs cloud latency differences without marketing over-claims.

### Requirement: VS Code Extension Marketplace and Antigravity IDE Zero-Config Onboarding
The `vscode-extension/README.md` and `vscode-extension/package.json` MUST clearly document zero-config local server binary management (`webserver.exe` in `~/.hipcortex-vscode`), interactive `@hipcortex` chat commands (`add`, `query`, `health`, `status`), and Language Model Tools (`hipcortex_search`, `hipcortex_health`, `hipcortex_predict`).

#### Scenario: End user reviews VS Code Extension Marketplace description
- **WHEN** a user visits the VS Code Marketplace or Open VSIX Registry for `hipcortex-memory` (`v0.4.9`)
- **THEN** they see clear instructions on how `@hipcortex` auto-bootstraps the local binary without requiring Rust/Cargo installation, and how `Headroom Mode` token optimization cuts Copilot token consumption by `59% to 88%`.

### Requirement: Universal MCP Multi-Agent Setup Guide
The `sdk/mcp/README.md` MUST provide exact, copy-pasteable JSON configuration snippets and CLI commands (`hipcortex setup`) across the full spectrum of modern autonomous agents: Claude Code, Cursor, Windsurf, Grok Code, Hermes, OpenClaw, Cline, RooCode, OpenAI Codex CLI, Aider, Gemini CLI, and Amazon Q Developer.

#### Scenario: Autonomous agent or user configures MCP server for Grok Code or Hermes or OpenClaw
- **WHEN** a developer consults `sdk/mcp/README.md` to connect HipCortex to their desktop AI orchestrator (Grok Code, Hermes, OpenClaw, Cursor, Windsurf, Cline, RooCode)
- **THEN** they find an exact JSON-RPC / stdio MCP configuration block (`hipcortex.mcp_server`) and environment variables (`OPTIMIZATION_MODE: headroom`) ready for immediate adoption.

### Requirement: Python and TypeScript SDK Documentation Parity
The `sdk/python/README.md` and `sdk/typescript/README.md` MUST illustrate multi-tier memory insertion (`WorkingSet`, `ShortTerm`, `LongTerm`, `Causal`, `Procedural`), self-model capability gates (`can_execute`), and proactive CodeAct execution loops.

#### Scenario: Developer builds python or typescript agent using HipCortex client
- **WHEN** a developer reads the Python or TypeScript SDK README
- **THEN** they see working examples showing how to insert tiered memories, check `can_execute` capacity gates, and run FSM skill execution loops.

### Requirement: Continuous CI/CD Token Optimization & Latency Verification
The codebase MUST include an automated GitHub Actions verification workflow (`.github/workflows/cognitive_os_token_verification.yml`) and verification harness that validates `v0.4.9` Headroom and Caveman token compression (`-59% to -88%`) and local Rust binary latency benchmarks against regressions on every push and pull request.

#### Scenario: CI pipeline executes token optimization verification
- **WHEN** a pull request or commit is pushed to `main`
- **THEN** the CI workflow executes the Headroom & Caveman token verification harness (`python test_e2e_cognitive_os_live.py` / `tests/verify_token_optimization.py`) to confirm zero drift on compression ratios and local memory latencies.

### Requirement: Interactive VS Code Extension Dynamic Mode Toggle (`@hipcortex mode <headroom|caveman>`)
The `vscode-extension/src/extension.ts` chat participant and commands MUST support `@hipcortex mode headroom` and `@hipcortex mode caveman` interactive commands, allowing users to dynamically switch token optimization search depths (`Top-5` vs `Top-3`) without restarting VS Code, reflecting the active mode (`[Headroom]` or `[Caveman]`) directly in the status bar and retrieval telemetry.

#### Scenario: Developer toggles optimization mode via chat command
- **WHEN** a user types `@hipcortex mode caveman` inside the VS Code Copilot Chat
- **THEN** the extension switches `hipcortex.optimizationMode` to `caveman`, limits queries to `limit: 3` (`Top-3`), updates the status bar label to indicate `[Caveman]`, and confirms the switch in the chat output.
