# Enterprise Outreach Templates

Target segments: Healthcare AI (GDPR/HIPAA), Financial Services AI (audit trail),
Robotics/Edge (embedded Rust), Research Labs (AGI infrastructure).

---

## Template 1 — Healthcare / GDPR (cold email)

**Subject:** AI agent memory with GDPR right-to-forget built in — 2-min read

Hi [Name],

Your team is building AI agents that handle patient or user data. Every time one of those agents stores a memory, you're creating a GDPR liability — unless deletion is baked into the memory layer from day one.

HipCortex is an open-source AI memory engine (Rust) that gives you:
- `DELETE /memory/forget/:actor` — propagates through temporal store, symbolic graph, and Merkle-chained audit log in one call
- AES-GCM encrypted memory files with per-entry nonces
- EU data residency on Fly.io Frankfurt (fra region)
- Sub-1ms write latency with full auditability

We're talking to [3–5 healthcare AI teams] about enterprise agreements (EU hosting, ISO 27001 roadmap, dedicated support).

Is this relevant to [Company]? Happy to do a 20-minute technical walkthrough.

[Your name]

P.S. Benchmark: HipCortex writes at 0.48ms p50 vs 142ms for cloud alternatives —
the audit trail doesn't cost you latency.

---

## Template 2 — Financial Services / Audit Trail (cold email)

**Subject:** Tamper-evident AI memory for regulated environments

Hi [Name],

LLMs and AI agents in financial services need an audit trail that holds up in court. Most memory systems are append-only logs at best. HipCortex is different:

- Every memory write is SHA-256 hashed and Merkle-chained into `audit.log`
- `AuditLog::verify()` detects any tampering in the chain
- Full snapshot + rollback for crash recovery
- SOC 2 Type I in progress (completion Q3 2026)

We're working with [2 fintech teams] on custom enterprise deployments. Team tier includes unlimited records, priority support, and a dedicated Slack channel.

15 minutes to see if this fits your architecture?

[Your name]

---

## Template 3 — Robotics / Edge AI (cold email)

**Subject:** 4MB AI memory engine for robots — zero cloud dependency

Hi [Name],

Running AI on Jetson / edge hardware? Your agents need persistent episodic memory, but pulling in a Python memory system with cloud dependencies kills latency and adds 200MB+ to your binary.

HipCortex compiles to a 4MB binary (Rust, petgraph_backend) with:
- Temporal indexer with decay — short/long-term memory, FSM-driven task traces
- Zero external dependencies — works fully offline
- Per-unit embedded license: $[X]/year per deployed robot/device
- ARM64 cross-compilation supported

What's your current memory stack? Happy to share a Jetson benchmark.

[Your name]

---

## Template 4 — Research Labs / AGI (cold email)

**Subject:** Persistent causal world-model infrastructure for AGI research

Hi [Name],

Current LLMs lack persistent dynamic world models — they predict tokens, not reality. HipCortex is built around a different thesis:

- `world_model_enhanced/` — Dirichlet-Multinomial state transitions, Kalman entity tracking, causal do-calculus interventions
- `coherence/` — cross-module consistency checking (temporal-symbolic mismatches, causal violations, graph acyclicity invariants)
- `self_model/` — capability registry, EWMA performance tracking, expected utility decision engine

We're exploring research partnerships (grants + co-authorship) with labs working on multi-agent consistency, cognitive architectures, or memory-augmented reasoning.

Would a technical discussion on the coherence architecture be useful for your group?

[Your name]

---

## Pricing Sheet (internal — customize per prospect)

| Tier | Price | Includes |
|------|-------|----------|
| Pro | $299/month | 1M records, email support, GDPR endpoints |
| Team | $999/month | Unlimited records, priority support, SLA |
| Enterprise | $15K–$150K/year | Custom deployment, compliance docs, dedicated support, SLA |
| Embedded | $500–$2,000/unit/year | Per-device license, binary only, no server needed |
| Research | Custom (grant-funded) | Co-development, co-authorship rights, source access |

**Discounts:**
- Annual upfront: 20% off monthly rate
- Startup (<$5M raised): 40% off Team tier
- Academic/non-profit: 50% off

---

## Target Company List (priority outreach)

### Healthcare AI
- [ ] Nabla (clinical AI, France — GDPR critical)
- [ ] Abridge (ambient AI, US — HIPAA)
- [ ] Regard (clinical AI, US)
- [ ] Corti (emergency medicine AI, Denmark — EU)
- [ ] Glass Health (clinical reasoning AI)

### Financial Services
- [ ] Hebbia (document AI for finance)
- [ ] Rogo (investment banking AI)
- [ ] Daloopa (financial data AI)

### Robotics / Edge
- [ ] Physical Intelligence (pi.ai)
- [ ] Figure AI
- [ ] Sanctuary AI
- [ ] Boston Dynamics AI team

### Research / AGI Labs
- [ ] Allen Institute for AI (Ai2)
- [ ] Sakana AI (Tokyo — evolutionary AI)
- [ ] Eleuther AI
- [ ] Conjecture (AI safety)
- [ ] ARC (Alignment Research Center)
