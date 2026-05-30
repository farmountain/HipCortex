# HipCortex — Session Handover Document
**Date:** 2026-05-30  
**Branch:** `claude/pedantic-edison-28b84c` (51 commits ahead of main)  
**Live URL:** https://hipcortex.fly.dev  
**PyPI:** `pip install hipcortex` (published, v0.2.0)  
**GitHub:** https://github.com/farmountain/HipCortex (public)

---

## What Was Built (Complete)

HipCortex is a **persistent causal memory engine for AI agents** — Rust binary, 4MB, zero external deps.

### Architecture
- **Language:** Rust (Axum 0.6 web server, petgraph backend)
- **Worktree:** `D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c`
- **Deployed:** `hipcortex.fly.dev` (Fly.io, Frankfurt, free tier)
- **Commercial branch:** `commercial-moat` (local only, NEVER push to GitHub)

---

## REST API — All 35+ Endpoints

### Memory Operations (core)
| Endpoint | Description |
|----------|-------------|
| `POST /memory/ingest` | **Zero-config** — auto-classifies record_type, priority, TTL, tags from plain text |
| `POST /memory/add` | Full-control add (actor, action, target, record_type, confidence, source, tags, priority, ttl_seconds, decay_factor) |
| `GET /memory/query` | Filter: actor/action/type/tags/priority/as_of (time-travel) |
| `POST /memory/search` | Cosine similarity + keyword; add `embedding_model` to auto-embed |
| `GET /memory/search-flat` | Plain string array — for no-code tools (Flowise, Dify, n8n) |
| `PATCH /memory/update/:id` | In-place correction, version++ |
| `GET /memory/latest` | Most recent fact per actor+action (no stale returns) |
| `DELETE /memory/forget/:actor` | GDPR — propagates through temporal + symbolic + audit |
| `POST /memory/bulk` | Up to N records in one call |
| `GET /memory/export` | Data portability |
| `POST /memory/consolidate` | Keyword dedup report |

### Fields on every MemoryRecord
`confidence`, `source`, `version`, `tags[]`, `priority` (pinned/high/normal/low), `ttl_seconds`, `decay_factor`, `expires_at`

**Pinned** = never decays, always in search results (safety constraints).

### Knowledge Graph
`POST /graph/node`, `POST /graph/edge`, `DELETE /graph/node/:id`, `GET /graph`, `GET /node/:id`

### Audit + Compliance
`GET /audit/verify` (Merkle tamper detection), `GET /audit/export`  
`POST /regulatory/hold`, `DELETE /regulatory/hold/:actor`, `GET /regulatory/hold` (MiFID II blocks GDPR forget)  
HIPAA BAA template: `docs/compliance/HIPAA-BAA-template.md`

### Observability
`GET /metrics` (Prometheus), `GET /stats`, `GET /coherence/status`, `GET /coherence/inconsistencies`

### World Model (commercial boundary)
`GET /worldmodel/status` — public stub (says "full inference in managed tier")  
`POST /worldmodel/predict` + `POST /worldmodel/observe` — **commercial-moat branch only**, Dirichlet-Multinomial + Kalman

### System
`GET /health`, `GET /stats`, `GET /tier`, `GET /pricing`, `GET /openapi.json`, `GET /ns`  
`POST /webhooks/register`, `DELETE /webhooks/:id`, `GET /webhooks`

---

## SDK + Integrations

### Python SDK (`pip install hipcortex`)
- `HipCortexClient` (sync), `AsyncHipCortexClient` (httpx)
- `client.remember("text")` → POST /memory/ingest (zero-config)
- `client.recall("query")` → GET /memory/search-flat (plain strings)
- `client.remember_and_recall()` — store + retrieve in one call
- `HipCortexMemory`, `AsyncHipCortexMemory` — LangChain drop-in
- `HipCortexChatStore`, `HipCortexStorageContext` — LlamaIndex
- `HipCortexAutoGenMemory` — AutoGen 0.4 Memory protocol
- `HipCortexRememberTool`, `HipCortexRecallTool`, `HipCortexForgetTool` — CrewAI

### TypeScript SDK (`npm install hipcortex`)
`sdk/typescript/` — native fetch, zero deps, full type safety

### CLI (`hipcortex install`)
**Interactive multi-select wizard** (openspec-style):
- Section 1 — Coding Assistants (12): Claude Code, Cursor, Windsurf, VS Code, Cline, RooCode, Continue, GitHub Copilot [guide], Codex CLI [guide], Aider [guide], Gemini CLI [guide], Amazon Q [guide]
- Section 2 — Agent Frameworks (7+): LangChain, CrewAI, AutoGen, LlamaIndex, Pydantic AI, n8n/Make.com, DSPy, Flowise/Dify [guide]

For coding assistants: configures MCP/SKILL.md automatically  
For frameworks: writes starter integration file in cwd (`hipcortex_langchain.py` etc.)  
Also: `hipcortex start`, `hipcortex backup`, `hipcortex restore`, `hipcortex status`, `hipcortex uninstall`

### Claude Code (SKILL.md native)
`hipcortex install` → writes `~/.claude/skills/hipcortex/SKILL.md` + registers in `~/.claude/CLAUDE.md`  
Slash commands: `/hipcortex remember`, `/hipcortex recall`, `/hipcortex latest`, `/hipcortex update`, `/hipcortex forget`, `/hipcortex stats`

### MCP Server
`sdk/mcp/server.py` — JSON-RPC 2.0 over stdio  
Tools: `add_memory`, `search_memory`, `forget_actor`, `get_stats`  
Install: `curl -fsSL .../sdk/mcp/install.sh | bash`  
Configures: Cursor, Windsurf, VS Code, Claude Code (mcpServers)

### Other SDKs
- Continue.dev: `sdk/continue/index.ts` — context provider + `/remember` `/recall`
- Gradio UI: `sdk/gradio/app.py` — HuggingFace Spaces (add/search/export/forget)
- Replit: `.replit` + `start.sh` (downloads binary on first run)

---

## Deployment

### Infrastructure
- **Fly.io:** `fly.toml` (Frankfurt `fra`, persistent volume, HTTPS)
- **Docker:** `Dockerfile` (Rust 1.87-bookworm builder)
- **Helm:** `deploy/helm/hipcortex/` (K8s Helm chart)
- **Systemd:** `docs/systemd/hipcortex.service`

### CI/CD
- `.github/workflows/ci.yml` — build/test/clippy/fmt + benchmark on PRs
- `.github/workflows/release.yml` — ARM64/AMD64/macOS/Windows binaries on release
- `.github/workflows/publish-pypi.yml` — PyPI publish on release (needs PYPI_API_TOKEN secret)

---

## Commercial Moat (LOCAL ONLY — never push to GitHub)

Branch: `commercial-moat` (1 commit ahead of public branch)

**Files (gitignored or local-only):**
- `src/commercial.rs` — `handle_llm_generate` (RAG+memory+LLM in one call) + `handle_worldmodel_predict`
- `src/bin/pro_server.rs` — commercial binary with private endpoints

**Build:**
```bash
git checkout commercial-moat
cargo run --bin pro_server --no-default-features --features "web-server,petgraph_backend,commercial"
```

**POST /llm/generate:**
1. Searches memory for relevant context
2. Injects into system prompt
3. Calls Ollama/OpenAI/Anthropic
4. Stores response (confidence=0.8, source="llm-{model}")

**POST /worldmodel/predict:** Dirichlet-Multinomial state transitions (real inference, not stub)

**Commercial boundary strategy:** API interface is public, intelligence algorithm is private (Redis/Elastic open-core model).

---

## Key Design Decisions

### Memory Model
- `actor/action/target` = who/what/content — not just key-value
- Temporal decay: exponential/linear per trace, configurable `decay_factor`
- Pinned priority bypasses decay (safety constraints never fade)
- SHA-256 integrity per record + Merkle chain audit log
- GDPR forget = atomic: temporal + symbolic + audit entry

### Smart Ingest Heuristics (`POST /memory/ingest`)
- Decision words ("decided", "chose") → `Symbolic`, `priority=high`, no TTL
- Constraint words ("never", "must") → `Symbolic`, `priority=pinned`, no TTL
- Time references ("today", "at 3pm") → `Temporal`, TTL=24h (working memory)
- Conversation words ("said", "replied") → `Reflexion`, TTL=24h
- Code patterns → `Procedural`, no TTL
- Actor extracted from "Name verb" pattern
- Tags auto-mapped from domain keywords

### Search
- `search_semantic()` always prepends pinned records (score=2.0)
- Falls back: cosine similarity → keyword if no embeddings
- `max_tokens` param truncates results to fit LLM context window

### Auth / Tiers
- `HIPCORTEX_API_KEYS=key:tier` env var → `X-Api-Key` header OR `?api_key=` query param
- Free: 10K writes/month, Pro: 1M, Team: unlimited
- Public endpoints: `/health`, `/stats`, `/metrics`, `/ns`, `/worldmodel/status`, `/coherence/inconsistencies`, `/memory/search-flat`, `/memory/latest`, `/openapi.json`, `/pricing`

---

## MiroFish Simulation History

9 simulations run, key findings:
- **Sim 1-3:** P0 features, 47 gaps, coding agent market discovered
- **Sim 4:** Publish gates broken (pip/npm) → fixed
- **Sim 5:** 10-action path to 500 stars (Show HN + ARM64 + async SDK)
- **Sim 6:** Post-install-CLI golden path audit
- **Sim 7:** Memory accuracy: confidence/source/version/update/latest
- **Sim 8:** AI ecosystem integration: tags, priority, as_of, graph write, consolidate
- **Sim 9:** Zero-config memory → POST /memory/ingest + client.remember()

---

## Pending Distribution (Manual Steps Required)

| Action | Status | Instructions |
|--------|--------|-------------|
| PyPI published | ✅ DONE | v0.2.0 live |
| npm publish | ❌ Needs `npm login` | `cd sdk/typescript && npm login && npm publish` |
| GitHub Release v0.2.2 | ✅ ARM64 binaries built | Check releases page |
| Show HN post | ❌ Ready to post | See `docs/launch/show-hn.md` — post Monday 9am ET |
| r/LocalLLaMA post | ❌ Ready | See `docs/launch/reddit-localllama.md` |
| HuggingFace Space | ❌ Needs HF account | Upload `sdk/gradio/app.py` + set HIPCORTEX_URL secret |
| VS Code marketplace | ❌ Needs publisher PAT | `cd vscode-extension && npm install && vsce publish` |

---

## Remaining Gaps (Phase 2 — gate on 500 stars)

Implemented so far from Phase 2:
- ✅ Prometheus /metrics
- ✅ hipcortex backup/restore CLI
- ✅ Helm chart
- ✅ HIPAA BAA template
- ✅ Namespace stub (GET /ns)
- ✅ Regulatory hold (POST /regulatory/hold)
- ✅ Token-aware search (max_tokens)
- ✅ Windsurf MCP support

Still pending:
- Full namespace/multi-tenancy isolation (5 days, needs >300 stars signal)
- OAuth 2.0 / OIDC
- SCIM provisioning
- GitHub Copilot Extension (different protocol from MCP, ~3 days)
- Semantic dedup ML (private algorithm)
- Cross-instance coherence
- World model REST API full exposure (commercial only)

---

## Quick Verification Commands

```bash
# Server health
curl https://hipcortex.fly.dev/health

# Zero-config ingest
curl -X POST https://hipcortex.fly.dev/memory/ingest \
  -H "X-Api-Key: hc-free-yGJcQexjZ4h8Mrd9" \
  -H "Content-Type: application/json" \
  -d '{"text": "Alice decided to use PostgreSQL"}'

# Build
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu check --no-default-features --features "web-server,petgraph_backend"

# Python SDK test
cd sdk/python && pytest tests/ -k "not integration" -v

# Deploy
C:\Users\user\.fly\bin\flyctl.exe deploy --app hipcortex --remote-only
```

---

## File Map (Key Files)

| Path | Purpose |
|------|---------|
| `src/web_server.rs` | All REST endpoints (~1800 lines) |
| `src/memory_record.rs` | MemoryRecord struct (confidence, source, version, tags, priority) |
| `src/memory_store.rs` | Storage: add, search, find_by_tags, find_latest, update, delete_by_actor |
| `src/audit_log.rs` | Merkle-chained audit (export, verify) |
| `src/openapi_spec.rs` | OpenAPI 3.0 static spec with operationIds |
| `src/commercial.rs` | ⚠️ LOCAL ONLY — LLM generate + world model predict |
| `sdk/python/hipcortex/cli.py` | Interactive install wizard + all CLI commands |
| `sdk/python/hipcortex/client.py` | HipCortexClient (sync) with remember/recall/create_node |
| `sdk/python/hipcortex/async_client.py` | AsyncHipCortexClient (httpx) |
| `sdk/python/hipcortex/install/SKILL.md` | Claude Code skill definition |
| `sdk/mcp/server.py` | MCP JSON-RPC 2.0 server for coding agents |
| `sdk/typescript/src/client.ts` | TypeScript SDK |
| `sdk/continue/index.ts` | Continue.dev context provider |
| `sdk/gradio/app.py` | Gradio UI for HuggingFace |
| `.tours/architect-hipcortex-overview.tour` | CodeTour walkthrough |
| `docs/launch/show-hn.md` | Show HN post (ready) |
| `docs/launch/reddit-localllama.md` | Reddit post (ready) |
| `docs/whitepaper.md` | arXiv-ready technical whitepaper |
| `deploy/helm/hipcortex/` | K8s Helm chart |
| `docs/compliance/HIPAA-BAA-template.md` | Enterprise HIPAA template |
| `Cargo.toml` | `commercial = ["web-server"]` feature flag |

---

## Next Session Priority Order

1. **Show HN + Reddit** — post `docs/launch/show-hn.md` Monday 9am ET (highest ΔV)
2. **npm publish** — `cd sdk/typescript && npm login && npm publish`
3. **MiroFish Sim #10** if needed to find remaining gaps
4. **GitHub Copilot Extension** — 3-day build, 3M users
5. **World model REST** — commercial-only, expose to hosted tier subscribers

---

## V(state) Current Estimate

```python
# Current state after all sessions:
S.arm64_shipped          = 1.0  # binaries on v0.2.2
S.async_client_shipped   = 1.0
S.ts_sdk_shipped         = 1.0  # npm pending publish
S.openapi_shipped        = 1.0  # with operationIds
S.bulk_add_shipped       = 1.0
S.ttl_shipped            = 1.0
S.auto_embed_shipped     = 1.0
S.autogen4_shipped       = 1.0
S.mcp_server_shipped     = 1.0
S.npm_published          = 0    # BLOCKING - needs npm login
S.pip_pypi_published     = 1.0  # DONE
S.github_release_created = 1.0
S.openapi_operation_ids  = 1.0
S.async_langchain        = 1.0
S.unified_search         = 1.0
S.graceful_shutdown      = 1.0
S.readme_order_fixed     = 1.0
S.zero_config_ingest     = 1.0  # POST /memory/ingest
S.install_wizard         = 1.0  # interactive multi-select
S.show_hn_posted         = 0    # HIGHEST ΔV REMAINING ACTION
S.github_stars           = 0    # nothing posted yet
```
