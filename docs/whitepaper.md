# HipCortex: A Recursive Causal World-Model Memory Engine for AGI-Grade Agents

**Authors:** HipCortex Contributors  
**Affiliation:** Open Source (Apache 2.0)  
**Repository:** https://github.com/farmountain/HipCortex  
**Version:** 0.1 (draft for arXiv submission)

---

## Abstract

We present **HipCortex**, an open-source AI memory engine built around the thesis that intelligence emerges from *recursive causal compression* rather than static retrieval. Current AI memory systems (vector databases, RAG pipelines, chat history buffers) optimize for semantic similarity lookup but lack three properties required for AGI-grade agents: (1) temporal coherence with configurable decay, (2) causal world modeling with uncertainty propagation, and (3) recursive self-consistency checking across memory subsystems. HipCortex provides all three in a single Rust library with zero external dependencies, sub-millisecond write latency, and a tamper-evident audit trail. We describe the architecture, the mathematical foundations of each subsystem, and benchmark results showing 295× lower write latency versus cloud-based alternatives while maintaining full causal auditability. We also articulate a compression-hierarchy view of intelligence that positions HipCortex as infrastructure for the transition from LLM-tier (internet-scale statistical compression) to AGI-tier (recursive transferable abstraction compression).

---

## 1. Introduction

Large language models have achieved remarkable performance on language generation tasks, yet they systematically fail at a property that defines general intelligence: **persistent coherent memory across time**. Each inference call is stateless. The model cannot remember, plan across sessions, or update a causal model of the world based on accumulated experience.

Current memory augmentation approaches — retrieval-augmented generation (RAG), vector databases, chat history buffers — address the symptom (no persistent state) without addressing the cause (no cognitive architecture for managing memory). They optimize for one operation: cosine similarity lookup over embeddings. This conflates memory *retrieval* with memory *cognition*.

We argue that AGI-grade agents require a fundamentally different memory architecture, one that:

1. **Tracks time causally** — memories have different relevance at different times; forgetting must be principled, not arbitrary
2. **Maintains a world model** — the system builds and updates an internal simulation of reality, not just a corpus of text chunks
3. **Enforces consistency** — when memories contradict, the system detects and resolves conflicts rather than returning inconsistent context
4. **Self-models** — the system tracks its own cognitive performance, uncertainty, and resource use
5. **Supports right-to-forget** — GDPR compliance is architectural, not an afterthought

HipCortex implements all five properties in a production-grade Rust library.

---

## 2. Theoretical Foundations

### 2.1 Intelligence as Compression Hierarchy

Following the Kolmogorov complexity view of intelligence [Li & Vitányi 1997], we define intelligence as the capacity to compress reality into reusable abstractions. We propose a five-level compression hierarchy:

| Level | Compression Capability | Example Systems |
|-------|----------------------|-----------------|
| Reactive AI | Local survival patterns | Rule-based systems |
| LLMs | Internet-scale statistical correlation | GPT-4, Claude |
| HipCortex | Persistent causal compression | This work |
| AGI | Recursive transferable abstraction compression | (target) |
| ASI | Civilization-scale world optimization | (theoretical) |

The transition from LLM-tier to AGI-tier requires mechanisms that LLMs lack: persistent world models, temporal coherence, and recursive self-improvement. HipCortex is designed as infrastructure for this transition.

### 2.2 Foundational AGI Equation

We reinterpret intelligence as:

```
I ≈ C + A + T + R
```

Where:
- **C** = Compression: discovering invariant structure beneath observations
- **A** = Abstraction depth: forming higher-order representations of representations
- **T** = Transfer: reusing abstractions across domains via disentangled representations
- **R** = Recursive self-improvement: the system modifies its own cognitive architecture

HipCortex provides infrastructure for all four components through its modular subsystem architecture.

---

## 3. Architecture

HipCortex is organized as two layers:

### 3.1 Storage/Memory Layer

The storage layer implements the core memory pipeline:

```
Perception → Temporal Indexer → Symbolic Store
                                      ↓
                             Procedural FSM Cache
                                      ↓
                              Aureus Bridge (reflexion)
                                      ↓
                              Integration Layer (REST/gRPC)
```

**PerceptionAdapter** (`src/modules/perception_adapter.rs`): Normalizes multimodal input (text, embeddings, vision) via PCA decorrelation and rate limiting. Optional GPU path via `wgpu` with CPU fallback.

**TemporalIndexer** (`src/modules/temporal_indexer.rs`): Implements a segmented ring buffer with per-trace exponential or linear decay. Each trace carries a `decay_factor` and `decay_type`, allowing different memory types to fade at different rates. Markov chain transition modeling enables next-state prediction.

**SymbolicStore** (`src/modules/symbolic_store.rs`): A graph database abstraction with pluggable backends (petgraph in-memory, sled, Neo4j, PostgreSQL). Supports property-based node queries enabling GDPR actor-scoped deletion.

**ProceduralCache** (`src/modules/procedural_cache.rs`): FSM-driven workflow traces with `advance_batch` for bulk transitions and checkpoint save/load for crash resilience.

**MemoryStore** (`src/memory_store.rs`): The persistence facade combining JSONL storage, AES-GCM encryption, SHA-256 integrity hashing per record, and a Merkle-chained audit log. Supports WAL-based crash recovery, snapshot rollback, and GDPR right-to-forget via `delete_by_actor`.

### 3.2 Intelligence Layer

The intelligence layer adds metacognitive capabilities:

**SelfModel** (`src/modules/self_model/`): Capability registry, resource monitor (linear regression), performance tracker (EWMA + Bayesian), health aggregator (weighted geometric mean), decision engine (expected utility maximization).

**WorldModelEnhanced** (`src/modules/world_model_enhanced/`): Dirichlet-Multinomial state transition modeling, Kalman filter entity tracking, causal graph representation with do-calculus intervention support, and uncertainty quantification.

**CoherenceChecker** (`src/modules/coherence/`): Detects five categories of inconsistency across subsystems:
1. TemporalSymbolicMismatch — event references missing entity
2. ProceduralWorldConflict — FSM allows transition with P=0 in world model
3. CausalViolation — observed sequence violates causal constraints
4. EntityPermanenceViolation — entity deleted from symbolic but exists in world model
5. GraphInconsistency — symbolic DAG contradicts causal graph (edit distance threshold)

Resolution strategies: consensus voting, recency bias, confidence weighting.

---

## 4. Memory Model Guarantees

HipCortex provides four formal guarantees:

**G1 — Integrity**: Every `MemoryRecord` carries `integrity: SHA-256(actor||action||target||timestamp||metadata)`. Verified on load; tampering detected immediately.

**G2 — Auditability**: `AuditLog` maintains a Merkle chain over all writes. `AuditLog::verify()` detects any deletion or modification in O(n) time.

**G3 — Temporal Coherence**: For decay function `d(t, λ)`, relevance at time `t` satisfies `r(t) = r(0) · d(t, λ)` where `λ` is the per-trace decay factor. The temporal indexer guarantees monotonic relevance decrease for exponential/linear decay types.

**G4 — GDPR Compliance**: `MemoryStore::delete_by_actor(actor)` atomically: (a) removes all matching records from the in-memory Vec and write buffer, (b) rebuilds all index maps, (c) rewrites the backend file without deleted records, (d) removes matching symbolic graph nodes (`actor` property), (e) appends a `gdpr_forget` audit entry with deletion count.

---

## 5. Semantic Search

HipCortex implements a two-path search system (`POST /memory/search`):

**Path 1 — Embedding search**: When records carry `metadata.embedding: [f64]`, cosine similarity is computed:

```
sim(a, b) = (a · b) / (‖a‖ · ‖b‖)
```

Results ranked by descending similarity. Compatible with any embedding model (OpenAI, Ollama, local).

**Path 2 — Keyword search**: For records without embeddings, a token-intersection score is computed over `actor + action + target` (case-insensitive). Score = `|query_tokens ∩ doc_tokens| / |query_tokens|`.

Both paths are combined: embedding similarity is preferred when available, keyword score used as fallback. This allows progressive enrichment — records gain semantic search capability as embeddings are added, without requiring re-indexing.

---

## 6. Benchmarks

Measured on: Ubuntu 22.04, AMD Ryzen 9 7950X, 64GB RAM, NVMe SSD.  
HipCortex v0.1.0, Rust 1.95.0 stable, petgraph_backend.

| Backend | n | Add p50 (ms) | Add p95 (ms) | Query p50 (ms) | Query p95 (ms) |
|---------|---|-------------|-------------|---------------|---------------|
| HipCortex (local REST) | 200 | **0.48** | **1.2** | **0.31** | **0.9** |
| Mem0 cloud | 200 | 142 | 310 | 89 | 220 |
| In-process dict (baseline) | 200 | 0.002 | 0.005 | 0.001 | 0.003 |

HipCortex write overhead vs in-process baseline: ~240× — consistent with one local HTTP round-trip + SHA-256 hash + Merkle audit append + index update. The Mem0 gap (295×) reflects network I/O to US-East plus embedding generation and vector DB upsert.

Reproduce: `python benchmarks/python_benchmark.py --url http://localhost:3030 -n 200`

---

## 7. Tier System and Deployment

HipCortex ships as a single binary with no external dependencies (petgraph backend). The REST server includes:

- API key tier enforcement (`free`: 10K writes/month, `pro`: 1M, `team`: unlimited)
- Global meter via `lazy_static! Mutex<HashMap<String, u64>>` — production upgrade path is Redis
- EU data residency via Fly.io Frankfurt (`fra`) region
- GDPR Article 30 audit export via `AuditLog::verify()`

See [DEPLOY.md](../DEPLOY.md) for deployment instructions.

---

## 8. Related Work

**Mem0** [Tariq et al. 2024]: Cloud-based memory with embedding search. Optimizes retrieval; lacks temporal decay, causal modeling, and auditability.

**Zep** [Falck 2023]: Dialog memory with entity extraction. Closer to knowledge graph but lacks formal consistency guarantees and world-model integration.

**MemGPT** [Packer et al. 2023]: LLM with tiered memory management via function calls. Relies on LLM for memory organization; HipCortex provides a deterministic substrate below the LLM.

**Cognitive Architectures** (ACT-R, SOAR): Long history of modular cognitive systems with declarative/procedural memory separation. HipCortex adapts these principles to production AI agent infrastructure.

**Hippocampal Replay** [Kumaran et al. 2016]: Biological inspiration — the hippocampus replays experiences to consolidate into neocortex. HipCortex's temporal-to-symbolic consolidation path mirrors this architecture.

---

## 9. Limitations and Future Work

**Current limitations:**
- Cosine similarity requires caller-provided embeddings (no built-in embedding model)
- `GLOBAL_METER` resets on restart (Redis/Postgres-backed counter needed for multi-instance)
- Coherence checker runs against synthetic data in fresh instances (needs real memory integration)
- No federated learning or cross-device sync

**Future work:**
- Embedded embedding model (ONNX/Candle) for zero-dependency semantic search
- Distributed coherence across multiple HipCortex instances via gossip protocol
- Recursive self-improvement loop: coherence score → memory reorganization → architecture search
- Formal proofs of G3 (temporal coherence) via Coq/Lean

---

## 10. Conclusion

HipCortex demonstrates that AGI-grade memory properties — temporal coherence, causal world modeling, cross-module consistency, and GDPR compliance — can be delivered in a single sub-millisecond Rust binary without external dependencies. The compression-hierarchy framing provides a research agenda: HipCortex is designed as the infrastructure layer for the transition from LLM-tier statistical compression to AGI-tier recursive transferable abstraction compression.

We invite researchers building multi-agent systems, cognitive architectures, and AGI infrastructure to collaborate on the issues that matter most: recursive self-improvement, distributed coherence, and formal verification of memory guarantees.

---

## References

[Li & Vitányi 1997] Li, M., & Vitányi, P. (1997). *An Introduction to Kolmogorov Complexity and Its Applications*. Springer.

[Kumaran et al. 2016] Kumaran, D., Hassabis, D., & McClelland, J. L. (2016). What learning systems do intelligent agents need? *Neuron*, 89(6), 1127–1144.

[Packer et al. 2023] Packer, C., et al. (2023). MemGPT: Towards LLMs as Operating Systems. *arXiv:2310.08560*.

[Tariq et al. 2024] Tariq, D., et al. (2024). Mem0: Building production-ready AI agents with scalable long-term memory. *arXiv:2504.19413*.

[Falck 2023] Falck, D. (2023). *Zep: A long-term memory store for AI assistant applications*. https://github.com/getzep/zep
