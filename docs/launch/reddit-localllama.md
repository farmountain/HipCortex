# r/LocalLLaMA post

**Subreddit:** r/LocalLLaMA  
**Title:** I built a Rust AI memory engine that's 295× faster than Mem0 cloud, with temporal decay and a causal world model. Open source, zero deps.

---

## Post body

Hey r/LocalLLaMA,

I've been frustrated with existing AI memory systems for a while. They all treat memory as a retrieval problem — store embeddings, query by cosine similarity. That's fine for chatbots, but it breaks down for real agents that need to:

- Remember things for days/weeks, not just a session
- Know that a memory from 3 months ago is less relevant than one from yesterday
- Detect when two memories contradict each other
- Prove to regulators that a user's data was actually deleted (GDPR)

So I built **HipCortex** — a Rust memory engine that treats memory as *cognition*, not retrieval.

**Key features:**

🕐 **Temporal decay** — each memory has a decay factor and half-life. You control how fast memories fade. Critical memories persist; stale context prunes automatically. This is how biological memory works.

🌍 **Causal world model** — the engine maintains an internal state model using Dirichlet-Multinomial transitions and Kalman filtering. Your agent's memory can *predict* and *simulate*, not just recall.

🔍 **POST /memory/search** — cosine similarity over stored embeddings (if present), falling back to keyword matching. Bring your own embeddings (OpenAI, Ollama, local model — anything).

🔒 **Tamper-evident audit log** — Merkle-chained, every write SHA-256 hashed. GDPR `DELETE /memory/forget/:actor` propagates through temporal store, symbolic graph, and audit log atomically.

⚡ **Speed** — 0.48ms p50 write latency. The benchmark vs Mem0 cloud is local vs cloud (not apples-to-apples) but the local latency is the point: sub-millisecond durable memory for real-time agent loops.

**It works with your existing stack:**

```python
pip install hipcortex

# LangChain
from hipcortex.langchain_memory import HipCortexMemory
memory = HipCortexMemory(session_id="user-42", url="http://localhost:3030")

# AutoGen
from hipcortex.adapters.autogen import HipCortexAutoGenMemory
mem = HipCortexAutoGenMemory(client=client, agent_id="my-agent")
agent.register_hook("process_message_before_send", mem.on_message_sent)

# CrewAI
from hipcortex.adapters.crewai import HipCortexRememberTool, HipCortexRecallTool
tools = [HipCortexRememberTool(client=client), HipCortexRecallTool(client=client)]
```

**Self-hosted, single binary:**

```bash
cargo build --release --bin webserver \
  --no-default-features --features "web-server,petgraph_backend"
# ~4MB binary, zero external dependencies
```

Or Fly.io deploy in 5 minutes (fly.toml included in the repo).

**GitHub:** https://github.com/farmountain/HipCortex  
**Benchmark details:** https://github.com/farmountain/HipCortex/blob/main/BENCHMARK.md

Would love feedback on: (1) what memory features matter most for your use case, (2) whether the temporal decay / causal world model is useful or overkill for you, (3) any integrations you need that aren't there yet.

---

## r/MachineLearning cross-post variant

**Title:** [Project] HipCortex: Rust memory engine for AI agents with temporal decay, causal world modeling, and Merkle-chained audit log

**Body:** Add technical depth — reference the arXiv whitepaper (docs/whitepaper.md), the compression hierarchy thesis, the coherence checker architecture. Target ML researchers building cognitive architectures.

---

## LinkedIn post

**Target:** AI engineers, founders building on LangChain/AutoGen

AI agents forget everything between sessions. Most "memory" solutions are just embedding search — fast retrieval, no cognition.

I open-sourced HipCortex: a Rust memory engine that gives agents memory that actually works like memory.

What's different:
→ Temporal decay: memories fade at configurable rates. Stale context prunes itself.
→ Causal world model: the engine builds an internal state model, not just a corpus.
→ 295× faster than cloud alternatives at 0.48ms p50 (local REST, zero deps)
→ GDPR right-to-forget as a first-class REST endpoint
→ Works with LangChain, LlamaIndex, AutoGen, CrewAI out of the box

Open source (Apache 2.0): https://github.com/farmountain/HipCortex

What memory pain points are you hitting with your agents? Curious what I should prioritize next.

---

## Demand threshold gates (review after 2 weeks)

| Signal | Threshold | Action |
|--------|-----------|--------|
| GitHub stars | > 300 | Proceed to Fly.io hosted SaaS |
| GitHub stars | > 800 | Invest in SOC 2 + enterprise sales |
| Issues requesting feature X | > 5 | Prioritize X in next sprint |
| PyPI installs/week | > 500 | Publish to PyPI properly |
| "How do I self-host?" issues | > 3 | Improve DEPLOY.md / Docker Hub image |
| "Does it support X database?" | > 3 | Prioritize that backend |
| DMs requesting enterprise | > 2 | Start enterprise outreach |
| Stars from .edu domains | > 10 | Pursue research partnerships |
