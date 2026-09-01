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

## What's new in v1.3.0 — Autonomous Agent Harness

| Capability | Details |
|-----------|---------|
| **Tool Discovery (self-prompting)** | `recommend_tools(task=<description>)` — agent self-prompts to discover required MCP servers, skills, tech stack, and setup commands before starting any complex task |
| **Proactive harness mode** | `hipcortex install --mode proactive` — SKILL.md strongly recommends `get_live_beliefs` before every project-state question; substrate carries state, not LLM context |
| **Unified `live_beliefs` surface** | `GET /memory/live_beliefs` merges symbolic facts, code KG, Aureus hypotheses, world model predictions, and self/coherence intel in one call |
| **AgentMessage auto-ingest** | `HIPCORTEX_AGENT_DEFAULTS=1` — PerceptionSession wired by default for agent paths; incoming messages auto-stored as low-priority Temporal records |
| **Multi-agent actor scoping** | `hipcortex install --actor <name>` — per-actor SKILL install; shared substrate with no cross-actor contamination |
| **`POST /memory/reflect`** | Substrate chain-of-thought via AureusBridge — world-model prior + coherence check before LLM final output |
| **G2a calibration fidelity** | `calibrate_after_tx` no longer zeroes prediction error — unattenuated Dirichlet entropy feeds CalibrationTracker |
| **`docs/harness.md`** | Full agent harness reference: action space, observations, ReAct loop pattern, multi-agent notes |
| **551 tests, 0 failures** | All prior tests green. |

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

**License:** [Apache-2.0](LICENSE) · **Version:** `1.3.0` · VSIX `1.3.0`
