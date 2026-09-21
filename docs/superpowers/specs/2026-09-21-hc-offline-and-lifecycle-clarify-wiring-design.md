# Design: HC Offline Fix + Lifecycle Clarification Wiring

**Date:** 2026-09-21  
**Status:** Approved

---

## Problem 1 — HC Server Offline

### Root Cause

The global `~/.claude/mcp.json` points to the **3.12.0 VSIX bundled launcher** at:
```
C:/Users/user/.vscode/extensions/farmountain.hipcortex-memory-3.12.0/server/launcher.py
```

That launcher uses a bundled 3.12.0 binary and a stale bundled `mcp_server.py`. When VS Code is closed, its managed server process dies. The VSIX launcher tries to restart it but the bundled binary's path is inside the extension directory; the live `target/release/webserver.exe` is never used. MCP stdio server starts fine; every REST call fails silently → "HC offline".

The project-level `.mcp.json` was already fixed to use `scripts/hipcortex_mcp_launcher.py`. The global config was not.

### Fix (3 layers)

**Layer 1 — Update `~/.claude/mcp.json`**  
Point to `scripts/hipcortex_mcp_launcher.py` (same as project-level `.mcp.json`). This uses `target/release/webserver.exe` and live `sdk/mcp/server.py`.

**Layer 2 — SessionStart hook in `~/.claude/settings.json`**  
A bash hook checks port 3030 on every session start. If dead, starts the binary directly. Catches "VS Code closed → server killed → next Claude Code session starts cold".

**Layer 3 — Verify VS Code extension health-recovery**  
Confirm `hipcortexService` in `extension.ts` retries if the server dies mid-session. Add a reconnect interval if absent.

### Acceptance Criteria

- **AC-O1:** `add_memory` from a non-VS Code Claude Code session (VS Code closed) returns `{"status": "ok"}`
- **AC-O2:** Killing port 3030 manually, then opening a new Claude Code session → `check_progress` succeeds (hook started the server)
- **AC-O3:** `hipcortex doctor` reports `ok` — server version matches 3.14.0

---

## Problem 2 — Lifecycle Clarification Wiring

### Design

Agent-side `tiers_spent` tracking. Agent increments after each `SelfPrompt` rung. All lifecycle handlers accept `tiers_spent` and embed `clarify_route` in their response when uncertainty is detected.

### Data Flow

```
Iteration N:
  check_progress(success_factors, obs, iter=N, max_iter=20, tiers_spent=0, cost_of_wrong_execution=1.0)
  → { ...existing fields...,
      clarify_route: { action: "self_prompt", tier: "T0_environment",
                       rationale: "0/3 rungs spent — self-prompt before asking" } }

  Agent self-prompts T0, increments tiers_spent to 1

Iteration N+3:
  check_progress(..., tiers_spent=3, cost_of_wrong_execution=0.9)
  → { clarify_route: { action: "ask_user", tier: "T3_ask_user", rationale: "..." } }

Exit bound: should_exit emits clarify_exhausted=true when tiers_spent >= MAX_CLARIFY_TIERS
  and progress_ratio has not improved — agent continues with current reading rather than looping.
```

### Changes

#### `src/agent_guidance.rs`

- Add `ClarifyRouteInfo { action: String, tier: String, rationale: String }` — JSON-serializable form of `ClarifyRoute`
- `check_progress(success_factors, observations, iteration, max_iterations, tiers_spent, cost_of_wrong_execution)` → `CheckProgressResult` gains `clarify_route: Option<ClarifyRouteInfo>` populated when `uncertainty_detected`
- `plan_validation(success_factors, tiers_spent)` → response includes `clarify_route: Option<ClarifyRouteInfo>` when plan has uncertainty
- `ExitDecision` (returned by `should_exit`) gains `clarify_exhausted: bool` — true when `tiers_spent >= MAX_CLARIFY_TIERS` and progress stalled

#### `src/web_server.rs`

- `/agent/check-progress` — accept `tiers_spent: u32` (default 0), `cost_of_wrong_execution: f64` (default 1.0); include `clarify_route` in response
- `/agent/plan-validation` — accept `tiers_spent: u32` (default 0); include `clarify_route` when applicable
- `/agent/should-exit` — accept `tiers_spent: u32` (default 0); include `clarify_exhausted` in response

#### `sdk/mcp/server.py`

- `check_progress` schema: add optional `tiers_spent` (integer, default 0) and `cost_of_wrong_execution` (number, default 1.0)
- `plan_validation` schema: add optional `tiers_spent` (integer, default 0)
- `should_exit` schema: add optional `tiers_spent` (integer, default 0)
- All three handlers pass new params to REST

#### `sdk/python/hipcortex/install/SKILL.md`

Update PROGRESS and EXIT sections to show `tiers_spent` param explicitly.

### Acceptance Criteria

| AC | Description | Assert |
|---|---|---|
| AC-L1 | `check_progress` + uncertainty + `tiers_spent=0` | `clarify_route.action == "self_prompt"`, `tier == "T0_environment"` |
| AC-L2 | `check_progress` + uncertainty + `tiers_spent=3` + high cost | `clarify_route.action == "ask_user"` |
| AC-L3 | `check_progress` + uncertainty + `tiers_spent=3` + low cost | `clarify_route.action == "decline_ask"` |
| AC-L4 | `check_progress` + `uncertainty_detected=false` | `clarify_route` absent or `action == "no_clarification"` |
| AC-L5 | `plan_validation` + `tiers_spent=1` | `clarify_route.action == "self_prompt"`, `tier == "T1_prior_art"` |
| AC-L6 | `should_exit` + `tiers_spent >= MAX` + stalled | `clarify_exhausted == true`, action `continue` (not stuck) |
| AC-L7 | MCP `check_progress` passes `tiers_spent=2` | REST receives correct param |
| AC-L8 | Full ReAct sim 4 iterations | route escalates T0→T1→T2→T3 across 4 calls |

### Invariants (never break)

- `clarify_route` absent when `uncertainty_detected == false` — no noise in happy path
- `tiers_spent` default 0 everywhere — fully backward-compatible; old callers get T0 behavior
- `ClarifyRouteInfo.action` is one of: `"no_clarification"`, `"self_prompt"`, `"ask_user"`, `"decline_ask"` — matches `ClarifyRoute` enum variants
- `should_exit` never returns `succeed` solely because clarification was exhausted — ladder exhaustion is informational, not a success signal

---

## Implementation Order

1. `src/agent_guidance.rs` — add `ClarifyRouteInfo`, extend `check_progress`, `plan_validation`, `ExitDecision`
2. `src/web_server.rs` — wire new params/fields into REST handlers
3. `sdk/mcp/server.py` — update schemas and handlers (3 tools)
4. `sdk/python/hipcortex/install/SKILL.md` — update param docs
5. Tests — Rust unit tests for AC-L1..L8, Python MCP test for AC-L7
6. `~/.claude/mcp.json` — update global config (Layer 1)
7. `~/.claude/settings.json` — add SessionStart hook (Layer 2)
8. Verify `hipcortexService` reconnect loop in `extension.ts` (Layer 3)
9. Full ReAct simulation test (AC-L8)
10. Commit + push

---

## Non-goals

- Schema migration for GoalPayload (agent-side tracking chosen)
- npm publish (blocked by ENEEDAUTH)
- VSIX marketplace upload (manual)
