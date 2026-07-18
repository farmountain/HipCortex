# Show HN: HipCortex — 0.6ms AI memory for agents. MCP for Cursor/Claude Code. Rust, ARM64, zero deps.

**URL to submit:** https://github.com/farmountain/HipCortex

---

## Post text

Show HN: HipCortex — Rust AI memory with metacognitive intelligence layer, MCP for Cursor/Claude Code

I built HipCortex because every memory system treats memory as retrieval. HipCortex treats it as cognition: temporal decay, causal world modeling, cross-module coherence, and metacognitive self-awareness.

What's different:
- **0.48–0.61ms p50 write latency** on Linux (≈2ms on Windows loopback) — SHA-256 Merkle audit trail included
- **Intelligence layer** — Self-Model (metacognitive health + decision engine), World-Model Enhanced (Dirichlet-Multinomial transitions, Kalman entity tracking, causal do-calculus), Coherence Checker (5 inconsistency types, 3 resolution strategies, 4 mathematical invariants with synchronous write-gating)
- **Proactive agent harness** — `hipcortex install --mode proactive` forces substrate-first loop (`GET /memory/live_beliefs` before frontier tokens; up to ~93% context reduction vs full history)
- **MCP server** for Cursor, Claude Code, Windsurf — one install → persistent memory across sessions
- **GDPR right-to-forget**: `DELETE /memory/forget/:actor` — atomic across temporal + symbolic + audit
- **ARM64 binary** — ~4MB, Raspberry Pi 5, Jetson, AWS Graviton, M1/M2/M4 Mac
- **Zero dependencies** — single binary, no database required (petgraph default)
- **Framework integrations**: LangChain, LlamaIndex, AutoGen 0.4, CrewAI, Continue.dev, Flowise, Dify
- **45+ REST endpoints** — health, prediction, counterfactual reasoning, coherence resolution, live_beliefs
- **npm + PyPI + VS Code extension**: `npm i hipcortex` / `pip install hipcortex`

**Metacognitive intelligence (what makes this different):**
```sh
# Self-model: check system health, get execution decisions
curl https://hipcortex.fly.dev/self/health
# World-model: predict next state, counterfactual reasoning
curl -X POST https://hipcortex.fly.dev/worldmodel/predict \
  -d '{"state":"idle","action":"process"}'
# Coherence: detect and auto-resolve memory inconsistencies
curl -X POST https://hipcortex.fly.dev/coherence/check
```

**Deploy:**
- Binary: `cargo build --release --bin webserver --features "web-server,petgraph_backend"`
- Fly.io: `fly launch && fly deploy` (fly.toml included)
- Docker: `docker run -p 3030:3030 hipcortex:latest`
- npm: `npm i hipcortex` — TypeScript SDK v0.2.0
- Python: `pip install hipcortex` — v0.2.0 on PyPI

The architecture reverse-engineers what AGI memory actually requires: not retrieval, but continuous reality compression with metacognitive self-awareness. Whitepaper: docs/whitepaper.md. Full intelligence docs: INTELLIGENCE.md.

Happy to answer questions on the Rust design, intelligence layer architecture, or the AGI framing.

GitHub: https://github.com/farmountain/HipCortex
Benchmark methodology: https://github.com/farmountain/HipCortex/blob/main/BENCHMARK.md

---

## Submission checklist
- [ ] All 3 install paths tested (binary, pip git+, MCP)
- [ ] hipcortex.fly.dev/health returns ok
- [ ] MCP server works in Cursor with add_memory tool
- [ ] ARM64 binary downloadable from releases (wait for CI)
- [ ] Post Monday or Tuesday 9am ET
- [ ] Cross-post to r/LocalLLaMA same day

## Expected comments to prepare for

**"How does this compare to MemGPT?"**
MemGPT relies on the LLM itself to manage memory tiers via function calls — nondeterministic and expensive. HipCortex provides a deterministic substrate below the LLM: memory management happens in Rust, not in the prompt.

**"The 295× comparison is misleading (local vs cloud)"**
Fair. The local vs cloud gap is network I/O. The more meaningful comparison is: HipCortex adds 0.48ms for durable, causally-structured, tamper-evident memory. An in-process dict adds 0.002ms for none of those properties. The 240× overhead is the cost of persistence + auditability.

**"Why Rust?"**
Four reasons: (1) edge deployment — 4MB binary with no Python runtime; (2) embedded targets — ARM64, Jetson, microcontrollers; (3) memory safety — no GC pauses in real-time agent loops; (4) compile-time feature gating — ship exactly the subsystems you need.

**"Is this production-ready?"**
The core (MemoryStore, TemporalIndexer, SymbolicStore) is solid. The world model and coherence checker are Alpha — functional but not battle-tested at scale. Use the petgraph_backend for production (no external deps); the Neo4j/Postgres backends are feature-gated.
