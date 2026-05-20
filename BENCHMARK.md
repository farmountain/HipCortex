# HipCortex Memory Engine — Latency Benchmarks

Measures real p50/p95 write+query latency for HipCortex vs. Mem0 vs. in-process dict baseline.

---

## Methodology

| Dimension | Value |
|-----------|-------|
| Operations per backend | 200 (add) + 200 (query) |
| Payload | `{actor, action, target, metadata}` — same across all |
| Transport | REST over localhost (HipCortex) / HTTPS (Mem0 cloud) |
| Measurement | `time.perf_counter()` — wall-clock per call |
| Stats | p50 = median, p95 = 95th percentile |
| Run | cold start excluded; 10-op warm-up before timing |

---

## Results (HipCortex v0.2.0, Rust 1.95.0 stable, petgraph_backend)

### HipCortex local REST — measured on two platforms

| Platform | add_p50_ms | add_p95_ms | query_p50_ms | query_p95_ms |
|----------|-----------|-----------|-------------|-------------|
| **Linux x86_64** (Ubuntu 22.04, Ryzen 9, NVMe) | **0.61** | **1.1** | **0.23** | **0.45** |
| Windows 11 (AMD Ryzen AI 7 350, NVMe) | 1.74 | 2.51 | 0.49 | 0.66 |

n=200 ops, release build, localhost REST round-trip included.

### Comparison context (different workloads — not apples-to-apples)

| System | What it does | add_p50_ms |
|--------|-------------|-----------|
| HipCortex (local, Linux) | local HTTP + SHA-256 + Merkle audit + index | **0.61** |
| In-process Python dict | no persistence, no network, no audit | 0.0002 |
| Mem0 cloud (US-East)¹ | transatlantic network + embedding + vector DB | ~142 |

¹ Mem0 cloud latency from [their published benchmark](https://mem0.ai) (US-East endpoint).  
  **Fair comparison:** HipCortex local vs Mem0 local would require self-hosted Mem0 — we encourage you to benchmark both in your environment. Run: `python benchmarks/python_benchmark.py --url http://localhost:3030 -n 200`

**What 0.61ms p50 means:** HipCortex adds ~0.6ms per write for durable, SHA-256-hashed, Merkle-audited, causally-structured memory. An in-process dict adds 0.0002ms for none of those properties.

---

## Reproduce

```bash
# 1. Start HipCortex server
cargo run --bin webserver --no-default-features --features "web-server,petgraph_backend"

# 2. Run benchmark (in another terminal)
pip install hipcortex requests tabulate
python benchmarks/python_benchmark.py --url http://localhost:3030 -n 200

# 3. Full comparison (requires MEM0_API_KEY)
MEM0_API_KEY=<your_key> python benchmarks/python_benchmark.py \
    --url http://localhost:3030 -n 200 --mem0
```

---

## What the numbers mean

- **HipCortex** stores memory with SHA-256 integrity hash, Merkle-chained audit log, and temporal indexer update per write — sub-1ms p50 while maintaining full auditability.
- **Mem0 cloud** includes network round-trip to US-East, embedding generation, and vector DB upsert — fundamentally different workload, different trade-offs.
- **In-process dict** is the theoretical floor (no I/O, no persistence). HipCortex adds 0.48ms for durable, auditable, causally-structured memory.

---

## Feature comparison

| Feature | HipCortex | Mem0 | Zep |
|---------|-----------|------|-----|
| Temporal decay | ✅ native | ❌ | ❌ |
| Causal world model | ✅ | ❌ | ❌ |
| Coherence checking | ✅ | ❌ | ❌ |
| GDPR right-to-forget | ✅ REST endpoint | ✅ | ✅ |
| Merkle-chained audit log | ✅ | ❌ | ❌ |
| AES-GCM encrypted storage | ✅ | ❌ | ❌ |
| LangChain Memory | ✅ | ✅ | ✅ |
| LlamaIndex | ✅ | ✅ | ✅ |
| AutoGen hook | ✅ | ⚠️ partial | ❌ |
| CrewAI tools | ✅ | ❌ | ❌ |
| Self-hosted | ✅ zero deps | ❌ | ✅ |
| Rust performance | ✅ | ❌ (Python) | ❌ (Go) |
| Edge / embedded deploy | ✅ 4MB binary | ❌ | ❌ |

---

## Run on CI

```yaml
# .github/workflows/benchmark.yml
- name: Benchmark
  run: |
    cargo run --bin webserver --no-default-features \
      --features "web-server,petgraph_backend" &
    sleep 3
    python benchmarks/python_benchmark.py --url http://localhost:3030 -n 50
```
