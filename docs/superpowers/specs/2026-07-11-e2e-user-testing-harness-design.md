# HipCortex v0.4.9 E2E User Testing Harness Design & Token Optimization Specification

**Date:** 2026-07-11  
**Status:** Approved for Implementation Planning  
**Target Repository:** `tests/e2e_user_harness/`  
**Reference Document:** `HipCortex_E2E_User_Testing_Plan.md`  

---

## 1. Executive Summary & Scope

This specification defines the complete architecture, test suite layout, and execution mechanics for the **HipCortex v0.4.9 End-to-End User Testing Harness** (`tests/e2e_user_harness/`). The harness bridges the gap between strong unit test coverage (31/31 passing in `TEST_VALIDATION_REPORT.md`) and production user acceptance testing (UAT).

A core focus of this harness is rigorous, multi-turn verification of HipCortex's adaptive context budgeting and token optimization capabilities (`WorkingSetBroker`), specifically:
- **Headroom Mode (`Top-5` Retrieval Guarantee):** Verifying exact `cl100k_base` token savings bounded between **`[59.0%, 89.0%]`**.
- **Caveman Mode (`Top-3` Retrieval Guarantee):** Verifying exact `cl100k_base` token savings bounded between **`[70.0%, 92.0%]`**.
- **Proactive Substrate Mode:** Verifying up to **`>= 93.0%`** context reduction via direct `live_beliefs` queries.

The harness implements a **Layered Suite-by-Phase Architecture (Approach 1)**, guaranteeing 100% test isolation via automated `cargo build --bin webserver` binary checks, dynamic loopback port allocation (`3031+`), and ephemeral temporary storage directories (`tempfile.mkdtemp`) for every test class/suite.

---

## 2. Directory & Architecture Structure

```
tests/e2e_user_harness/
├── __init__.py
├── conftest.py                 # Session/class scoped pytest fixtures & state isolation hooks
├── server_manager.py           # Auto-build & child process lifecycle (`cargo build --bin webserver`)
├── client_factory.py           # Typed HipCortexClient + raw httpx wrapper + Allure logging hooks
├── data_generators.py          # Synthetic multi-turn coding session & causal DAG generators
├── assertions.py               # Custom Headroom/Caveman token savings, Merkle audit & causal invariants
├── reporters.py                # Allure step wrappers, markdown summary generator & matplotlib charts
├── scenarios/                  # Phase 8 multi-turn user journey orchestrators
│   ├── __init__.py
│   ├── journey_developer_onboarding.py
│   ├── journey_context_optimization.py
│   └── journey_worldmodel_causal.py
└── suites/                     # Phase 0 through Phase 7 modular test suites
    ├── __init__.py
    ├── test_phase0_bootstrap.py
    ├── test_phase1_core_memory_ops.py
    ├── test_phase2_cognitive_and_token_optimization.py
    ├── test_phase3_integrations.py
    ├── test_phase4_vscode_ide_workflows.py
    ├── test_phase5_persistence_and_audit.py
    ├── test_phase6_perf_and_scalability.py
    └── test_phase7_guardrails.py
```

---

## 3. Core Infrastructure Design

### 3.1 `server_manager.py` (Server Lifecycle & Isolation Engine)
- **Auto-Build Gate (`session` scope):** On startup, `HipCortexServerManager` checks binary timestamps. If modified or missing, it invokes `subprocess.run(["cargo", "build", "--bin", "webserver", "--features", "web-server,petgraph_backend"], check=True)`.
- **Dynamic Port Assignment (`socket`):** Binds briefly to port `0` to secure a free local loopback port (`3031..3999`), avoiding collisions with standard port `3030`.
- **Ephemeral Storage (`tempfile.mkdtemp`):** Provisions a dedicated `storage_dir` (`hipcortex_test_{port}_{timestamp}`) passed via `HIPCORTEX_STORAGE` and `--port`.
- **Health Polling (`tenacity`):** Polls `http://127.0.0.1:{port}/health` with exponential backoff up to 5 seconds. Captures `stdout` and `stderr` directly into `storage_dir/server.log`.
- **Teardown & Log Attachments:** Sends `SIGTERM` on exit (falling back to `SIGKILL`). If a test fails, `reporters.py` automatically attaches `server.log` tail (`100` lines) and `stats.json` (`GET /stats`) to the Allure report before deleting or preserving (`--keep-storage-on-fail`) the storage directory.

### 3.2 `conftest.py` & `client_factory.py`
- **Fixtures:**
  - `@pytest.fixture(scope="session") def hipcortex_binary()`: Guarantees binary compilation across multi-threaded workers.
  - `@pytest.fixture(scope="class") def hipcortex_server()`: Spawns an isolated `HipCortexServerManager` per test class.
  - `@pytest.fixture(scope="function") def client(hipcortex_server)`: Returns a typed `HipCortexClient`.
  - `@pytest.fixture(scope="function") def raw_client(hipcortex_server)`: Returns an `httpx.Client` configured with custom timeouts (`30s`) and Allure request/response interception hooks.

---

## 4. Data Generators & Custom Assertions

### 4.1 `data_generators.py`
- **`CodingSessionTraceBuilder(turns=30, actors=["dev_alice", "agent_claude"])`:** Synthesizes realistic IDE coding traces (`"read_file"`, `"compile_error"`, `"refactor_decision"`, `"user_instruction"`) with deterministic token counts (`~150-300 cl100k_base` tokens per record) and multi-tiered tags (`WorkingSet`, `ShortTerm`, `LongTerm`).
- **`CausalDagBuilder(depth=5, branch_factor=2)`:** Generates synthetic causal chains where memory $M_k$ references predecessor $M_{k-1}$ or connects via shared semantic/temporal targets, producing verifiable Directed Acyclic Graphs with known shortest paths.
- **`WorldModelTransitionBuilder(states=10, actions=4)`:** Generates initial belief distributions and transition matrices (e.g., gridworld or multi-armed bandit states) to provide exact ground-truth expectations for `/worldmodel/rollout`.

### 4.2 `assertions.py`
- **`assert_token_savings_bounds(baseline_text: str, retrieved_records: list[dict], mode: str)`:**
  - Calculates `baseline_tokens = estimate_tokens(baseline_text)` (`cl100k_base` via exact formula parity with `TokenTracker.ts`).
  - Calculates `used_tokens` from retrieved payloads and computes `savings_pct = (baseline_tokens - used_tokens) / baseline_tokens * 100`.
  - **Headroom Mode (`Top-5`):** Asserts `len(retrieved_records) <= 5` and `59.0% <= savings_pct <= 89.0%`.
  - **Caveman Mode (`Top-3`):** Asserts `len(retrieved_records) <= 3` and `70.0% <= savings_pct <= 92.0%`.
- **`assert_causal_path_valid(client, source_id: str, target_id: str, expected_min_length: int)`:** Queries `/causal/paths` or graph endpoints to assert that a directed path exists between `source_id` and `target_id`, matching `expected_min_length` with zero cycles.
- **`assert_merkle_chain_integrity(storage_dir: Path, expected_record_count: int)`:** Parses `audit.log` or persisted storage logs inside `storage_dir`, recomputing SHA-256 hashes (`hash[i] = sha256(hash[i-1] + content[i])`) to verify 100% cryptographic link continuity across operations and restarts.
- **`assert_worldmodel_rollout_valid(rollout_resp: dict, expected_states: list[str], tolerance: float = 0.05)`:** Asserts that predicted state probabilities sum to `1.0 ± tolerance` and align with expected transitions.

---

## 5. Phase-by-Phase Suite Specifications (`suites/`)

### 5.1 `test_phase0_bootstrap.py` (P0 — Environment, Install & Server Lifecycle)
- `test_pip_cli_install_flow`: Verifies `pip install hipcortex`, `hipcortex install --yes`, and checks VS Code extension/LangChain starter file generation.
- `test_binary_auto_build_and_health`: Verifies sub-500ms startup and clean `GET /health` JSON response (`version="0.4.9"`).
- `test_port_collision_and_recovery`: Verifies clean error handling on port binding collisions without hanging processes.

### 5.2 `test_phase1_core_memory_ops.py` (P0 — Core CRUD & Search)
- `test_add_and_retrieve_persistence`: Verifies multi-tiered metadata insertion, search retrieval exact content match, and SHA-256 hash generation.
- `test_query_filtering_and_pagination`: Inserts 50 records; asserts exact filtering by `actor`, `action`, time window, and pagination parameters (`limit=10`, `offset=0/10`).
- `test_bulk_forget_and_selective_pruning`: Exercises forget endpoints by actor/pattern/ID and confirms zero retrieval of pruned items without affecting adjacent records.

### 5.3 `test_phase2_cognitive_and_token_optimization.py` (P1/P0 — Cognitive Differentiators)
- `test_headroom_vs_caveman_token_savings` (`TC-COG-TOKEN-001`): Populates 20+ multi-turn coding session records; asserts Top-5 (`Headroom Mode`, `59-89%` savings) and Top-3 (`Caveman Mode`, `70-92%` savings) bounds and formula parity (`Math.max(0, baseline - used) / baseline * 100`).
- `test_causal_topological_graph_traversal` (`TC-COG-CAUSAL-001`): Inserts causal sequence $A \to B \to C \to D$; verifies shortest path queries and causal depth invariants.
- `test_worldmodel_rollout_and_live_beliefs` (`TC-COG-WORLDMODEL-001`): POSTs `/worldmodel/rollout` with initial beliefs/policy; asserts predicted state probabilities sum to `1.0` and `GET /memory/live_beliefs` reflects updated world model.
- `test_fsm_skill_compilation_trigger` (`TC-COG-FSM-001`): Inserts repeated procedural traces; triggers FSM skill compilation (`LoopEngine`); verifies subsequent queries recall compiled rules with reduced latency.

### 5.4 `test_phase3_integrations.py` (P0 — Framework Adapters)
- `test_langchain_conversation_memory_loop` (`TC-INT-LANG-001`): Uses LangChain `ConversationChain` memory adapter (`pytest.importorskip("langchain")`) across 12 turns; verifies accurate recall of turn 1 facts (`"deadline Sep 30"`) at turn 12 without full prompt stuffing.
- `test_crewai_multi_agent_collaboration` (`TC-INT-CREW-001`): Simulates two CrewAI agents sharing a namespace (`actor="crew_research"`); verifies Agent B recalls findings persisted by Agent A across independent execution turns.

### 5.5 `test_phase4_vscode_ide_workflows.py` (P1 — IDE & `@hipcortex` Chat)
- `test_vscode_extension_command_parity`: Verifies REST commands issued by the VS Code extension (`/memory/add`, `/memory/query`, `/stats`) process inputs cleanly from `@hipcortex` chat participant commands (`add`, `search`, `forget`).
- `test_ide_context_budgeting_telemetry`: Simulates rapid IDE file switching and memory updates; checks that backend telemetry correctly reports working set pruning and token reduction metrics.

### 5.6 `test_phase5_persistence_and_audit.py` (P0/P1 — Recovery & Tamper Evidence)
- `test_server_crash_recovery_and_wal`: Inserts records, sends `SIGKILL` (`kill -9`), restarts on same `storage_dir`, and asserts 100% record recoverability.
- `test_merkle_audit_chain_verification` (`TC-COG-AUDIT-001`): Invokes `assert_merkle_chain_integrity(storage_dir)` across sequential additions and deletions to guarantee cryptographic chain continuity.

### 5.7 `test_phase6_perf_and_scalability.py` & `test_phase7_guardrails.py`
- **Phase 6 Performance:** Asserts local roundtrip HTTP latency `< 5ms` (`p50 <= 0.8ms` on Linux) and checks concurrent async requests (`50+` parallel workers) without lock contention or deadlocks.
- **Phase 7 Guardrails:** Injects oversized payloads, malformed JSON, command injection strings, and invalid UTF-8 bytes; verifies `SafetyGuardrail` blocks invalid mutations with clean `400/422` responses and zero server crashes.

---

## 6. Multi-Turn Scenario Orchestrators & Reporting (`scenarios/` & Allure)

### 6.1 Phase 8 Multi-Turn Journey Orchestrators (`scenarios/`)
- **`journey_developer_onboarding.py` (J1):**
  1. Bootstraps local server (`server_manager.py`) and configures `HipCortexClient`.
  2. Simulates 15 sequential IDE file reads, error diagnoses, and architecture decisions stored via `/memory/add`.
  3. Queries memory via `@hipcortex search` patterns, verifying sub-millisecond retrieval of exact code snippets.
  4. Executes mid-session server restart (`server_manager.stop()` $\to$ `.start()`) and proves zero data loss.
- **`journey_context_optimization.py` (J6 — Headroom & Caveman Deep-Dive):**
  1. Simulates a 50-turn intensive coding session (`CodingSessionTraceBuilder`), populating 10,000+ baseline tokens across `WorkingSet`, `ShortTerm`, and `LongTerm` tiers.
  2. **Headroom Mode Verification:** Queries `/memory/query?limit=5`; asserts `len(records) <= 5` and exact savings bounded in **`[59.0%, 89.0%]`**.
  3. **Caveman Mode Verification:** Queries `/memory/query?limit=3`; asserts `len(records) <= 3` and exact savings bounded in **`[70.0%, 92.0%]`**.
  4. **Proactive Substrate Verification:** Queries `GET /memory/live_beliefs` directly; asserts token compression reaches **`>= 93.0%`** vs full conversation history while preserving core task context.
- **`journey_worldmodel_causal.py` (J3 & J4):**
  1. Inserts a causal chain of observations (`CausalDagBuilder`).
  2. Triggers `/worldmodel/rollout` with custom policy over horizon $T=5$; validates probability distributions and updates `live_beliefs`.
  3. Simulates repeated procedural tasks to trigger FSM compilation (`LoopEngine`); verifies future recall uses compiled FSM rules.
  4. Verifies full cryptographic Merkle audit trail (`assert_merkle_chain_integrity`).

### 6.2 `reporters.py` & Allure Integration
- **Structured Step Logging (`@allure.step`):** Every API interaction is wrapped in readable Allure steps (`"Step: Execute Caveman Mode Query (limit=3)"`).
- **Automatic Failure Attachments (`pytest_runtest_makereport` hook):** On test failure, `reporters.py` automatically attaches `server.log` (`100` lines tail), `stats.json` (`GET /stats`), and a directory snapshot of `storage_dir`.
- **Performance & Token Charts (`matplotlib/seaborn`):** Auto-generates `token_savings_distribution.png` and `latency_distribution.png` plotting Headroom/Caveman curves and sub-millisecond execution bounds.
- **Summary Report (`E2E_Execution_Report.md`):** Aggregates pass/fail status across all 8 phases into a clean Markdown table ready for inclusion in `TEST_VALIDATION_REPORT.md`.

---

## 7. Verification Plan

1. **Syntax & Import Validation:** Run `python -m pytest tests/e2e_user_harness/ --collect-only -q` to verify clean discovery of all suites, scenarios, and fixtures.
2. **Phase 0 & 1 Baseline Run:** Run `pytest tests/e2e_user_harness/suites/test_phase0_bootstrap.py tests/e2e_user_harness/suites/test_phase1_core_memory_ops.py -v` to confirm server auto-build, ephemeral port isolation, and core CRUD persistence.
3. **Token Optimization Deep-Dive Run:** Run `pytest tests/e2e_user_harness/suites/test_phase2_cognitive_and_token_optimization.py tests/e2e_user_harness/scenarios/journey_context_optimization.py -v` to strictly verify Headroom Top-5 (`[59%, 89%]`) and Caveman Top-3 (`[70%, 92%]`) savings.
4. **Full Suite Allure Generation:** Run `pytest tests/e2e_user_harness/ --alluredir=reports/allure -v` followed by `reporters.py` summary aggregation to produce `E2E_Execution_Report.md`.
