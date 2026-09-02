# HipCortex

[![PyPI](https://img.shields.io/pypi/v/hipcortex.svg)](https://pypi.org/project/hipcortex/)
[![npm](https://img.shields.io/npm/v/hipcortex.svg)](https://www.npmjs.com/package/hipcortex)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/farmountain/HipCortex)](https://github.com/farmountain/HipCortex/releases/latest)

**Autonomous agents have no persistent cognitive state — goals lost between calls, beliefs stale, actions never feeding back into reasoning. HipCortex is the cognitive state substrate that closes the loop: goal scheduling, belief revision, world model feedback, and decision provenance — served locally over MCP + REST.**

⭐ **If that solves a pain you feel, [star the repo](https://github.com/farmountain/HipCortex)** — it helps others find it.  
💬 **Tried it?** [Open an issue](https://github.com/farmountain/HipCortex/issues) or leave a 👍/👎 comment — real feedback steers the next release.

---

## Why it exists

Every agent invocation starts cognitively blind. Goals set in one call vanish before the next. Beliefs accumulated from observations are never revised when contradicted. Actions taken by the agent never update its world model. Decisions leave no audit trail. There is no loop — just isolated acts.

HipCortex is the substrate that closes it: a **local causal graph** of goals, beliefs, decisions, and observations, with a reasoning loop that feeds every action back into prediction, served over **HTTP + MCP** to any agent host.

| Without HipCortex | With HipCortex |
|-------------------|---------------|
| Goals re-stated every call | GoalScheduler tracks + prioritizes across sessions |
| Stale beliefs silently persist | BeliefInvalidator detects contradictions, decays confidence |
| Actions never update world model | WorldModelUpdater closes the feedback loop |
| Decisions leave no trace | DecisionPayload + provenance chain per act-phase |
| Agent doesn't know what it's allowed to do | ActionRegistry + ExecutionGate answer that in one call |

---

## What's new in v1.5.0 — Gap Closure (Gaps 1, 5, 6, 7, 8)

Five production gaps from the autonomous cognitive OS assessment — all closed and test-covered.

| Gap | Capability | Details |
|-----|-----------|---------|
| **7 — GoalClarify gate** | ReactEngine pre-flight check | `run()` returns `Err` when `success_factors` is empty — forces `/goal/:id/clarify` before loop starts; no silent no-op runs |
| **5 — Named drift isolation** | Per-node OLS drift tracking | `PredictionMonitor::observe_named(node, error, x, y)` tracks (x,y) pairs per named node; `most_drifted_node()` returns the node with highest OLS weight; exposed via `SelfModel::most_drifted_node()` |
| **6 — WM-state auth gate** | Live WM data guards ops | `list_authorized_world_model(sm, wm)` now gates per WM state: rollout requires `transition_count() > 0`, counterfactual requires `causal_node_count() > 0`, intervene requires `causal_edge_count() > 0`; REST `GET /v1/actions/authorized-wm` passes live WM |
| **1 — Daemon real goals** | Daemon dequeues InProgress goals | Stage 1 finds highest-priority InProgress goal for actor via `search_by_goal_status`; Stage 3 CriticVeto uses real goal's `success_factors`; ExitCheck marks goal `Succeeded` when all factors satisfied |
| **8 — Continuous dynamics bridge** | WM entity snapshots per tick | Daemon Stage 6 iterates `entity_mean_vectors()` and writes `Temporal{action="wm_state_snapshot", target=entity_name}` per entity per iteration |
| | **620 tests, 0 failures** | 406 unit · 158 integration · 56 property |

---

## What's new in v1.4.0 — Depth & Ownership (Phases 1–5)

Six architectural mechanisms that give the agent true executive control over its own reasoning loop.

| Phase | Capability | Details |
|-------|-----------|---------|
| **1 — Clarify gate** | GoalNotClarified guard | `CognitiveDelta::AddMemory(Goal{InProgress, empty factors})` → `GoalNotClarified` error; `POST /goal/:id/clarify` returns clarifying questions; self-prompting resolves ambiguity before loop starts |
| **2 — CriticGate + VerifierGate** | Pre-action veto + post-observe check | `CriticGate`: iter 0 always passes; iter N with success fraction < 0.25 → `Decision{action="rejected"}` + iteration skipped. `VerifierGate`: WM prediction vs actual target — mismatch → `Belief{action="verifier_mismatch"}` + skip |
| **3a — Workspace lease** | TTL-bounded workspaces | `lease_until: Option<SystemTime>` on `Workspace`; `POST /v1/workspace/:id/renew` extends lease; backward compatible (`None` = no expiry) |
| **3b — OLS drift isolation** | Coordinate-wise drift weights | `PredictionMonitor::feed_with_obs(error, x, y)` accumulates feature/target pairs; `fit_ols()` returns per-dimension weights identifying which inputs drive drift |
| **3c — WM authorization table** | Static op constraints | `WM_CONSTRAINTS` table (rollout ≤ depth 10/iter 200, counterfactual ≤ 5/50, intervene ≤ 3/10); `GET /v1/actions/authorized-wm` filters by `SelfModel` health |
| **3d — Motif contraction** | Cycle guard + archive-before-delete | `has_derived_from_cycle()` skips motifs with corrupted provenance; causal validity check via WM; source records appended to `ArchiveStore` before hot-store deletion |
| **4 — 8-stage daemon** | Owned cognitive loop | `SubstrateDaemon` runs Observe→Reflect→Plan→CriticVeto→Predict→Act→Update→ExitCheck per tick; `CognitiveLoopConfig` controls timing + consolidation threshold; `POST /v1/loop/stop/:handle` |
| **5 — harness.md rewrite** | Accurate docs | `docs/harness.md` fully rewritten to match v1.4.0 — daemon-owned loop, all new endpoints, veto semantics, clarify gate, self-prompting lifecycle |
| | **608 tests, 0 failures** | 395 unit · 157 integration · 56 property |

---

## What's new in v1.3.0 — Cognitive Loop Closure (Phases A–H)

Nine structural gaps in the cognitive architecture closed. Every gap has a dedicated test.

| Gap | Capability | Details |
|-----|-----------|---------|
| **G-WHY** | Decision rationale chain | `DecisionPayload.rationale_chain: Vec<String>` — human-readable decision trace alongside UUID evidence links; persisted per ReactEngine act-phase |
| **G-AUTH** | Contextual action authorization | `list_authorized_contextual(goal_id, actor, has_workspace, health_score)` — Q9 of `CognitiveStateReport` now filters live actions by active goal + health, not a static list |
| **G-REV** | JTMS-routed belief retraction | `BeliefInvalidator::process` is now read-only; returns `Vec<Uuid>` to retract — callers route through `CognitiveHandle::retract_belief` → JTMS cascade |
| **G-ABS** | Causal motif mining | `mine_and_consolidate` induces `Skill` + `Belief` records from recurring `derived_from` chains; REST `POST /v1/memory/consolidate?strategy=motif` |
| **G-WS** | Durable workspaces | `Workspace.save(dir)` / `load(path)` / `load_all(dir)` JSONL persistence; OR-Set CRDT merge survives restart |
| **G-SHIFT** | Kalman prediction monitor | `PredictionMonitor` rolling-window drift detector — 5 consecutive errors > 0.3 emits `CognitiveDelta::RewriteStructuralEquation` |
| **G-LOOP** | SubstrateDaemon background worker | Per-actor maintenance threads (GC + AutoConsolidate every 30 s); REST `POST /v1/loop/subscribe` + `GET /v1/loop/status/:handle` |
| **G-CRIT/VER** | Critic + Verifier in ReactEngine | Critic writes `Belief{action="critic_score"}` per iteration (fraction of success_factors satisfied); Verifier writes `Belief{action="verifier_report"}` on exit; `GET /goal/:id/verify` |
| **G-EXPORT** | Versioned state export schema | `EXPORT_SCHEMA_VERSION = env!("CARGO_PKG_VERSION")` — compile-time single source of truth; `StateExportSchema::current()` stamped on every `GET /v1/state/export` |
| | **583 tests, 0 failures** | 371 unit · 148 integration · 8 acceptance · 56 property |

---

## What's new in v1.2.2 — Calibration Fidelity

| Fix | Details |
|-----|---------|
| **G2a calibration signal unattenuated** | `calibrate_after_tx` no longer calls `record_prediction_error(0.0)` after every transaction — the Dirichlet entropy set by G2a in `apply_delta` is now the sole prediction-error signal, not silently damped by 0.9× |
| **README version stamps corrected** | All `1.2.0` / `1.1.0` stale references in README and VSIX description updated |

---

## What's new in v1.2.1 — Cognitive Substrate Closure

| Capability | Details |
|-----------|---------|
| **WMUpdater auto-wired** | Every `AddMemory(Temporal)` now fires `update_from_temporal` → world model stays live without ReactEngine |
| **BeliefInvalidator auto-wired** | Temporal/Reflexion writes automatically invalidate contradicting Symbolic beliefs via `apply_delta` |
| **EmergenceDetector auto-wired** | Every 10th Temporal write triggers emergence scan; synthesised Beliefs written to Hot Store |
| **Live calibration** | G2a: Dirichlet entropy from real WM transitions replaces hardcoded 0.0 signal in `CalibrationTracker` |
| **Causal topo wired at startup** | `CoherenceChecker.set_consistency_topo()` called in webserver + MCP server; causal violations now detected |
| **`POST /v1/loop/omega`** | REST endpoint exposes `LoopEngine.run_omega_loop()` — coverage gap detection, rollout sim, belief mutation |
| **`run_omega_loop` MCP tool** | 20th MCP tool; agents can trigger one omega iteration from Claude Code / Cursor |
| **550+ tests, 0 failures** | 358 unit + 53 property + 140 integration. All prior tests green. |

---

## What's new in v1.2.0 — Causal SCM Continuous Substrate

| Capability | Details |
|-----------|---------|
| **Structural Equations** | Every causal node carries `f_i(PA_i, U_i)` via `StructuralEquation` trait. `LinearSE` is evaluable + invertible. |
| **do-calculus** | `apply_intervention(var, val)` mutates shared graph in-place (persistent). `do_operator` clones for rollout simulation. |
| **Counterfactual Credit Assignment** | Full AAP triad: Abduction → Action → Prediction. `CreditAssign` returns the single broken structural equation. OOD invariance: stable equations never blamed. |
| **Continuous substrate as primary** | `DigitalTwin.step()` clamps RK4 output to pinned intervention vars — causal impulses override ODE dynamics. |
| **ExperienceStore provenance** | `rollout_hybrid` with `causal_nodes` writes a `causal_provenance` record to the fork store. |
| **Transactional gate** | All 4 SCM operators (`Intervene`, `Counterfactual`, `CreditAssign`, `RewriteStructuralEquation`) through `CognitiveDelta` with Reflexion audit records. |
| **542 tests** | 353 unit + 53 property + 138 integration. 0 regressions. |

---

## What's new in v1.1.0 — Cognitive Loop Closure

| Capability | Details |
|-----------|---------|
| **GoalScheduler** | Ranks Pending/InProgress Goals by `urgency / estimated_cost` — always returns highest-priority next goal |
| **EmergenceDetector** | Scans last 50 Temporal records every 10 writes; tokens in ≥5 records auto-synthesize into a new Belief with evidence pointers |
| **BeliefInvalidator** | Contradiction detection; decays confidence by `score × 0.3`; writes `belief_invalidated` marker at conf < 0.2 |
| **DecisionPayload** | New `MemoryType::Decision` per ReactEngine act-phase — captures `option_chosen`, `alternatives`, `rationale`, `confidence`, `outcome` |
| **CognitiveStateReport** | Single call answers all 10 cognitive questions: goals, beliefs, assumptions, decisions, failures, authorized actions, next recommendation |
| **WorldModelUpdater** | Closes feedback loop: ReactEngine feeds each observation into the Dirichlet-Multinomial world model |
| **45 MCP tools / 7 resources** | New: `cognitive_report`, `list_authorized_actions`, `get_provenance` |
| **4 new REST endpoints** | `GET /v1/cognitive/report`, `GET /v1/goals`, `GET /v1/actions/authorized`, `GET /v1/memory/:id/provenance` |
| **parse_record_type_alias fix** | Goal/Skill/Belief/Decision now correctly stored via REST — no more silent Temporal fallback |

---

## Install in 60 seconds

Works on **Windows, macOS, and Linux**.

```bash
pip install -U hipcortex
hipcortex install          # pick your IDE (Claude, Cursor, VS Code, Grok, …)
hipcortex start            # local server on http://127.0.0.1:3030
hipcortex doctor           # health check
```

Non-interactive:

```bash
hipcortex install --yes
hipcortex install --url https://hipcortex.fly.dev   # optional managed endpoint
```

**TypeScript client:**

```bash
npm install hipcortex
```

**VS Code / Antigravity VSIX** (multi-OS server binaries bundled; extension **1.3.0**):  
Package from repo (`vscode-extension`) or latest GitHub Release VSIX. Mac/Linux auto-`chmod` bundled bins.

```bash
code --install-extension hipcortex-memory-1.3.0.vsix
```

Honest support matrix (what's native vs docs-only): **[docs/channels.md](docs/channels.md)** · CLI: `hipcortex channels`

---

## 60-second usage

**Python**

```python
from hipcortex import HipCortexClient

client = HipCortexClient("http://127.0.0.1:3030")
client.add_memory(actor="alice", action="decided", target="Use Postgres for sessions")
print(client.search("sessions", limit=5))
# client.forget("alice")  # GDPR-style wipe for an actor
```

**TypeScript**

```typescript
import { HipCortexClient } from "hipcortex";

const client = new HipCortexClient({ baseUrl: "http://127.0.0.1:3030" });
await client.addMemory({ actor: "alice", action: "decided", target: "Use Postgres" });
const { results } = await client.search({ query: "Postgres", limit: 5 });
```

**Live try (no local install)**

```bash
curl https://hipcortex.fly.dev/health
```

---

## Where it plugs in

| Surface | How |
|---------|-----|
| **Claude Code** | `hipcortex install` → skill + optional `--mode proactive` |
| **Cursor / VS Code / Windsurf / Grok / …** | MCP config via wizard |
| **Python agents** | `pip install hipcortex` + LangChain / CrewAI / AutoGen adapters |
| **Node agents** | `npm install hipcortex` |
| **Rust core** | Local `webserver` binary from [Releases](https://github.com/farmountain/HipCortex/releases) |

Deep host notes: [docs/hosts/README.md](docs/hosts/README.md)

---

## What "good" looks like

- **Remember** → agent stops re-asking the same project decisions  
- **Recall** → search / live beliefs return the right fact in one call  
- **Lean context** → fewer tokens than pasting full history  
- **Yours** → data stays local unless you point at a remote URL  

Benchmark notes (local latency & token savings): [BENCHMARK.md](BENCHMARK.md)

---

## Contribute & feedback

We ship faster when users tell us what broke or what you love.

1. **Star** the repo if you want this to exist  
2. **Install** and run `hipcortex doctor`  
3. **Report** bugs / "I expected X" in [Issues](https://github.com/farmountain/HipCortex/issues)  
4. **PRs** welcome — [docs/contributing.md](docs/contributing.md)

---

## Docs & license

| Doc | For |
|-----|-----|
| [docs/usage.md](docs/usage.md) | CLI, harness, day-to-day use |
| [docs/architecture.md](docs/architecture.md) | How the engine is built |
| [docs/channels.md](docs/channels.md) | Channel honesty matrix |
| [DEPLOY.md](DEPLOY.md) | Self-host / Fly / Docker |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Build from source |

**License:** [Apache-2.0](LICENSE) · **Version:** `1.5.0` · VSIX `1.5.0`
