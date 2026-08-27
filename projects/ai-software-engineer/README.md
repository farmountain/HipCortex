# AI Software Engineer

An autonomous software-engineering agent built on two research foundations:

- **HipCortex** — persistent cognitive state, causal memory, experience, world/self/goal state.
- **Kakeya Abstraction Reasoning Action Model (KARAM)** — typed abstraction, reasoning/action control, contracts, validation, and distillation.

## Product thesis

Turn a GitHub repository into an executable software world model, then drive a goal → acceptance criteria → plan → action → test → failure analysis → repair → validation loop.

```text
Developer Goal
     |
     v
+-------------------+
| Cognitive Compiler|
| goal -> contract  |
+---------+---------+
          |
          v
+-------------------+        +-----------------------+
| HipCortex State   |<------>| repo / task memory    |
| beliefs / goals   |        | decisions / failures  |
| experience / twin |        +-----------------------+
+---------+---------+
          |
          v
+-------------------+
| KARAM Reasoner    |
| abstraction       |
| planning          |
| action composition|
| validation gates  |
+---------+---------+
          |
          v
+-------------------+
| Software Harness  |
| inspect -> edit    |
| build -> test      |
| observe -> repair  |
+---------+---------+
          |
          v
     Validated PR
```

## MVP

The first demonstrator focuses on one narrow but compelling workflow:

> **Give the agent a GitHub repository and a software goal. It autonomously derives acceptance criteria, inspects the codebase, changes files, runs tests, remembers decisions/failures, and iterates until the contract is satisfied or a hard stop is reached.**

### MVP components

- `cognitive_compiler.py` — converts a natural-language goal into a structured engineering contract.
- `hipcortex_adapter.py` — persistent state, memory, experience and optional DigitalTwin access.
- `kakeya_adapter.py` — abstraction/reasoning/action boundary; implementation is kept isolated from application orchestration.
- `software_harness.py` — safe repository observation, command execution and test evidence capture.
- `agent.py` — closed-loop controller.
- `cli.py` — hackathon-friendly command line entry point.

## Foundation references

- https://github.com/farmountain/HipCortex
- https://github.com/farmountain/Kakeya_Abstraction_Reasoning_Action_Model

The application layer deliberately treats both projects as foundations rather than copying their internals. This keeps the experiment replaceable and makes benchmark results attributable to the integrated system.

## Run

```bash
python -m ai_software_engineer --repo /path/to/repo --goal "Fix the failing authentication tests"
```

The first MVP can run in **dry-run mode** so that planning and validation are demonstrated before enabling write actions.

## Hackathon success metrics

1. Goal-to-validated-change completion rate.
2. Tests passed / tests attempted.
3. Repair iterations per task.
4. Repeated-context reduction from HipCortex memory retrieval.
5. LLM calls per successful task.
6. Native/KARAM actions versus teacher-generated actions.
7. Time-to-green and token cost versus a baseline coding agent.

## Roadmap

**Phase 1 — Repo Agent:** inspect, contract, plan, edit, test.

**Phase 2 — Cognitive State:** persistent decisions, beliefs, goals, failures and reusable experience.

**Phase 3 — Abstraction:** repository structures become typed abstractions; similar bugs/tasks share reusable operators.

**Phase 4 — World Model:** dependency graph + execution state + test state support counterfactual planning.

**Phase 5 — Distillation:** successful task trajectories become native operators and reduce runtime LLM dependence.

**Phase 6 — Autonomous Engineering:** agent proposes new abstractions/operators, validates them empirically, and opens a PR with evidence.
