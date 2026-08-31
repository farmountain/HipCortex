# HipCortex

[![PyPI](https://img.shields.io/pypi/v/hipcortex.svg)](https://pypi.org/project/hipcortex/)
[![npm](https://img.shields.io/npm/v/hipcortex.svg)](https://www.npmjs.com/package/hipcortex)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/farmountain/HipCortex)](https://github.com/farmountain/HipCortex/releases/latest)

**AI coding agents forget decisions, re-read the same context, and burn tokens — HipCortex is local persistent memory you install once so Claude, Cursor, VS Code, and your Python/TS agents remember what matters and spend less on every turn.**

⭐ **If that solves a pain you feel, [star the repo](https://github.com/farmountain/HipCortex)** — it helps others find it.  
💬 **Tried it?** [Open an issue](https://github.com/farmountain/HipCortex/issues) or leave a 👍/👎 comment — real feedback steers the next release.

---

## Why it exists

Long agent sessions either dump full chat history into the prompt (expensive, noisy) or forget yesterday's decisions (frustrating). HipCortex stores memories as a **local causal graph**, serves them over **HTTP + MCP**, and injects **only the small, relevant slice** your agent needs next.

| You get | Without HipCortex |
|--------|-------------------|
| Decisions persist across sessions | "We already decided that" — lost |
| Smaller prompts (Headroom / Caveman modes) | Context stuffing & higher API cost |
| One install for many hosts | Hand-edited MCP configs per tool |
| Runs on your machine (Win / macOS / Linux) | Cloud-only memory lock-in |

---

## What's new in v1.1.0 — Cognitive Loop Closure

v1.1.0 closes all 9 gaps in the cognitive architecture loop:

**Persistent State → Abstraction → World Model → Reasoning → Criticism → Verification → Action → Feedback → State Update**

| Capability | What it does |
|-----------|-------------|
| **GoalScheduler** | Ranks concurrent Pending/InProgress Goals by `urgency / estimated_cost` — always returns the highest-priority goal to pursue next |
| **EmergenceDetector** | Scans last 50 Temporal records every 10 writes; tokens appearing in ≥5 records auto-synthesize into a new Belief with evidence pointers |
| **BeliefInvalidator** | Token-overlap + negation-keyword contradiction detection; decays belief confidence by `score × 0.3`; writes `belief_invalidated` marker when confidence < 0.2 |
| **DecisionPayload** | New `MemoryType::Decision` record per ReactEngine act-phase — captures `option_chosen`, `alternatives`, `rationale`, `confidence`, `outcome` (back-filled) |
| **CognitiveStateReport** | Single `build_report(store, actor)` call answers all 10 cognitive questions: goals, beliefs, assumptions, decisions, failures, abstractions, uncertainties, authorized actions, next recommendation |
| **WorldModelUpdater** | Closes the feedback loop: ReactEngine calls `update_from_temporal(obs, wm)` after each observation, feeding transitions into the Dirichlet-Multinomial world model |
| **ActionRegistry** | `ALL_OPS` list + `list_authorized(self_model)` via ExecutionGate — agent always knows what it's allowed to do |
| **`search_by_goal_status`** | Filter Goal records by status (`pending`, `inprogress`, `failed`, `succeeded`) — failure index built in |
| **Provenance chain** | BFS traversal of `derived_from` + `evidence` links, depth 20 — full audit trail for any record |

**New REST endpoints:**

| Endpoint | Returns |
|---------|---------|
| `GET /v1/cognitive/report?actor=X` | Full cognitive state report |
| `GET /v1/goals?actor=X&status=pending` | Filtered goal list |
| `GET /v1/actions/authorized` | Authorized ops via ExecutionGate |
| `GET /v1/memory/:id/provenance` | Provenance chain for any record |

**New MCP tools:** `cognitive_report`, `list_authorized_actions`, `get_provenance`

**Bug fix:** `POST /memory/add` with `record_type: "Goal"/"Skill"/"Belief"/"Decision"` previously silently stored as `Temporal`. Now correctly routed — GoalScheduler and search_by_goal_status work for REST-seeded records.

**Test coverage:** 339 lib · 348 unit · 138 integration · 11 REST SIT · 10/10 v1.1.0 ACs · 8/8 v1.0.0 ACs

---

## What's new in v1.0.0 — Causal SCM Substrate

| Capability | Details |
|-----------|---------|
| **Causal SCM** | `StructuralEquation` trait, do-calculus interventions, counterfactual reasoning |
| **Credit assignment** | Gated causal credit across the topo graph |
| **MGVOperator** | Feeling-of-Knowing (FOK) + Judgment-of-Learning (JOL) metacognitive signals |
| **DigitalTwin** | `fork_under_intervention` — simulate causal counterfactuals on a twin |
| **OOD invariance** | Topological rewiring preserves attribution under distribution shift |

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

**VS Code / Antigravity VSIX** (multi-OS server binaries bundled; extension **1.1.0**):  
Download `hipcortex-memory-1.1.0.vsix` from [Releases](https://github.com/farmountain/HipCortex/releases/latest).

```bash
code --install-extension hipcortex-memory-1.1.0.vsix
```

Honest support matrix: **[docs/channels.md](docs/channels.md)** · CLI: `hipcortex channels`

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

**Cognitive state (v1.1.0)**

```python
import requests

# What is the agent's current cognitive state?
report = requests.get("http://127.0.0.1:3030/v1/cognitive/report?actor=alice").json()
print(report["active_goals"])          # What are we pursuing?
print(report["learned_beliefs"])       # What do we know?
print(report["next_recommendation"])   # What should happen next?

# What actions are authorized right now?
ops = requests.get("http://127.0.0.1:3030/v1/actions/authorized").json()

# Full audit trail for a memory record
chain = requests.get(f"http://127.0.0.1:3030/v1/memory/{record_id}/provenance").json()
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
curl "https://hipcortex.fly.dev/v1/cognitive/report?actor=demo"
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
- **Close the loop** → GoalScheduler picks next goal; EmergenceDetector surfaces patterns; BeliefInvalidator prunes stale assumptions  
- **Yours** → data stays local unless you point at a remote URL  

Benchmark notes: [BENCHMARK.md](BENCHMARK.md)

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

**License:** [Apache-2.0](LICENSE) · **Version:** `1.1.0` · VSIX `1.1.0`
