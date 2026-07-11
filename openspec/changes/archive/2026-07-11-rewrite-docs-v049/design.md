## Context

HipCortex v0.4.9 represents a major architectural leap from early sub-ms memory prototypes (`v0.1`–`v0.3`) into a full-fledged **Autonomous Cognitive OS and Causal Substrate (`0.48 ms p50 writes, 59-88% token savings`)**. Our public documentation currently suffers from fragmentation, unverified marketing comparisons (`300× faster without stating the network/embedding comparison gap`), and missing onboarding copy for modern agents (`Grok Code`, `Hermes`, `OpenClaw`, `Windsurf`, `Cline`, `RooCode`, `Amazon Q`, `Gemini CLI`). 

To ensure our documentation stands up to the most rigorous, skepticism-driven engineering audit (`Headroom` and `Caveman` modes), we must structure all README files and marketplace descriptions with exact, auditable benchmark figures, ASCII architectural charts, and universal multi-agent JSON-RPC / MCP configurations.

## Goals / Non-Goals

**Goals:**
- Rewrite `README.md` (root GitHub repo) with verified benchmark tables, the 6-layer Cognitive Architecture diagram, and the **Caveman Comparison Matrix** (`0.48–0.61 ms local vs 15–35 ms local vector vs 142 ms cloud`).
- Update `vscode-extension/README.md` and `vscode-extension/package.json` to clearly communicate zero-config background `v0.4.9` binary management (`webserver.exe`), interactive `@hipcortex` chat commands, and `Headroom Mode` token reduction in Copilot.
- Expand `sdk/mcp/README.md` into the definitive Universal MCP setup reference covering all 12 major autonomous coding agents and desktop orchestrators.
- Update `sdk/python/README.md` and `sdk/typescript/README.md` with multi-tier memory state (`WorkingSet`, `ShortTerm`, `LongTerm`, `Causal`, `Procedural`) and FSM skill compilation examples.
- Commit to remote GitHub repo (`git commit -m "docs: rewrite v0.4.9 cognitive os documentation across repo, vscode marketplace, mcp servers, and sdks"` and `git push origin main`).

**Non-Goals:**
- Modifying core Rust memory/world-model logic or API schemas (which are already verified and cleanly passing all tests in `v0.4.9`).

## Decisions

1. **Adopt the Caveman Comparison Matrix Across All Surfaces**
   - *Rationale*: Stating `300× faster than cloud memory APIs (Mem0 ~142ms)` without mentioning that `0.48 ms` is local loopback (`127.0.0.1`) while `142 ms` is cloud REST (`mem0.ai`) invites skepticism. By explicitly detailing the breakdown—`0.48–0.61 ms local Rust` (~300× faster than cloud vector APIs due to zero public network RTT + zero cloud embedding bottlenecks; ~15–30× faster than self-hosted local Python vector databases due to compiled Rust + `petgraph` causal indexing + SHA-256 Merkle chains)—we achieve unassailable engineering credibility.
2. **Standardize on exact `tiktoken cl100k_base` Token Reduction Numbers**
   - *Rationale*: We use the exact figures verified by `benchmarks/token_reduction_benchmark.py`: `Headroom Mode (Top-5)` delivers `-59% steady-state savings (-84% at 50 turns)`, while `Caveman Mode (Top-3)` delivers `-70% steady-state savings (-88% at 50 turns)` and `Proactive Substrate Harness` (`live_beliefs`) delivers `-93% steady-state savings`.
3. **Universal Agent Setup Matrix in `sdk/mcp/README.md`**
   - *Rationale*: Autonomous agents use slightly different configuration paths and transport models (`stdio` vs `http/sse`). Providing exact JSON configuration blocks for `Claude Code`, `Cursor`, `Windsurf`, `Grok Code`, `Hermes`, `OpenClaw`, `Cline`, `RooCode`, `Codex`, `Aider`, `Gemini CLI`, and `Amazon Q` makes onboarding instantaneous (`hipcortex setup`).

## Risks / Trade-offs

- [Risk] Outdated shields/badges pointing to `v0.3.0` in existing markdown docs → *Mitigation*: Update all shields across all READMEs to `v0.4.9`.
- [Risk] Marketplace package description truncation limits → *Mitigation*: Ensure `vscode-extension/package.json` description is concise (`<= 200 characters`) while packing key value propositions (`Persistent causal topological memory, world model prediction, and Headroom token optimization (59-88% savings) for Copilot & Antigravity IDE.`).
