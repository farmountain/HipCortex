#!/usr/bin/env python3
"""
HipCortex Token Reduction Benchmark
====================================
Measures how much token consumption is reduced when using HipCortex
selective memory retrieval vs naive history injection in coding assistant sessions.

Usage:
    python benchmarks/token_reduction_benchmark.py

Requirements:
    pip install tiktoken  (optional but recommended for accurate Copilot token counts)

No running server required — uses in-process simulation.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import List, Tuple, Dict


def count_tokens(text: str) -> int:
    """Count tokens using tiktoken (cl100k_base = GPT-4 / Copilot tokenizer).
    Falls back to len//4 estimate if tiktoken is not installed."""
    try:
        import tiktoken
        enc = tiktoken.get_encoding("cl100k_base")
        return len(enc.encode(text))
    except ImportError:
        return max(1, len(text) // 4)


class InProcessMemoryStore:
    """Simulates HipCortex memory store in-process without a running server."""

    def __init__(self):
        self.records: List[Dict] = []

    def add(self, actor: str, action: str, target: str) -> None:
        self.records.append({
            "actor":  actor,
            "action": action,
            "target": target,
            "text":   f"[{action}] {target}",
        })

    def search_semantic(self, query: str, top_k: int) -> List[str]:
        """Simple keyword-overlap search (simulates HipCortex search_semantic)."""
        if not self.records:
            return []
        query_words = set(query.lower().split())
        scored = []
        for rec in self.records:
            text = f"{rec['actor']} {rec['action']} {rec['target']}".lower()
            score = sum(1 for w in query_words if len(w) > 2 and w in text)
            if score > 0:
                scored.append((score, rec["text"]))
        scored.sort(key=lambda x: -x[0])
        return [t for _, t in scored[:top_k]]

    def clear(self) -> None:
        self.records.clear()


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------

SCENARIO_1_NAME = "HipCortex Dev Decisions (real, 20 turns)"
SCENARIO_1: List[Tuple[str, str]] = [
    ("What database should we use for the memory store?",
     "Decided JSONL file as default backend with petgraph for graph store. PostgreSQL and RocksDB are optional feature-gated backends."),
    ("How should we handle API authentication?",
     "HIPCORTEX_API_KEYS env var with format key:tier. Three tiers: Free (10K writes/month), Pro (1M), Team (unlimited). Public endpoints bypass auth."),
    ("What should the memory record schema look like?",
     "MemoryRecord: id, record_type (Temporal/Symbolic/Procedural/Reflexion), actor, action, target, confidence, source, priority, tags, version, status, expires_at, decay_factor, metadata, integrity hash."),
    ("How should decay work?",
     "Exponential decay per trace, configurable decay_factor per record. Pinned priority bypasses decay. TTL via expires_at for working memory."),
    ("How do we handle GDPR right-to-forget?",
     "DELETE /memory/forget/:actor propagates atomically through temporal + symbolic + audit log. Regulatory holds (MiFID II) can block forget."),
    ("What is the architecture for the world model?",
     "Dirichlet-Multinomial for state transitions, Kalman filter for entity tracking, causal DAG with do-calculus. Persists to worldmodel.json on shutdown."),
    ("How should AureusBridge work?",
     "Bayesian belief update over HypothesesGraph. Takes MemoryStore for context search, optionally calls LLM. Exposed via POST /memory/reflect."),
    ("What VS Code extension approach should we use?",
     "Register as chatParticipant (@hipcortex) in Copilot Chat. Also register as LM Tool so Copilot can call hipcortex_search automatically."),
    ("How do we benchmark token savings?",
     "Compare full-history vs rolling-10 vs HipCortex top-5 vs top-3. Use tiktoken cl100k_base. Three scenarios: dev decisions, web API, debugging."),
    ("What is the deployment strategy?",
     "Fly.io for managed tier (Frankfurt). Docker + Helm for self-hosted. Single binary, 4MB, no external deps for petgraph backend."),
    ("How should coherence checking work?",
     "ConsistencyChecker runs every 60s in background. Detects conflicts using keyword overlap, recency, confidence. ConflictResolver with three strategies."),
    ("What memory types should we support?",
     "Temporal (events, 24h TTL default), Symbolic (decisions, permanent), Procedural (FSM traces), Reflexion (hypotheses). Plus Perception for multimodal."),
    ("How does the Python SDK zero-config API work?",
     "client.remember(text) calls POST /memory/ingest. client.recall(query) calls GET /memory/search-flat. client.recall_with_metadata() returns full records."),
    ("What should the search response include?",
     "MemoryRecordResponse with 15 fields: confidence, source, priority, tags, version, status, expires_at plus id/type/timestamp/actor/action/target/metadata/integrity."),
    ("How should WorldModel auto-feed work?",
     "After every POST /memory/add and /ingest, non-blocking try_write() to observe_transition(actor, action, target). Pinned Symbolic records add causal edge."),
    ("What is the AppState architecture?",
     "AppState<B> bundles MemoryStore + SymbolicStore + WorldModelEnhanced + AureusBridge + SelfModel + CoherenceChecker. run_with_state() is primary entry point."),
    ("How should SelfModel capability gating work?",
     "Advisory-only can_execute() logs warning but never blocks requests. Hard gate after enough observations to calibrate."),
    ("What GitHub Copilot billing issue affects our users?",
     "June 1 2026: GitHub switched to token-based AI Credits billing. Business ~1900 credits/month at $0.01/credit. Agentic sessions exhaust budget in 1 day."),
    ("How should LM Tool registration work in VS Code?",
     "vscode.lm.registerTool('hipcortex_search') requires VS Code 1.90+. Copilot calls tool automatically when it needs memory context."),
    ("What token savings do we expect from HipCortex?",
     "vs full history: expect 80-90% savings. vs rolling-10: expect 50-75% savings. Actual benchmark needed before enterprise pitch."),
]

SCENARIO_2_NAME = "Web API Development (synthetic, 15 turns)"
SCENARIO_2: List[Tuple[str, str]] = [
    ("How should we structure the authentication middleware?",
     "JWT tokens with RS256 signing. Token expiry 24 hours. Refresh tokens in Redis with 30-day TTL. Middleware validates every request except /health and /auth/login."),
    ("What database should we use for user data?",
     "PostgreSQL 15 with pgvector extension. Connection pooling via pgbouncer. Read replicas for query-heavy endpoints."),
    ("How should we handle rate limiting?",
     "Token bucket algorithm. Per-user: 100 req/min free, 1000 req/min paid. Redis-backed. Return 429 with Retry-After header."),
    ("What is the error response format?",
     "Standard JSON: {error, code, details?}. HTTP status follows RFC 7807. Always include request_id for tracing."),
    ("How do we deploy the API?",
     "Docker on Kubernetes. Horizontal pod autoscaling. Blue-green deployment. Secrets via Vault."),
    ("How should we structure the test suite?",
     "Unit tests for business logic, integration tests against test DB, contract tests for external services. pytest + factory_boy. 80% coverage target."),
    ("What caching strategy should we use?",
     "L1: in-process LRU cache (5 min TTL). L2: Redis (1 hour TTL). Cache invalidation on write. Keys include user_id to prevent leakage."),
    ("How should we handle file uploads?",
     "Pre-signed S3 URLs for direct uploads. Max 50MB. ClamAV virus scanning. Metadata in PostgreSQL."),
    ("What is our logging strategy?",
     "Structured JSON logs to stdout. Correlation IDs. PII redaction middleware. Ship to Datadog. Alert on error rate > 1% or p99 > 500ms."),
    ("How to handle database migrations?",
     "Alembic. Always forward-only in production. Shadow migrations in CI. Migration runs before app rollout."),
    ("What is the API versioning strategy?",
     "URL path versioning /v1/ /v2/. Maintain 2 major versions. 6-month deprecation notice. Sunset header on deprecated endpoints."),
    ("How should we handle background jobs?",
     "Celery with Redis broker. Dead letter queue. Exponential backoff retries (max 3). Results stored 24 hours."),
    ("What monitoring setup should we use?",
     "Prometheus metrics on /metrics. Grafana dashboards. PagerDuty for on-call alerts."),
    ("How to handle CORS?",
     "Allow listed origins only. Credentials: true for same-site cookies. Preflight cache 24 hours. Allow: GET, POST, PUT, DELETE, OPTIONS."),
    ("What is our search implementation?",
     "Elasticsearch for full-text. Postgres tsvector for simple queries. pgvector for semantic search. Results ranked by relevance + recency."),
]

SCENARIO_3_NAME = "Debugging Session (synthetic, 15 turns)"
SCENARIO_3: List[Tuple[str, str]] = [
    ("Users report 500 errors on POST /api/orders intermittently. Logs show connection pool exhausted.",
     "Database connections not being released. Check missing conn.close() or with statement misuse in order handler."),
    ("Order validation runs 3 separate DB queries without connection pool. Each creates new connection.",
     "Replace with single transaction using connection pool. Use with get_db_connection() as conn pattern."),
    ("After fixing, still seeing 500s. Error: deadlock detected.",
     "Two transactions waiting on each other locks. Check lock order: payment locks orders then inventory, stock reservation does inventory then orders."),
    ("Payment locks orders then inventory, stock reservation does inventory then orders.",
     "AB-BA deadlock. Standardize: always lock inventory first, then orders. Update both code paths."),
    ("Fixed deadlock. Now seeing slow queries. EXPLAIN ANALYZE shows sequential scans on orders.",
     "Missing index. Add index on orders.user_id and orders.created_at. Check if orders.status needs index."),
    ("Added indexes. Performance improved but memory spikes in worker processes.",
     "Large result sets not paginated. Add LIMIT to all DB queries in workers."),
    ("Order export job loads ALL orders into memory. We have 50M+ orders.",
     "Stream instead of batch. Use server-side cursors or paginate. Write directly to S3 in chunks."),
    ("After pagination, export takes 3 hours instead of 30 minutes.",
     "OFFSET scans are O(n). Switch to keyset pagination: WHERE id > last_seen_id ORDER BY id LIMIT 1000."),
    ("Export works but some orders duplicated, some missing.",
     "Concurrent writes during export. Use REPEATABLE READ isolation or snapshot order IDs at start."),
    ("Chose snapshot approach. Email job sometimes fails silently.",
     "Exceptions being swallowed. Check Celery FAILURE states. Add explicit error handling and retry logic."),
    ("Email delivery has 10% bounce rate. Some IPs blacklisted.",
     "IP reputation issue. Switch to dedicated IP. Implement SPF, DKIM, DMARC. Consider SendGrid or AWS SES."),
    ("Production deployment failed. Migration ran but app throwing column does not exist.",
     "Migration applied to wrong DB or version mismatch. Check env vars, alembic current, all read replicas."),
    ("Migration applied to primary but not replicas.",
     "Add pre-deployment check verifying all replicas at expected migration version. Add to readiness probe."),
    ("Added replica check. API returning stale cached data after fixing price calculation bug.",
     "Cache invalidation issue. Find all price cache locations, ensure invalidated. Manually flush Redis."),
    ("Stale cache cleared. All issues resolved. Writing postmortem.",
     "Cover: timeline, root causes (connection pool + deadlock + pagination + cache), impact, fixes, prevention."),
]

SCENARIOS = [
    (SCENARIO_1_NAME, SCENARIO_1),
    (SCENARIO_2_NAME, SCENARIO_2),
    (SCENARIO_3_NAME, SCENARIO_3),
]


@dataclass
class ScenarioResult:
    name: str
    n_turns: int
    baseline_full_tokens: int = 0
    baseline_rolling10_tokens: int = 0
    hipcortex_top5_tokens: int = 0
    hipcortex_top3_tokens: int = 0


def run_scenario(name: str, turns: List[Tuple[str, str]], rolling_window: int = 10) -> ScenarioResult:
    result = ScenarioResult(name=name, n_turns=len(turns))
    store = InProcessMemoryStore()
    history: List[str] = []

    for query, answer in turns:
        turn_text = f"Q: {query}\nA: {answer}"

        full_context = "\n".join(history)
        result.baseline_full_tokens += count_tokens(full_context + "\n" + query)

        rolling_context = "\n".join(history[-rolling_window:])
        result.baseline_rolling10_tokens += count_tokens(rolling_context + "\n" + query)

        top5 = store.search_semantic(query, top_k=5)
        result.hipcortex_top5_tokens += count_tokens("\n".join(top5) + "\n" + query)

        top3 = store.search_semantic(query, top_k=3)
        result.hipcortex_top3_tokens += count_tokens("\n".join(top3) + "\n" + query)

        store.add(actor="session", action="said", target=turn_text)
        history.append(turn_text)

    return result


def savings_pct(baseline: int, treatment: int) -> float:
    if baseline == 0:
        return 0.0
    return (baseline - treatment) / baseline * 100.0


def credits_saved(baseline_tokens: int, treatment_tokens: int) -> float:
    return max(0, baseline_tokens - treatment_tokens) / 1000.0 * 0.01


def fmt_savings(vs: float | None, is_baseline: bool = False) -> str:
    if is_baseline:
        return "baseline"
    if vs is None:
        return "n/a"
    return f"-{vs:.1f}%" if vs > 0 else f"+{abs(vs):.1f}%"


def print_results(results: List[ScenarioResult]) -> None:
    COPILOT_BUSINESS_CREDITS = 1900

    print("\n" + "=" * 72)
    print("  HipCortex Token Reduction Benchmark")
    print("=" * 72)
    try:
        import tiktoken
        print("  Token counter: tiktoken cl100k_base (exact GPT-4/Copilot tokenizer)")
    except ImportError:
        print("  Token counter: len//4 estimate  (pip install tiktoken for exact count)")
    print()

    col = [35, 14, 16, 16]
    hdr = f"{'Approach':<{col[0]}}{'Input Tokens':>{col[1]}}{'vs Full Hist':>{col[2]}}{'vs Rolling-10':>{col[3]}}"
    sep = "-" * sum(col)

    total_full = total_rolling = total_top5 = total_top3 = 0

    for r in results:
        print(f"Scenario: {r.name}  ({r.n_turns} turns)")
        print(sep)
        print(hdr)
        print(sep)
        rows = [
            ("Full History (baseline)",   r.baseline_full_tokens,    None, None, True,  True),
            ("Rolling Window (last 10)",  r.baseline_rolling10_tokens,
             savings_pct(r.baseline_full_tokens, r.baseline_rolling10_tokens), None, False, True),
            ("HipCortex Top-5",          r.hipcortex_top5_tokens,
             savings_pct(r.baseline_full_tokens, r.hipcortex_top5_tokens),
             savings_pct(r.baseline_rolling10_tokens, r.hipcortex_top5_tokens), False, False),
            ("HipCortex Top-3",          r.hipcortex_top3_tokens,
             savings_pct(r.baseline_full_tokens, r.hipcortex_top3_tokens),
             savings_pct(r.baseline_rolling10_tokens, r.hipcortex_top3_tokens), False, False),
        ]
        for label, tokens, vf, vr, is_full_baseline, is_rolling_baseline in rows:
            print(f"{label:<{col[0]}}{tokens:>{col[1]},}"
                  f"{fmt_savings(vf, is_full_baseline):>{col[2]}}"
                  f"{fmt_savings(vr, is_rolling_baseline):>{col[3]}}")
        cred = credits_saved(r.baseline_full_tokens, r.hipcortex_top5_tokens)
        print(f"\n  HipCortex Top-5 saves ~${cred:.3f}/session vs full history\n")
        total_full += r.baseline_full_tokens
        total_rolling += r.baseline_rolling10_tokens
        total_top5 += r.hipcortex_top5_tokens
        total_top3 += r.hipcortex_top3_tokens

    print("=" * 72)
    print("  SUMMARY (all scenarios combined)")
    print("=" * 72 + "\n")
    print(sep)
    print(hdr)
    print(sep)
    summary = [
        ("Full History (baseline)",   total_full,    None, None, True,  True),
        ("Rolling Window (last 10)",  total_rolling,
         savings_pct(total_full, total_rolling), None, False, True),
        ("HipCortex Top-5",          total_top5,
         savings_pct(total_full, total_top5),
         savings_pct(total_rolling, total_top5), False, False),
        ("HipCortex Top-3",          total_top3,
         savings_pct(total_full, total_top3),
         savings_pct(total_rolling, total_top3), False, False),
    ]
    for label, tokens, vf, vr, is_full_baseline, is_rolling_baseline in summary:
        print(f"{label:<{col[0]}}{tokens:>{col[1]},}"
              f"{fmt_savings(vf, is_full_baseline):>{col[2]}}"
              f"{fmt_savings(vr, is_rolling_baseline):>{col[3]}}")

    sessions_full = COPILOT_BUSINESS_CREDITS / max(1, total_full / 1000.0)
    sessions_top5 = COPILOT_BUSINESS_CREDITS / max(1, total_top5 / 1000.0)
    multiplier    = sessions_top5 / max(1, sessions_full)
    savings_sess  = credits_saved(total_full, total_top5)

    print(f"""
  Copilot Business plan: ~{COPILOT_BUSINESS_CREDITS:,} credits/month @ $0.01/credit
  Sessions/month (full history): ~{sessions_full:.0f}
  Sessions/month (HipCortex):    ~{sessions_top5:.0f}  ({multiplier:.1f}× more sessions)
  Estimated savings:             ~${savings_sess:.3f}/session  (Top-5 vs full history)
""")


def main() -> None:
    results = []
    for name, turns in SCENARIOS:
        print(f"Running: {name} ...", end=" ", flush=True)
        results.append(run_scenario(name, turns))
        print("done")
    print_results(results)


if __name__ == "__main__":
    main()
