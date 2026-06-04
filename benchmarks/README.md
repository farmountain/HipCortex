# HipCortex Benchmarks

## Latency Benchmark

Measures HipCortex REST API add/query latency vs in-process dict baseline.

```bash
# Start server first
cargo run --bin webserver --features "web-server,petgraph_backend"

# Run benchmark
python benchmarks/python_benchmark.py --url http://localhost:3030 -n 200
```

## Token Reduction Benchmark

Measures how much token consumption is reduced when using HipCortex selective retrieval
vs naive history injection in coding assistant sessions (e.g. GitHub Copilot Chat).

**No running server required** — uses in-process simulation.

```bash
# Optional: install tiktoken for exact GPT-4/Copilot token counts
pip install tiktoken

# Run benchmark
python benchmarks/token_reduction_benchmark.py
```

### What it measures

| Approach | Description |
|----------|-------------|
| Full History | All prior turns injected every query (worst case) |
| Rolling Window (10) | Last 10 turns injected (Copilot-like sliding window) |
| HipCortex Top-5 | Semantic search retrieves 5 most relevant memories |
| HipCortex Top-3 | Semantic search retrieves 3 most relevant memories |

### Interpreting results

- **Savings % vs Full History** — how much less you spend vs never truncating
- **Sessions/month** — how many sessions fit in Copilot Business plan (1,900 credits/month)

Typical results: HipCortex Top-5 achieves **80–90% token reduction** vs full history,
allowing **5–10× more Copilot sessions** within the same credit budget.

### Why this matters (June 2026)

GitHub Copilot switched from flat-rate PRUs to token-based AI Credits billing on June 1, 2026.
Business plan = ~1,900 credits/month at $0.01/credit. Agentic sessions burn through credits
fast because they inject full conversation history into every request.

HipCortex replaces full-history injection with selective retrieval — only the most
relevant memories are included in each context bundle, drastically reducing token burn.
