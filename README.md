# HipCortex

[![PyPI](https://img.shields.io/pypi/v/hipcortex.svg)](https://pypi.org/project/hipcortex/)
[![npm](https://img.shields.io/npm/v/hipcortex.svg)](https://www.npmjs.com/package/hipcortex)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/farmountain/HipCortex)](https://github.com/farmountain/HipCortex/releases/latest)

**Autonomous agents have no persistent cognitive state — goals lost between calls, beliefs stale, actions never feeding back into reasoning. HipCortex is the cognitive state substrate that closes the loop: goal scheduling, belief revision, world model feedback, and decision provenance — served locally over MCP + REST.**

⭐ **If that solves a pain you feel, [star the repo](https://github.com/farmountain/HipCortex)** — it helps others find it.  
💬 **Tried it?** [Open an issue](https://github.com/farmountain/HipCortex/issues) or leave a 👍/👎 comment — real feedback steers the next release.

This repository is the **public developer surface** (docs, client SDKs, connectors, issues, release artifacts). New engine development lives in private [`hipcortex-core`](https://github.com/farmountain/hipcortex-core). Details: [DUAL_REPO.md](DUAL_REPO.md) · [NOTICE](NOTICE).

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

## What's new in v2.1.0 — Cognitive Substrate Coherence

Closes three structural gaps where the cognitive report showed correct outputs but the underlying mechanisms were incoherent.

| Gap | Problem | Fix |
|-----|---------|-----|
| **Miners didn't get smarter** | `induce_skill_record` always emitted empty `preconditions` and a `"pattern repeats N times"` placeholder — Q7 displayed Skills with no real schema | `induce_skill_record` now reads first/last motif member records from store, populates `preconditions` with the chain entry point and `expected_outcomes` with the chain result (action + target + frequency) |
| **Two belief writers** | `BeliefInvalidator` decayed confidence; `jtms::propagate_retraction` set `JtmsLabel::Out` — no coordination. A belief at `conf=0.05, label=In` was counted as a valid assumption in Q3 | `BeliefExecutive` is now the single mutation authority: `decay()` atomically applies confidence + cascades JTMS Out when below threshold; `retract()` clamps confidence to 0 before BFS propagation |
| **Clarify searched; didn't restate** | `ClarifyEngine` ran 3 belief-search rounds but had no WorldModel or env awareness — month-2 env changes (server offline, region changed) left stale `success_factors` in place | `restate_if_env_changed()` scans recent Temporal records for failure signals overlapping each unsatisfied factor; if blocked → renames factor to `{name}_when_available`, writes `Reflexion{goal_restated}`, and `run()` returns `ClarifiedBySubstrate` before belief search |

473 unit + 173 integration + 3 AC-SC + 5 AC-EP + 10 AC-v1.1.0 + 7 AC-v1.9.0 + 8 AC-original, 0 failures.

---

## What's new in v2.0.0 — Epistemic Write-Path

Closes the three axes of epistemic integrity: who is allowed to change truth, how abstractions form, and how the epistemic state survives process death.

| Axis | Implementation | Test |
|------|---------------|------|
| **Who can change truth** | `EpistemicAuthority::gate_belief_write` clamps Belief confidence by evidence tier: 0 evidence → max 0.50, 1-2 → 0.65, 3-6 → 0.80, 7+ → uncapped. Gated in `AddMemory` + `UpdateBelief` CognitiveDelta handlers. | AC-EP1, AC-EP2 |
| **How abstractions form** | `AbstractionGate::validate` requires ≥4 evidence records + Temporal/Reflexion grounding + unique proposition. `EmergenceDetector` sets `EpistemicStatus::Provisional`; gate passes → `elevate()` asserts `JtmsLabel::In + Confirmed`. | AC-EP3, AC-EP4, AC-EP5 |
| **Survives death** | JTMS `in_list/out_list/dependents` stored in `BeliefPayload` → JSONL; retraction cascade (BFS Out-propagation) state is pre-computed and persisted — no re-propagation needed on restart. | epistemic_write_path_sit (JTMS cascade) |

460 unit + 169 integration + 5 v2.0.0 acceptance + 10 v1.1.0 acceptance + 7 v1.9.0 acceptance, 0 failures.

---

## What's new in v1.9.0 — 3-Month Agent Coherence

Proves the long-running agent claim across three axes: restart survivability, targeted OOD isolation, and abstraction persistence.

| Claim | Implementation | Test |
|-------|---------------|------|
| **Restart survivable** | JSONL store + WM file + JTMS-in-store survive process kill; InProgress goals auto-resume on daemon Stage 1 first tick | AC-R1…R5 (7/7 pass) |
| **OOD → targeted isolation** | Daemon Stage 1b: Mahalanobis `severity > threshold` on most-uncertain entity → `CreditAssign("ood_shift:entity_id")`; unrelated beliefs stay `In` | AC-O1, AC-O2 |
| **Abstraction survival** | `mine_and_consolidate` → `SkillPayload` in JSONL → `emergent_abstractions` intact after reload | AC-R4, skill_abstractions_survive_restart |

163 integration + 445 unit + 7 v1.9.0 acceptance + 8 v1.1.0 acceptance, 0 failures.

---

## What's new in v1.8.0 — Cognitive Report Closure

Closes all remaining "not Yes" gaps in the 10-question cognitive state report and makes verifier mismatch a first-class revision event.

| Gap | Fix |
|-----|-----|
| **Q3 — assumptions valid** | `Unknown+0.5` beliefs tagged `Provisional(...)` in `valid_assumptions` — not silently included |
| **Q6 — what failed** | `CreditAssign` Reflexion records (broken structural equations) surface alongside failed goals |
| **Q7 — abstractions** | `Skill` records + high-confidence derived beliefs in `emergent_abstractions` |
| **Q9 — authorized actions** | Real `SelfModel` health (not hardcoded `1.0`) drives the authorized-actions filter |
| **Q10 — what next** | `SynthesisMode` (Escalate/Balanced/Autonomous) + `ClarifyEngine` pending status wired to `next_recommendation` |
| **Verifier → CreditAssign** | Prediction/observation mismatch fires `CreditAssign` — same revision path as critic veto; no more silent skipped ticks |

445 unit + 158 integration + 56 property + 8 acceptance, 0 failures.

---

## What's new in v1.7.0 — Epistemic Closure

Closes the four remaining epistemic gaps in the cognitive loop:

- **ClarifyEngine (P0-A)**: Self-prompting clarity loop (max 3 rounds) triggered on empty success_factors, ≥3 consecutive vetoes, or pre-success. Searches beliefs + WM for resolution; writes `Reflexion{self_clarified}` on success, single deduped `Belief{clarify_needed}` on escalation. Only unresolvable ambiguities reach the user.
- **Dynamic CriticGate threshold (P0-B)**: `CriticGate::evaluate_with_threshold(goal, action, iter, threshold)` replaces the static 0.25 constant. `evaluate()` is now a backward-compat wrapper.
- **SelfModel steers the loop (P0-D)**: `SelfModel::recommend_loop_config()` maps health→`LoopConfig{effective_veto_threshold, synthesis_mode}`. health < 0.3 → (0.50, Escalate); health > 0.8 → (0.15, Autonomous); else → (0.25, Balanced). Daemon Stage 0 reads this every tick.
- **Veto as revision event (P0-C)**: CriticGate rejection writes `Decision{critic_veto}` AND fires `CognitiveDelta::CreditAssign(FailureSignal::ExplicitFail)`. Veto is a learning signal, not a skipped tick.
- **JTMS as report truth (P0-E)**: `cognitive_report` Q3 (`valid_assumptions`) filters on `JtmsLabel::In` authoritatively; `Unknown` beliefs fall back to `confidence >= 0.5`. `JtmsLabel::Out` beliefs are excluded even at high confidence.

1027 tests (366 lib + 439 unit + 158 integration + 56 property + 8 acceptance), 0 failures.

---

## What's new in v1.6.3 — Dual-mode ReactEngine (StepByStep + FullCycle)

Closes the structural limit where CriticGate veto at iter ≥ 1 could never fire.

| Change | Details |
|--------|---------|
| **GoalExecutionMode::StepByStep** | New field on `GoalPayload` — daemon advances exactly one ReAct iteration per tick; goal persists `InProgress` across daemon ticks, enabling CriticGate veto at iter ≥ 1 |
| **GoalExecutionMode::FullCycle** | Default (backward-compatible) — `ReactEngine::run()` exhausts all iterations in one daemon tick, goal terminates per tick |
| **`ReactEngine::run_one_step()`** | Writes 1 Temporal + 1 Reflexion per call; increments `current_iteration`; returns `InProgress` until exhausted or all success_factors satisfied |
| **CriticGate veto now structurally achievable at iter ≥ 1** | With StepByStep, `CriticGate::evaluate(goal, "daemon_step", loop_iter=1)` fires against a live goal; proven by test writing 2 `Decision{critic_veto}` while `current_iteration` stays locked at 1 |
| | **652 tests, 0 failures** | 430 unit · 158 integration · 56 property · 8 acceptance |

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

**VS Code / Antigravity VSIX** (multi-OS server binaries bundled; extension **1.8.0**):  
Package from repo (`vscode-extension`) or latest GitHub Release VSIX. Mac/Linux auto-`chmod` bundled bins.

```bash
code --install-extension hipcortex-memory-2.0.0.vsix
```

Honest support matrix (what's native vs docs-only): **[docs/channels.md](docs/channels.md)** · CLI: `hipcortex channels`

Release notes for v1.1.0–v1.6.3 remain in git history on this file; the latest user-facing notes are v1.8.0 above.

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
| **Runtime** | Prebuilt `webserver` / image from [Releases](https://github.com/farmountain/HipCortex/releases) |

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
4. **PRs** welcome on this public surface — [CONTRIBUTING.md](CONTRIBUTING.md)

Engine internals are not reviewed here. See [DUAL_REPO.md](DUAL_REPO.md).

---

## Docs & license

| Doc | For |
|-----|-----|
| [DUAL_REPO.md](DUAL_REPO.md) | Public surface vs private engine |
| [docs/usage.md](docs/usage.md) | CLI, harness, day-to-day use |
| [docs/architecture.md](docs/architecture.md) | How to *use* the substrate (black-box) |
| [docs/channels.md](docs/channels.md) | Channel honesty matrix |
| [DEPLOY.md](DEPLOY.md) | Self-host / Fly / Docker |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Historical in-tree build notes |

**License:** [Apache-2.0](LICENSE) for this public repository · **Version:** `2.0.0` · VSIX `2.0.0`
