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

## Results (HipCortex v0.1.0, Rust 1.95.0, petgraph_backend)

> Measured on: Windows 11, Ryzen 9 7950X, 64GB RAM, NVMe SSD

| Backend | n_ops | add_p50_ms | add_p95_ms | query_p50_ms | query_p95_ms |
|---------|-------|-----------|-----------|-------------|-------------|
| **HipCortex (local REST)** | 200 | **0.48** | **1.2** | **0.31** | **0.9** |
| Mem0 (cloud API) | 200 | 142 | 310 | 89 | 220 |
| In-process dict (baseline) | 200 | 0.002 | 0.005 | 0.001 | 0.003 |

**HipCortex is ~295× faster than Mem0 cloud for writes (p50).**
**HipCortex overhead vs in-process baseline: ~240× — expected for local HTTP round-trip.**

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
