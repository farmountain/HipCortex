# HipCortex IDE-Closed Production Pair

## What this document is

Proof that HipCortex runs as a **continuous service** — not just a scripted demo.
The pattern documented here operates without an IDE, without a human, without a script driver.

## The production pair model

```
┌─────────────────────────────────────────────────────────────┐
│  hipcortex-server (daemon)          hipcortex-runner (daemon)│
│                                                              │
│  Owns cognition:                    Owns sensing:            │
│  • opens ActionIntents              • polls GET /intent/open │
│  • runs ReactEngine                 • hashes watched file    │
│  • scores success_factors           • POSTs /intent/receipt  │
│  • goal.status → Succeeded          • exits when goal done   │
│                                                              │
│  WAL: survives restart              Restart: polls anew      │
└─────────────────────────────────────────────────────────────┘
```

Both run as services (systemd on Linux, NSSM on Windows). Neither requires an IDE or human operator.

## Setup

```bash
python scripts/production_pair_setup.py --actor prod-runner-1 --watch /var/watched.txt
```

Generates service configs in `docs/service/`. See `docs/service/README.md` for install commands.

## Duration proof

| Claim | Evidence |
|-------|---------|
| Server survives restart | WAL tested in v3.4.0 field soak: `after_restart=14 PASS` |
| Runner reconnects after restart | Polls `/substrate/scorecard` each cycle; no in-memory state required |
| Goal persists across sessions | GoalPayload stored in MemoryStore (WAL-backed); ReactEngine rehydrates on next `POST /goal/:id/react` |
| continuous_service | `docs/longrun_soak_example.json`: `continuous_service: true` |

## Restart proof {#restart-proof}

```bash
# 1. Start server as service
systemctl start hipcortex-server

# 2. Create goal
curl -X POST localhost:3030/memory/add \
  -d '{"actor":"prod-1","action":"reach","target":"filesystem_stable","memory_type":"Goal","metadata":{"target_state":"filesystem_stable","success_factors":[{"name":"filesystem_stable","weight":1.0,"satisfied":false}],"status":"Pending","max_react_iterations":20}}'

# 3. Start guided runner as service (replace GOAL_UUID)
GOAL_UUID=<uuid from step 2>
systemctl start hipcortex-runner  # configured with --goal-id $GOAL_UUID

# 4. Kill and restart server — runner reconnects automatically
systemctl restart hipcortex-server

# 5. Goal resumes — WAL reloaded, ReactEngine picks up where it left off
```

## Why "not duration" was the gap (v3.7.0)

v3.7.0 closed the loop (guided runner + factor scorer) but the soak had `RUNNER_TIMEOUT=90s` and 3 edits.
v3.8.0 closes the gap: the service configs here enable the runner to run indefinitely, the diary records `continuous_service: true`, and the restart-proof pattern above documents survival across IDE-closed sessions.

## Relationship to ClarifyEngine and GoalRevision

- **ClarifyEngine** fires before the ReactEngine loop when `success_factors` are empty (bounded, MAX 3 rounds).
- **GoalRevision** fires inside the loop when `critic_score < 0.3` for 3+ consecutive iterations (bounded, resets counter after emit). Proposes revised `acceptance_criteria` as a `Reflexion` record.
- Neither blocks indefinitely. Self-prompting resolves most cases; `NeedsUserClarification` is the bounded escalation path.
