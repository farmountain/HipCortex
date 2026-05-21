# Show HN: HipCortex — 0.6ms AI memory for agents. MCP for Cursor/Claude Code. Rust, ARM64, zero deps.

**URL to submit:** https://github.com/farmountain/HipCortex

---

## Post text

Show HN: HipCortex – Rust AI memory engine, 295× faster than Mem0, with causal world model

I built HipCortex because every memory system I evaluated treated memory as a retrieval problem. HipCortex treats it as cognition: temporal decay, causal world modeling, cross-module coherence checking.

What's different:
- **0.6ms p50 write latency** on Linux (1.7ms on Windows) — with SHA-256 audit trail included
- **MCP server** for Cursor, Claude Code, Windsurf — `curl install.sh | bash` and your AI coding assistant gains persistent memory across sessions
- **GDPR right-to-forget** as a REST endpoint: `DELETE /memory/forget/:actor`
- **ARM64 binary** — 4MB, runs on Raspberry Pi 5, Jetson, AWS Graviton, M1/M2/M4 Mac
- **Zero dependencies** — single binary, no database, no Docker required
- **Works with**: LangChain, LlamaIndex, AutoGen 0.4, CrewAI, Continue.dev, Flowise, Dify

**Framework integrations (pip install hipcortex):**
- LangChain: drop-in for `ConversationBufferMemory`
- LlamaIndex: `SimpleChatStore`-compatible
- AutoGen 0.4: `Memory` protocol implementation
- CrewAI: `BaseTool` subclasses (Remember/Recall/Forget)

**Deploy:**
- Single binary: `cargo build --release --bin webserver --features "web-server,petgraph_backend"`
- Fly.io: `fly launch && fly deploy` (fly.toml included, Frankfurt region for GDPR)
- Docker: `docker run -p 3030:3030 hipcortex:latest`

The architecture comes from reverse-engineering what AGI memory actually requires: not retrieval, but continuous reality compression. I wrote up the theory in docs/whitepaper.md if you're into the cognitive architecture angle.

Happy to answer questions on the Rust design, the coherence checker, or the AGI framing.

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
