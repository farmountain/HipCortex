# HipCortex v0.4.9 — Extremely Detailed End-to-End User Testing Plan & Test Harness Engineering Guide

**For Execution by Autonomous Coding Agents (Claude Code, Cursor, Windsurf, Grok, etc.)**

**Repository:** https://github.com/farmountain/HipCortex  
**Current Version:** v0.4.9 (as of plan creation)  
**Plan Version:** 1.0  
**Date:** July 11, 2026  
**Status:** Ready for immediate implementation and execution  
**Audience:** Coding agents tasked with building, running, debugging, and reporting on a production-grade automated E2E user testing harness.

---

## 1. Executive Summary & Rationale

HipCortex is a high-performance, zero-dependency Rust-based **persistent causal topological memory engine** and cognitive substrate for autonomous AI agents. It delivers sub-millisecond operations (0.48–0.61 ms p50 writes on Linux), SHA-256 Merkle audit chains, recursive Bayesian world-model simulation (/worldmodel/rollout + MCTS), automatic FSM skill compilation, adaptive context budgeting (WorkingSetBroker delivering 59–93% token savings), and deep integrations with LangChain, LlamaIndex, AutoGen, CrewAI, VS Code (via MCP + chat participant), and multiple graph backends (petgraph default, Sled, Neo4j, Postgres, RocksDB).

**Current Testing Maturity (from TEST_VALIDATION_REPORT.md, BENCHMARK.md, architecture docs):**
- Strong **unit tests** (31/31 passing, core memory ops, temporal indexer, symbolic store, perception, etc.).
- **Benchmarks** exist and validate performance claims vs Mem0 (15–300× faster).
- **Integration & UAT pending** — exactly the gap this plan addresses.
- Advanced cognitive features (causal graphs, world models, hypothesis management, audit verification, SafetyGuardrails) have limited end-to-end user validation in realistic agent workflows.
- VS Code extension and framework adapters have basic coverage but lack full user-journey automation.

**This plan delivers:**
- A **complete, phased, prioritized E2E test strategy** focused on real user personas and journeys (Developer onboarding → daily coding with memory → advanced autonomous agent with world-model simulation and skill compilation).
- **Full test harness engineering blueprint** (Python pytest + supporting modules) that a coding agent can implement incrementally in the repo.
- **Actionable, copy-pasteable code patterns**, fixtures, data generators, custom assertions, and reporting.
- **50+ detailed test cases** (grouped, with templates, examples, automation notes, oracles where possible).
- **Execution playbook** so the coding agent can bootstrap, run targeted suites, debug failures using server logs/state, iterate, and produce professional reports (HTML/Allure + perf charts).
- Leverage of existing assets: `benchmarks/python_benchmark.py`, `tests/integration_tests.rs`, quick-start scripts, OpenAPI spec, Python SDK (`hipcortex` package), docker-compose, etc.

**Success Criteria (Quantitative):**
- 100% pass rate on all P0 (critical user paths) and ≥95% on P1.
- Performance assertions within 10–15% of published claims on target hardware.
- Token savings validated in simulated long sessions (Headroom/Caveman/Proactive modes).
- All major REST endpoints exercised (via SDK + raw HTTP where needed); discovery via `/openapi.json`.
- Persistence + Merkle audit chain verified across restarts/crashes.
- At least one full multi-turn agent journey per major integration (LangChain + CrewAI minimum).
- Clean CI-friendly execution; no flakiness from state leakage.
- Final report includes pass/fail matrix, latency distributions, token-savings charts, failure triage logs, and recommendations for v0.5+.

This harness turns "pending UAT/integration" into **continuously validated user trust**.

---

## 2. Test Strategy & Philosophy

**User-Centric Black-Box + Grey-Box E2E**
- Primary interface: Public contracts (Python/TS SDK, REST API, CLI `hipcortex`, VS Code `@hipcortex` commands, framework adapters).
- Grey-box: Direct HTTP to advanced endpoints (`/memory/reflect`, `/memory/live_beliefs`, `/worldmodel/rollout`, audit verification if exposed, or via storage inspection in harness).
- Property-based where feasible (synthetic causal chains, hypothesis graphs) using `hypothesis` (Python) or extend existing Rust `proptest`.
- **Shift-left + agent-executable**: Every test case is written so a coding agent can implement/run/fix in one flow.

**Harness Design Principles**
- **Isolation first**: Prefer fresh server instances or resettable state per major suite (or test class). Fallback: sequential execution + explicit cleanup.
- **Reproducible env**: Pin versions, use deterministic seeds for data generators, capture full server stdout/stderr + metrics.
- **Rich oracles**: Not just HTTP 200 — validate semantic correctness (causal path exists and is minimal, world-model rollout produces valid probability distributions and updates beliefs, token counts match expected savings, Merkle chain verifies after operations, FSM reachability, SafetyGuardrail logs no violations on valid flows).
- **Observable & debuggable**: Every failing test dumps server logs snippet, last N memory records, graph stats, current working set, and (if possible) Merkle root.
- **Extensible**: Markers (`@pytest.mark.core`, `@pytest.mark.worldmodel`, `@pytest.mark.integration_langchain`, `@pytest.mark.slow`, `@pytest.mark.perf`), easy addition of new journeys.
- **Leverage existing**: Wrap/extend `python_benchmark.py`, call `cargo test` for Rust core, use quick-start scripts as reference for server startup.

**Risk Mitigation**
- Stateful server → harness-managed lifecycle + optional reset endpoint contribution (or temp storage paths).
- Advanced feature oracles (MCTS, Bayesian updates) → synthetic ground-truth scenarios + statistical invariants + sample human review.
- VS Code UX → API/SDK primary; extension treated as high-value secondary with semi-automated checklist or VS Code test API.
- Integration deps (LangChain etc.) → optional extras or separate `test-requirements.txt`; mark and skip gracefully if missing.
- Hardware variance → record machine specs (CPU, RAM, OS) in every report; have relative thresholds.

---

## 3. Personas & Key User Journeys

**Primary Personas**
1. **Solo Developer / Power User** (VS Code + Claude Code / Cursor): Installs, configures MCP, uses memory in daily coding sessions for decisions, code patterns, debugging history. Relies on context budgeting + @hipcortex chat.
2. **Agent Framework Engineer** (LangChain / CrewAI / AutoGen): Builds persistent memory into multi-turn agents/crews. Expects drop-in replacements that survive restarts and improve reasoning over sessions.
3. **AI Researcher / Autonomous Systems Builder**: Heavy use of world-model rollout for "what-if" simulation, causal inference (backdoor adjustment, hypothesis graphs via AureusBridge/HypothesisManager), FSM skill compilation from procedural traces, live_beliefs surface, reflect/ingest for active inference loops.
4. **DevOps / Production Deployer**: Docker/Fly.io/binary deployment, monitoring (stats, dashboard), audit/Merkle verification for compliance/tamper-evidence, long-running stability, backend switching (petgraph → Postgres).

**Core End-to-End Journeys (to be automated as orchestrated test scenarios)**
- **J1: Developer Onboarding & Daily Workflow** (P0) — pip/CLI install → server start → SDK add/search → VS Code @hipcortex usage → LangChain agent with memory → restart persistence check → token savings measurement.
- **J2: Persistent Multi-Turn Agent Loop** (P0) — CrewAI/LangChain agent performs complex task over 10–20 turns; memory added automatically or via tools; later turns recall early facts/decisions without full history injection.
- **J3: World-Model Simulation & Causal Reasoning** (P1) — Researcher adds observation sequence → triggers /worldmodel/rollout with policy → inspects predicted futures + belief updates via /memory/live_beliefs → records hypothesis via /memory/reflect → verifies causal paths and audit chain.
- **J4: Skill Compilation & Procedural Memory** (P1) — Agent executes repeated successful traces → triggers FSM compilation (via LoopEngine/Omega) → later recalls compiled skill/procedure efficiently.
- **J5: Production-like Load + Recovery** (P1) — High-volume writes/queries + crash (kill -9) + restart + Merkle verify + no data loss + continued operation.
- **J6: Context Optimization Validation** (P0) — Simulated 30–50 turn coding session; compare Headroom / Caveman / Proactive substrate modes vs full history; assert token savings and reasoning quality proxy (e.g., successful task completion rate).

---

## 4. Test Harness Architecture & Implementation Blueprint

**Recommended Location in Repo**
```
tests/e2e_user_harness/
├── __init__.py
├── conftest.py                 # pytest fixtures (server, client, data)
├── server_manager.py           # Lifecycle: download/build/start/stop/health/poll + logging capture
├── client_factory.py           # HipCortexClient + raw httpx wrappers + session helpers
├── data_generators.py          # Synthetic realistic traces (coding decisions, causal chains, multi-actor)
├── assertions.py               # Custom: causal_path_exists, worldmodel_valid, token_savings, merkle_verify, etc.
├── reporters.py                # Allure steps, perf plots (matplotlib/seaborn), JSON/HTML summary
├── scenarios/
│   ├── journey_developer_onboarding.py
│   ├── journey_agent_loop.py
│   ├── journey_worldmodel_causal.py
│   └── ...
├── test_core_memory_ops.py
├── test_advanced_cognitive.py
├── test_integrations_*.py
├── test_persistence_audit.py
├── test_performance.py
├── test_error_guardrails.py
├── test_vscode_extension_checklist.py  # or manual
└── README_harness.md           # Auto-generated or maintained usage for agents
```

**Key Modules (Coding Agent Should Implement in This Order)**

### 4.1 server_manager.py (Highest Priority Enabler)
```python
import subprocess, time, requests, psutil, tempfile, shutil, os
from contextlib import contextmanager
from pathlib import Path
import tenacity

class HipCortexServerManager:
    def __init__(self, binary_path: str | None = None, port: int = 3030, storage_dir: Path | None = None, 
                 features: str = "web-server,petgraph_backend", build_from_source: bool = False):
        self.port = port
        self.storage_dir = storage_dir or Path(tempfile.mkdtemp(prefix="hipcortex_test_"))
        self.process = None
        self.binary_path = binary_path or self._resolve_binary(build_from_source, features)
        self.log_file = self.storage_dir / "server.log"

    def _resolve_binary(self, build: bool, features: str) -> Path:
        if build:
            # cargo build --bin webserver --features "..."
            # return target/debug/webserver or release
            pass
        # else: download from release or use system hipcortex or docker exec
        # For simplicity in v1: assume `hipcortex` in PATH or explicit binary
        return Path(shutil.which("hipcortex") or "./hipcortex")

    @tenacity.retry(stop=tenacity.stop_after_attempt(10), wait=tenacity.wait_fixed(0.5))
    def wait_healthy(self):
        r = requests.get(f"http://127.0.0.1:{self.port}/health", timeout=2)
        assert r.status_code == 200 and r.json().get("status") == "ok"

    def start(self):
        env = os.environ.copy()
        env["HIPCORTEX_STORAGE"] = str(self.storage_dir)  # if supported; else document assumption
        cmd = [str(self.binary_path), "start", "--port", str(self.port)]  # adjust to actual CLI
        self.process = subprocess.Popen(cmd, stdout=open(self.log_file, "w"), stderr=subprocess.STDOUT, env=env)
        self.wait_healthy()
        return self

    def stop(self):
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except:
                self.process.kill()
        # optional: shutil.rmtree(self.storage_dir) in teardown if desired

    @contextmanager
    def running(self):
        self.start()
        try:
            yield self
        finally:
            self.stop()
```

**Enhancements for Agent:**
- Support Docker: `docker run -p {port}:3030 -v {storage}:/data hipcortex:latest`
- Auto-detect/build from source using `cargo` when in repo root.
- Capture full logs on failure.
- Expose `get_stats()`, `trigger_reflect()`, etc.

### 4.2 conftest.py (Core Fixtures)
```python
import pytest
from .server_manager import HipCortexServerManager
from hipcortex import HipCortexClient  # or from hipcortex import ...

@pytest.fixture(scope="session")
def hipcortex_server():
    mgr = HipCortexServerManager(port=3031)  # use unique port per session or class
    with mgr.running():
        yield mgr

@pytest.fixture
def client(hipcortex_server):
    return HipCortexClient(f"http://127.0.0.1:{hipcortex_server.port}")

@pytest.fixture
def raw_client(hipcortex_server):
    import httpx
    return httpx.Client(base_url=f"http://127.0.0.1:{hipcortex_server.port}", timeout=30)

@pytest.fixture(scope="function")
def clean_state(client):
    # If API supports bulk forget or admin clear: client.forget_all() or POST /admin/reset
    # Otherwise: rely on fresh server per class or explicit cleanup of known actors
    yield
    # post-test cleanup
```

**Note on Isolation:** Inspect OpenAPI or source for reset capability. If none, implement in harness or contribute a test-only endpoint. For now, many tests can use unique `actor` namespaces or run sequentially.

### 4.3 data_generators.py
Generate realistic traces:
- Coding session: file edits, errors, decisions, "user said X", "agent planned Y because Z", causal links via metadata or explicit fields.
- Multi-actor (alice/bob/researcher).
- Temporal sequences with timestamps.
- For causal: sequences that form DAGs with known ground-truth paths.
- For worldmodel: simple known transition models (e.g., gridworld toy) for oracle comparison.

Use `faker`, `datetime`, `random` with seeds.

### 4.4 assertions.py (Examples)
```python
def assert_add_success(response_or_record):
    assert response_or_record.get("success") or response_or_record.status_code == 200
    assert "id" in record and "created_at" in record and "hash" in record

def assert_causal_path_exists(client, from_id: str, to_id: str, max_depth: int = 5):
    # Use /causal/paths or live_beliefs + graph traversal; or raw query
    # For v1: search with causal filters or inspect SymbolicStore if accessible
    pass  # Implement via known endpoints or storage inspection

def assert_worldmodel_rollout_valid(rollout_response):
    assert "predicted_states" in rollout_response or "belief_updates" in rollout_response
    # Check probabilities sum to ~1, states are valid dicts, etc.

def assert_token_savings(mode: str, original_tokens: int, optimized_tokens: int, min_savings_pct: float):
    savings = (original_tokens - optimized_tokens) / original_tokens
    assert savings >= min_savings_pct, f"{mode} mode only achieved {savings:.1%} (expected ≥{min_savings_pct:.0%})"

def assert_merkle_chain_valid(storage_dir: Path):
    # Parse audit.log or Merkle root files; re-compute hashes; verify links
    # Or call any exposed /audit/verify endpoint
    pass
```

### 4.5 reporters.py & pytest Configuration
- Use `pytest-html` or **Allure** (`allure-pytest`) for beautiful step-by-step E2E reports with attachments (logs, charts, server state).
- Custom hook to attach server.log tail + stats JSON on failure.
- Performance: integrate `pytest-benchmark` or custom timing context manager + save CSV + plots.
- Final summary script that aggregates to Markdown/JSON for the coding agent to present.

**pytest.ini or pyproject.toml additions**
```ini
[pytest]
markers =
    core: Core memory CRUD & search
    worldmodel: World model rollout & live beliefs
    causal: Causal topological graph & inference
    integration: Framework integrations (LangChain, CrewAI...)
    persistence: Restart, recovery, audit/Merkle
    perf: Performance & load (may be slow)
    vscode: VS Code extension flows (semi-auto)
addopts = --strict-markers -ra --tb=short
```

---

## 5. Phased Test Catalog (Prioritized)

### Phase 0: Environment, Install & Bootstrap (P0 — Blocking)
**Goal:** Every install path works cleanly; server starts reliably; basic CLI/SDK functional.

**TCs (examples — coding agent expands to 10–12):**
- **TC-INST-001 (pip + CLI non-interactive)**: Clean venv → `pip install hipcortex` → `hipcortex install --yes` → verify configs written for VS Code + LangChain starter file created → `hipcortex --help` + `hipcortex start` (background) → health OK.
- **TC-INST-002 (Docker)**: `docker run -d -p 3030:3030 hipcortex:latest` → health + stats endpoint.
- **TC-INST-003 (Build from source)**: In repo → `cargo build --bin webserver --features "web-server,petgraph_backend"` → run binary → health.
- **TC-INST-004 (VS Code extension)**: Install .vsix or via marketplace → restart VS Code → open chat → type `@hipcortex help` → commands listed (add, search, forget, stats...).
- **TC-INST-005 (Cross-platform sanity)**: At minimum Linux (primary); note Windows differences in paths/timings.
- Edge: Already installed, re-install, invalid flags, missing deps for optional features.

**Automation:** Fully scriptable with subprocess + assertions on files created + HTTP health. Use `test_install_flows.py`.

### Phase 1: Core Memory Operations (P0)
**TC-CORE-ADD-001 to TC-CORE-SEARCH-012, etc. (15–20 cases)**

**Sample Detailed Case:**
**TC-CORE-ADD-PERSIST-001**  
**Title:** Add memory via Python SDK → immediate retrieve → restart server → still retrievable + hash intact  
**Preconditions:** Clean server instance.  
**Steps (in test code):**
1. `client = HipCortexClient(...)`
2. `rec = client.add_memory(actor="dev_alice", action="decided", target="Adopt HipCortex for persistent memory in all agents", metadata={"confidence": 0.95, "project": "e2e-test"})`
3. `assert_add_success(rec)`; store `rec["id"]`
4. `results = client.search("persistent memory", limit=5, actor="dev_alice")`; assert rec in results with high relevance.
5. Capture current `/stats` and Merkle root (if exposed) or storage files.
6. `server_manager.stop()`; sleep 1s; `server_manager.start()` (same storage_dir).
7. New client; repeat search → record present with identical content + hash.
8. Optional: `assert_merkle_chain_valid(storage_dir)`

**Similar cases for:**
- Bulk add (with/without TTL)
- Forget (by actor, by pattern, selective)
- Search with filters (time range, action type, causal depth, metadata)
- Pagination & large result sets
- Concurrent adds (threading/asyncio) — eventual consistency or strong?
- Error injection: missing actor/action/target, huge payload, invalid UTF-8, SQL injection attempts (guardrails)
- `/memory/ingest` and `/memory/reflect` for high-fidelity/reflexion paths
- Live beliefs surface: `GET /memory/live_beliefs` returns coherent current world state

**Automation Level:** 95%+ fully automated.

### Phase 2: Advanced Cognitive Features (P1 — Core Differentiator)
**Focus Areas:** CausalTopoGraph, WorldModelEnhanced + MCTS rollout, HypothesisManager/AureusBridge, LoopEngine/FSM compilation, WorkingSetBroker token optimization, SafetyGuardrail, AuditLog::verify.

**Key TCs:**
- **TC-COG-CAUSAL-001:** Add causally linked sequence (explicit or via temporal + actor patterns) → query predecessors/successors/paths → assert expected minimal causal path exists and backdoor adjustment logic (if exposed) produces sensible adjustment sets.
- **TC-COG-WORLDMODEL-001:** POST `/worldmodel/rollout` with initial beliefs + action sequence or policy + horizon → validate returned structure (states, probs, expected rewards?, belief updates) → re-query `/memory/live_beliefs` shows updated world model.
  - Oracle: Use simple known environment (e.g., multi-armed bandit or grid) where harness can compute ground-truth rollout for comparison within tolerance.
- **TC-COG-HYPOTHESIS-001:** Use `/memory/reflect` or HypothesisManager flows → add conflicting observations → verify pruning of low-confidence branches, best_path selection, DOT export if applicable.
- **TC-COG-FSM-001:** Repeated successful procedural traces (via add or perception hooks) → trigger skill compilation (if explicit API or automatic via LoopEngine) → later recall uses compiled FSM efficiently (lower latency or fewer records surfaced).
- **TC-COG-TOKEN-001 (Critical for value prop):** Simulate 30-turn coding session with realistic memory volume.
  - Mode A: Full history injection (baseline tokens via tiktoken).
  - Mode B: Headroom (Top-5 or WorkingSetBroker default) → assert ≥59–63% savings.
  - Mode C: Caveman (Top-3) → ≥69% savings.
  - Mode D: Proactive substrate (live_beliefs direct query) → ≥93% savings.
  - Proxy for quality: Agent using optimized context still completes task successfully.
- **TC-COG-AUDIT-001:** Perform writes → call audit verify (or harness re-computes Merkle) → tamper (manually flip a bit in storage if safe) → verify detects corruption → recovery via WAL/snapshot if supported.

**Automation Notes:** Many require raw `raw_client.post("/worldmodel/rollout", json=...)`. For oracles, start simple (Markov toy models) and expand. SafetyGuardrail: inject invalid mutations and assert they are blocked + logged.

### Phase 3: Framework Integrations E2E (P0)
**TC-INT-LANG-001 (LangChain):**
- Instantiate `HipCortexMemory(session_id=..., url=...)` or `AsyncHipCortexMemory`.
- Use in `ConversationChain` or custom agent with tools.
- Run 12-turn conversation where early facts ("project deadline is Sep 30", "budget approved by alice") must be recalled accurately in turn 10–12 without stuffing full history.
- Assert memory records were created (via separate client query) and context stayed lean.
- Repeat with restart of server mid-conversation.

**TC-INT-CREW-001 (CrewAI):**
- Create crew with `HipCortexRememberTool` + `HipCortexRecallTool`.
- Task requires collaboration across agents and persistence (e.g., "Research X, remember findings, later agent uses them to decide Y").
- Execute crew.kickoff() multiple times or long session; verify cross-run recall.

**Similar for AutoGen (v0.4 protocol), LlamaIndex ChatStore.**

**Automation:** Each in own file under `scenarios/`. Install optional deps or `pytest.importorskip("langchain")`. Provide minimal working agent examples in harness.

### Phase 4: VS Code Extension & IDE Workflows (P1)
- Launch VS Code with extension loaded (via CLI `code --install-extension ... --force` + user data dir).
- Basic: Chat participant registration, `@hipcortex help`, add/search/forget commands return sensible output in chat.
- Integration: In a workspace with long chat history, use memory to reduce effective context (proxy: successful code gen that references earlier decisions).
- Dynamic headroom / context optimization visible in extension UI or logs.
- **Automation Level:** Partial. Use `pytest` + subprocess for launch + basic command execution. Full UX (typing, seeing responses) may require Playwright + VS Code desktop or remain as guided checklist for coding agent to execute manually + screenshot.

### Phase 5: Persistence, Recovery, Audit & Production Readiness (P0/P1)
- Restart, crash (SIGKILL during write), power-loss simulation (if storage on tmpfs).
- Merkle chain continuous verification.
- Backend switch (if CLI/env supports): petgraph → sled/RocksDB → verify data roundtrip.
- Long-running stability (hours simulated via accelerated time or loop): no OOM, no audit corruption, graceful degradation.
- Stats / monitoring endpoints accurate under load.
- Docker/Fly.io deployment parity (if harness runs in container).

### Phase 6: Performance, Load & Scalability (P0 — Validate Claims)
- Re-execute and assert on `benchmarks/python_benchmark.py` results (p50/p95 within thresholds on same hardware class).
- Concurrent load: 50–200 clients hammering add/search (asyncio or locust).
- Worldmodel rollout perf at increasing horizons/depths.
- Token reduction benchmark extended into pytest with assertions + charts saved to `reports/`.
- Memory/CPU profiling (optional, `memory_profiler` or `py-spy`).

**Pass Criteria Example:** Linux test env: add_p50 ≤ 0.8 ms, query_p50 ≤ 0.4 ms (allowing some variance).

### Phase 7: Error Handling, Guardrails & Edge Cases (P1)
- All error paths return structured, safe errors (no stack traces to client).
- SafetyGuardrail blocks/prevents invalid state mutations.
- Rate limiting, input validation (XSS, injection, oversized).
- Resource exhaustion (huge graphs, very long causal chains, high-cardinality metadata).
- Unicode/emoji, very long strings, null bytes.
- Missing optional features gracefully degrade.

### Phase 8: Full Orchestrated Journeys (Highest Value — Run Last or Nightly)
Implement `test_journey_*.py` that compose multiple phases into realistic user stories. Use Allure `@allure.step` or `pytest-bdd` for readability. These are the ultimate "user testing" proof.

---

## 6. Execution Playbook for Coding Agent

**Step-by-Step Bootstrap (Copy into your first response/actions):**

1. **Repo Setup**
   ```bash
   git checkout -b feat/e2e-user-testing-harness-v1
   mkdir -p tests/e2e_user_harness/{scenarios,reporters}
   ```

2. **Python Env (recommended: uv or venv + pip)**
   ```bash
   python -m venv .venv_e2e
   source .venv_e2e/bin/activate
   pip install -U pip pytest pytest-html pytest-benchmark hypothesis httpx psutil tenacity rich matplotlib seaborn pandas
   pip install hipcortex  # or -e if SDK in repo
   # Optional for integrations
   pip install langchain crewai autogen "langchain[all]"  # or use extras
   ```

3. **Implement in Order (Agent Tasks)**
   - `server_manager.py` + basic health/start/stop (test manually first with `python -c "from ... import *; mgr=...; mgr.start()..."`)
   - `conftest.py` + one core test file (`test_core_memory_ops.py`) with 5–6 happy-path + persistence TCs.
   - Run: `pytest tests/e2e_user_harness/test_core_memory_ops.py -q --tb=line`
   - Add data_generators + assertions incrementally.
   - Extend to advanced cognitive (start with live_beliefs + simple rollout).
   - Add integration tests (one framework at a time).
   - Wire performance assertions around existing benchmark script.
   - Add journey orchestrators.
   - Add reporting (Allure recommended: `pytest ... --alluredir=reports/allure && allure serve reports/allure`).

4. **Running Specific Suites**
   ```bash
   pytest tests/e2e_user_harness/ -m "core and not slow" -q
   pytest ... -m "worldmodel" --alluredir=reports/allure
   pytest ... -m perf --benchmark-only
   ```

5. **On Failure (Debug Loop)**
   - Read `server.log` tail.
   - Print `client.get_stats()` or raw `/stats`.
   - Inspect storage_dir contents (JSONL? SQLite? audit.log).
   - Reproduce minimal failing sequence in Python REPL.
   - Check OpenAPI: `raw_client.get("/openapi.json").json()` for exact schemas/endpoints.
   - Update test or open issue/PR with minimal repro.

6. **Final Deliverables from Agent**
   - All code committed + passing on Linux (primary) + note Windows.
   - `reports/` with HTML/Allure + perf PNGs + `E2E_Execution_Report.md` (auto-generated summary: pass matrix, key metrics, top issues found, harness coverage).
   - Updated `TEST_VALIDATION_REPORT.md` or new `E2E_UAT_REPORT.md`.
   - Any bugs found triaged with minimal repros.
   - Recommendations for production (e.g., "add /admin/reset endpoint", "expose Merkle verify API", "improve error messages for X").

---

## 7. Metrics, Reporting & Continuous Improvement

**Core Metrics Tracked**
- Functional pass rate by phase/marker.
- Latency distributions (p50/p95/p99) per operation type.
- Token savings % by mode vs baseline.
- Server resource usage (RSS, open FDs) under load.
- Audit/Merkle verification success rate.
- Integration task success rate (agent completes goal using memory).

**Reporting Stack**
- Allure for rich E2E step history + attachments.
- Custom `reporters.py` → Markdown summary + charts.
- GitHub Actions (add workflow) for nightly E2E on main (Linux + optional Windows/macOS runners).

**Evolution**
- This plan lives in repo as living doc.
- Coding agent updates plan + harness as new features (v0.5 worldmodel enhancements, new backends, WASM plugins, etc.) are added.
- Property-based tests expanded for causal invariants and hypothesis pruning rules.

---

## 8. Open Questions / Items for Coding Agent to Clarify or Contribute

1. Exact CLI flags for `hipcortex start` / storage path / backend selection? (Inspect `src/main.rs` or run `--help`).
2. Full list of REST endpoints + request/response schemas? (Always start harness by fetching `/openapi.json`).
3. Is there an explicit API for causal path queries, FSM compilation trigger, or Merkle verify? If not exposed, does harness inspection of storage + internal Rust calls (via pyo3? advanced) or contribute endpoints?
4. How are causal links explicitly created vs inferred? (metadata? separate /causal/link endpoint?)
5. VS Code extension test support (official test API or recommended pattern)?
6. Multi-tenancy / session isolation model (actor vs explicit session_id)?
7. Encryption / WAL / snapshot features maturity for recovery tests?

The coding agent should explore these via code reading (`read_file` on src/, docs/), running the binary, and `/openapi.json` — then document findings in harness README or update this plan.

---

## 9. Conclusion

This plan transforms HipCortex's strong core and benchmarks into **provably reliable user experiences** across installation, daily developer use, sophisticated agent reasoning with world models and causal memory, and production deployment.

A capable coding agent following this blueprint will deliver:
- A maintainable, observable, extensible automated harness.
- High-confidence validation of the unique value propositions (speed + causal depth + token efficiency + auditability).
- Actionable feedback loop for the HipCortex maintainers.

**Next Immediate Action for Coding Agent:**  
Implement `server_manager.py` and the first 5 core tests. Run them. Report back with results and any blockers discovered. Then expand phase by phase.

**This is the foundation for trustworthy, production-grade autonomous cognitive memory.**

---

*Plan generated with deep analysis of repo structure, existing reports (TEST_VALIDATION_REPORT, BENCHMARK, architecture/usage docs), performance claims, and user value streams. Ready for execution.*

**File saved to:** `/home/workdir/artifacts/HipCortex_E2E_User_Testing_Plan.md`

You (the coding agent or user) can now:
- Read this file.
- Start implementing the harness in the HipCortex repo.
- Use the sandbox here if needed for local validation (though internet is disabled, so binary/build must be prepared externally).
- Iterate and expand the test catalog with even more granular cases as you discover exact API shapes.

Let me know the first component you'd like me to help flesh out with more code (e.g., full server_manager with Docker support, sample journey test, or specific assertions for worldmodel), or if you want me to generate additional files like a starter `test_core_memory_ops.py` or `pytest.ini` updates. 

We can also create supporting docs (e.g., `tests/e2e_user_harness/README.md`) or even a lightweight Allure config.

Ready when you are — let's engineer this harness to extremely high standards.