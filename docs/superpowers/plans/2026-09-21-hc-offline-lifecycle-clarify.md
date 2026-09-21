# HC Offline Fix + Lifecycle Clarification Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire `clarify_route` into `check_progress`/`plan_validation`/`should_exit` responses so every lifecycle gate embeds the next clarification rung; fix global MCP config so HC is always online; add SessionStart hook to auto-start the REST backend.

**Architecture:** Agent-side `tiers_spent` tracking — agent increments after each `SelfPrompt` rung and passes it at every gate. Rust functions call `route_uncertainty()` internally and embed `ClarifyRouteInfo` (JSON-serializable form of `ClarifyRoute`) in responses when uncertainty is detected. REST handlers extract the new params with defaults of 0 so all existing callers keep working.

**Tech Stack:** Rust (`src/agent_guidance.rs`, `src/web_server.rs`), Python (`sdk/mcp/server.py`, `sdk/python/tests/`), SKILL.md, global `~/.claude/mcp.json`, `~/.claude/settings.json`

**Constants (from `src/clarify_engine.rs`):**
- `MAX_CLARIFY_TIERS = 3`
- `COST_OF_ASKING = 0.25`
- `P_UNRESOLVABLE_AFTER_LADDER = 1.0` (search_incomplete=false)
- `P_UNRESOLVABLE_INCOMPLETE = 0.5` (search_incomplete=true)
- Gate: `p_unresolvable * cost_of_wrong_execution > COST_OF_ASKING`
- Tier strings: `"T0_environment"`, `"T1_prior_art"`, `"T2_causal"`, `"T3_ask_user"`

---

## File Map

| File | Action | Responsible for |
|------|--------|----------------|
| `src/agent_guidance.rs` | Modify | `ClarifyRouteInfo` struct, From impl, new fields on structs, extended function signatures + implementations, updated existing tests, new AC-L1..L6 tests |
| `src/web_server.rs` | Modify | 3 REST handlers extract `tiers_spent`, `cost_of_wrong_execution` with defaults and pass to Rust functions |
| `sdk/mcp/server.py` | Modify | 3 tool schemas add optional params, 3 handlers pass new params to REST |
| `sdk/python/tests/test_lifecycle_clarify_mcp.py` | Create | AC-L7 Python test — verifies MCP handler passes `tiers_spent` through to REST body |
| `sdk/python/hipcortex/install/SKILL.md` | Modify | Update PROGRESS and EXIT lines to show `tiers_spent` param |
| `scripts/hipcortex_start_if_dead.py` | Create | Port 3030 check + spawn `target/release/webserver.exe` if dead |
| `~/.claude/mcp.json` | Modify | Point global config to `scripts/hipcortex_mcp_launcher.py` instead of stale 3.12.0 VSIX launcher |
| `~/.claude/settings.json` | Modify | Add SessionStart hook to run `hipcortex_start_if_dead.py` |
| `vscode-extension/src/extension.ts` | Modify if needed | Verify/add periodic health reconnect interval |

---

## Task 1: Add `ClarifyRouteInfo` struct + new fields on existing structs

**Files:** `src/agent_guidance.rs`

- [ ] **Step 1: Insert `ClarifyRouteInfo` struct and `From<ClarifyRoute>` impl after the `ClarifyRoute` enum (around line 396)**

  Find the block ending with `ClarifyRoute::DeclineAsk { tier: String, rationale: String },` and insert after the closing `}` of the enum:

  ```rust
  /// JSON-serializable projection of [`ClarifyRoute`]. Embedded in lifecycle responses.
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct ClarifyRouteInfo {
      pub action: String,  // "no_clarification" | "self_prompt" | "ask_user" | "decline_ask"
      pub tier: String,
      pub rationale: String,
  }

  impl From<ClarifyRoute> for ClarifyRouteInfo {
      fn from(r: ClarifyRoute) -> Self {
          match r {
              ClarifyRoute::NoClarification { rationale } => ClarifyRouteInfo {
                  action: "no_clarification".into(),
                  tier: "none".into(),
                  rationale,
              },
              ClarifyRoute::SelfPrompt { tier, rationale } => ClarifyRouteInfo {
                  action: "self_prompt".into(),
                  tier,
                  rationale,
              },
              ClarifyRoute::AskUser { tier, rationale } => ClarifyRouteInfo {
                  action: "ask_user".into(),
                  tier,
                  rationale,
              },
              ClarifyRoute::DeclineAsk { tier, rationale } => ClarifyRouteInfo {
                  action: "decline_ask".into(),
                  tier,
                  rationale,
              },
          }
      }
  }
  ```

- [ ] **Step 2: Add `clarify_route` field to `ValidationPlan` (currently at line 157)**

  Change:
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  pub struct ValidationPlan {
      pub steps: Vec<ValidationStep>,
      pub unknown_factors: Vec<String>,
      pub validation_order: String,
  }
  ```
  To:
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  pub struct ValidationPlan {
      pub steps: Vec<ValidationStep>,
      pub unknown_factors: Vec<String>,
      pub validation_order: String,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub clarify_route: Option<ClarifyRouteInfo>,
  }
  ```

- [ ] **Step 3: Add `clarify_route` field to `ProgressCheck` (currently at line 234)**

  Change:
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  pub struct ProgressCheck {
      pub iteration: u32,
      pub max_iterations: u32,
      pub satisfied_factors: Vec<String>,
      pub pending_factors: Vec<String>,
      pub progress_ratio: f32,
      pub on_track: bool,
      pub recommended_action: String,
      pub uncertainty_detected: bool,
  }
  ```
  To:
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  pub struct ProgressCheck {
      pub iteration: u32,
      pub max_iterations: u32,
      pub satisfied_factors: Vec<String>,
      pub pending_factors: Vec<String>,
      pub progress_ratio: f32,
      pub on_track: bool,
      pub recommended_action: String,
      pub uncertainty_detected: bool,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub clarify_route: Option<ClarifyRouteInfo>,
  }
  ```

- [ ] **Step 4: Add `clarify_exhausted` field to `ExitDecision` (currently at line 315)**

  Change:
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  pub struct ExitDecision {
      pub action: ExitAction,
      pub rationale: String,
      pub next_step: String,
  }
  ```
  To:
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  pub struct ExitDecision {
      pub action: ExitAction,
      pub rationale: String,
      pub next_step: String,
      pub clarify_exhausted: bool,
  }
  ```

- [ ] **Step 5: Try to build (will fail with missing field errors — that's expected)**

  ```sh
  cargo build --no-default-features --features petgraph_backend 2>&1 | grep "error\[" | head -20
  ```

  Expected: errors about missing `clarify_route` field in `ValidationPlan { ... }` struct literal at line ~225, missing `clarify_route` in `ProgressCheck { ... }` at line ~292, missing `clarify_exhausted` in all `ExitDecision { ... }` constructors at lines ~338-376.

---

## Task 2: Write failing tests for AC-L1..L6

**Files:** `src/agent_guidance.rs` mod tests block (after line 641)

- [ ] **Step 1: Append the following tests inside `mod tests { ... }` at the bottom of the file**

  ```rust
  // ── Lifecycle clarification wiring (AC-L1 .. AC-L6) ─────────────────────────

  #[test]
  fn test_ac_l1_check_progress_uncertainty_zero_tiers_gives_self_prompt_t0() {
      let factors = ["deploy_ready", "tests_passing", "review_done", "auth_complete"];
      let obs = ["started something"];
      // 60% budget used, 0% progress → uncertainty_detected=true, tiers_spent=0 → T0
      let p = check_progress(&factors, &obs, 30, 50, 0, 1.0);
      assert!(p.uncertainty_detected);
      let cr = p.clarify_route.expect("clarify_route must be Some when uncertainty_detected");
      assert_eq!(cr.action, "self_prompt");
      assert_eq!(cr.tier, "T0_environment");
  }

  #[test]
  fn test_ac_l2_check_progress_ladder_spent_high_cost_asks_user() {
      let factors = ["deploy_ready", "tests_passing", "review_done", "auth_complete"];
      let obs = ["started something"];
      // tiers_spent=MAX → ladder spent; cost=1.0 → 1.0*1.0=1.0 > COST_OF_ASKING(0.25) → AskUser
      let p = check_progress(&factors, &obs, 30, 50, 3, 1.0);
      assert!(p.uncertainty_detected);
      let cr = p.clarify_route.expect("clarify_route must be Some");
      assert_eq!(cr.action, "ask_user");
  }

  #[test]
  fn test_ac_l3_check_progress_ladder_spent_low_cost_declines() {
      let factors = ["deploy_ready", "tests_passing", "review_done", "auth_complete"];
      let obs = ["started something"];
      // cost=0.1 → 1.0*0.1=0.1 ≤ COST_OF_ASKING(0.25) → DeclineAsk
      let p = check_progress(&factors, &obs, 30, 50, 3, 0.1);
      assert!(p.uncertainty_detected);
      let cr = p.clarify_route.expect("clarify_route must be Some");
      assert_eq!(cr.action, "decline_ask");
  }

  #[test]
  fn test_ac_l4_check_progress_no_uncertainty_no_clarify_route() {
      let factors = ["auth_complete", "tests_passing"];
      let obs = ["auth complete for JWT", "tests passing in CI"];
      // 10% budget, good progress → uncertainty_detected=false
      let p = check_progress(&factors, &obs, 5, 50, 0, 1.0);
      assert!(!p.uncertainty_detected);
      assert!(p.clarify_route.is_none(), "no route when no uncertainty");
  }

  #[test]
  fn test_ac_l5_plan_validation_unknown_factor_tiers_1_gives_t1() {
      // "xyzzy_plugh" is unrecognised → unknown_factors non-empty → clarify_route emitted
      // tiers_spent=1 → rungs[1] = T1_prior_art
      let plan = plan_validation(&["xyzzy_plugh_factor"], 1);
      assert!(!plan.unknown_factors.is_empty());
      let cr = plan.clarify_route.expect("clarify_route must be Some for unknown factors");
      assert_eq!(cr.action, "self_prompt");
      assert_eq!(cr.tier, "T1_prior_art");
  }

  #[test]
  fn test_ac_l6_should_exit_clarify_exhausted_when_tiers_max_and_stalled() {
      // tiers_spent=3 (MAX) and progress_ratio=0.05 (<0.1) → clarify_exhausted=true
      let d = should_exit(10, 50, 0.05, 0.1, 3);
      assert!(d.clarify_exhausted);
      // action stays Continue — exhaustion is informational, not a success/fail signal
      assert_eq!(d.action, ExitAction::Continue);
  }

  #[test]
  fn test_ac_l6b_should_exit_not_exhausted_when_progress_above_threshold() {
      // progress_ratio=0.5 (≥ 0.1) → clarify_exhausted=false even if tiers maxed
      let d = should_exit(10, 50, 0.5, 0.1, 3);
      assert!(!d.clarify_exhausted);
  }

  #[test]
  fn test_ac_l8_react_sim_tier_escalation() {
      // Simulate 4 iterations, each spending a rung; verify tier escalates T0→T1→T2→T3
      let factors = ["deploy_ready", "tests_passing", "review_done", "auth_complete"];
      let obs = ["minimal observation"];
      let expected_actions = ["self_prompt", "self_prompt", "self_prompt", "ask_user"];
      let expected_tiers = ["T0_environment", "T1_prior_art", "T2_causal", "T3_ask_user"];
      for (i, (exp_action, exp_tier)) in expected_actions.iter().zip(expected_tiers.iter()).enumerate() {
          let p = check_progress(&factors, &obs, 30, 50, i as u32, 1.0);
          let cr = p.clarify_route.expect("uncertainty triggers route");
          assert_eq!(&cr.action, exp_action, "iter {i}");
          assert_eq!(&cr.tier, exp_tier, "iter {i}");
      }
  }
  ```

- [ ] **Step 2: Verify tests fail at compile with function signature mismatches (expected)**

  ```sh
  cargo test --no-default-features --features petgraph_backend --lib agent_guidance 2>&1 | grep "error\[" | head -10
  ```

  Expected: errors on `check_progress(&factors, &obs, 30, 50, 0, 1.0)` — wrong number of args.

---

## Task 3: Extend `check_progress`, `plan_validation`, `should_exit`

**Files:** `src/agent_guidance.rs`

- [ ] **Step 1: Update `plan_validation` signature and body (currently at line 165)**

  Change:
  ```rust
  pub fn plan_validation(success_factors: &[&str]) -> ValidationPlan {
  ```
  To:
  ```rust
  pub fn plan_validation(success_factors: &[&str], tiers_spent: u32) -> ValidationPlan {
  ```

  And change the `ValidationPlan { ... }` constructor at line ~225 from:
  ```rust
  ValidationPlan {
      steps,
      unknown_factors: unknown,
      validation_order: "validate in order: data/scrape → implementation → tests → deploy → review".into(),
  }
  ```
  To:
  ```rust
  let clarify_route = if !unknown.is_empty() {
      Some(ClarifyRouteInfo::from(route_uncertainty(true, tiers_spent, true, 1.0)))
  } else {
      None
  };
  ValidationPlan {
      steps,
      unknown_factors: unknown,
      validation_order: "validate in order: data/scrape → implementation → tests → deploy → review".into(),
      clarify_route,
  }
  ```

- [ ] **Step 2: Update `check_progress` signature and body (currently at line 248)**

  Change:
  ```rust
  pub fn check_progress(
      success_factors: &[&str],
      observations: &[&str],
      iteration: u32,
      max_iterations: u32,
  ) -> ProgressCheck {
  ```
  To:
  ```rust
  pub fn check_progress(
      success_factors: &[&str],
      observations: &[&str],
      iteration: u32,
      max_iterations: u32,
      tiers_spent: u32,
      cost_of_wrong_execution: f64,
  ) -> ProgressCheck {
  ```

  And change the `ProgressCheck { ... }` constructor at line ~292 from:
  ```rust
  ProgressCheck {
      iteration,
      max_iterations,
      satisfied_factors: satisfied,
      pending_factors: pending,
      progress_ratio: progress,
      on_track,
      recommended_action,
      uncertainty_detected,
  }
  ```
  To:
  ```rust
  let clarify_route = if uncertainty_detected {
      Some(ClarifyRouteInfo::from(route_uncertainty(
          true, tiers_spent, false, cost_of_wrong_execution,
      )))
  } else {
      None
  };
  ProgressCheck {
      iteration,
      max_iterations,
      satisfied_factors: satisfied,
      pending_factors: pending,
      progress_ratio: progress,
      on_track,
      recommended_action,
      uncertainty_detected,
      clarify_route,
  }
  ```

- [ ] **Step 3: Update `should_exit` signature and all `ExitDecision` constructors (line 331)**

  Change:
  ```rust
  pub fn should_exit(
      iteration: u32,
      max_iterations: u32,
      progress_ratio: f32,
      surprise_signal: f32,
  ) -> ExitDecision {
  ```
  To:
  ```rust
  pub fn should_exit(
      iteration: u32,
      max_iterations: u32,
      progress_ratio: f32,
      surprise_signal: f32,
      tiers_spent: u32,
  ) -> ExitDecision {
  ```

  At the very top of the function body, add (before the `if progress_ratio >= 1.0` check):
  ```rust
  let clarify_exhausted = tiers_spent >= MAX_CLARIFY_TIERS && progress_ratio < 0.1;
  ```

  Then add `clarify_exhausted,` to every `ExitDecision { ... }` constructor in the function. There are 5 constructors (Succeed, Fail-max, Escalate, Fail-stuck, Continue). Each becomes:
  ```rust
  ExitDecision {
      action: ExitAction::Succeed,   // (or Fail/Escalate/Continue as appropriate)
      rationale: "...".into(),
      next_step: "...".into(),
      clarify_exhausted,
  }
  ```

---

## Task 4: Fix existing test call sites

**Files:** `src/agent_guidance.rs` mod tests

All existing tests that call the 3 extended functions need the new params added. The values are backward-compatible defaults.

- [ ] **Step 1: Fix `test_plan_validation_known_factors` (line 483)**

  Change:
  ```rust
  let plan = plan_validation(&["tests_passing", "auth_system_tested", "data_collected"]);
  ```
  To:
  ```rust
  let plan = plan_validation(&["tests_passing", "auth_system_tested", "data_collected"], 0);
  ```

- [ ] **Step 2: Fix `test_check_progress_satisfied` (line 493)**

  Change:
  ```rust
  let p = check_progress(&factors, &obs, 5, 50);
  ```
  To:
  ```rust
  let p = check_progress(&factors, &obs, 5, 50, 0, 1.0);
  ```

- [ ] **Step 3: Fix `test_check_progress_uncertainty` (line 502)**

  Change:
  ```rust
  let p = check_progress(&factors, &obs, 30, 50); // 60% budget, minimal progress
  ```
  To:
  ```rust
  let p = check_progress(&factors, &obs, 30, 50, 0, 1.0); // 60% budget, minimal progress
  ```

- [ ] **Step 4: Fix all `should_exit` test call sites (lines 509, 515, 522, 528, 535)**

  ```rust
  // test_should_exit_succeed (line 509)
  let d = should_exit(10, 50, 1.0, 0.1, 0);
  // test_should_exit_max_iterations (line 515)
  let d = should_exit(50, 50, 0.6, 0.2, 0);
  // test_should_exit_escalate (line 522)
  let d = should_exit(30, 50, 0.3, 0.9, 0);
  // test_should_exit_stuck (line 528)
  let d = should_exit(15, 50, 0.0, 0.2, 0);
  // test_should_exit_continue (line 535)
  let d = should_exit(5, 50, 0.4, 0.3, 0);
  ```

- [ ] **Step 5: Run lib tests and verify all pass**

  ```sh
  cargo test --no-default-features --features petgraph_backend --lib agent_guidance 2>&1 | tail -20
  ```

  Expected: `test result: ok. N passed; 0 failed` where N ≥ 20 (all old tests + 8 new AC tests).

---

## Task 5: Update web server handlers

**Files:** `src/web_server.rs`

Three handlers at lines 7108-7139. Each needs to extract the new params with defaults.

- [ ] **Step 1: Update `handle_agent_plan_validation` (line 7108)**

  Change:
  ```rust
  async fn handle_agent_plan_validation(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
      let factors: Vec<String> = req.get("success_factors")
          .and_then(|v| serde_json::from_value(v.clone()).ok())
          .unwrap_or_default();
      let refs: Vec<&str> = factors.iter().map(|s| s.as_str()).collect();
      let plan = crate::agent_guidance::plan_validation(&refs);
      Json(serde_json::to_value(plan).unwrap_or(serde_json::json!({"error": "serialization failed"})))
  }
  ```
  To:
  ```rust
  async fn handle_agent_plan_validation(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
      let factors: Vec<String> = req.get("success_factors")
          .and_then(|v| serde_json::from_value(v.clone()).ok())
          .unwrap_or_default();
      let tiers_spent = req.get("tiers_spent").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
      let refs: Vec<&str> = factors.iter().map(|s| s.as_str()).collect();
      let plan = crate::agent_guidance::plan_validation(&refs, tiers_spent);
      Json(serde_json::to_value(plan).unwrap_or(serde_json::json!({"error": "serialization failed"})))
  }
  ```

- [ ] **Step 2: Update `handle_agent_check_progress` (line 7117)**

  Change:
  ```rust
  async fn handle_agent_check_progress(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
      let factors: Vec<String> = req.get("success_factors")
          .and_then(|v| serde_json::from_value(v.clone()).ok())
          .unwrap_or_default();
      let obs: Vec<String> = req.get("observations")
          .and_then(|v| serde_json::from_value(v.clone()).ok())
          .unwrap_or_default();
      let iteration = req.get("iteration").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
      let max = req.get("max_iterations").and_then(|v| v.as_u64()).unwrap_or(20) as u32;
      let f_refs: Vec<&str> = factors.iter().map(|s| s.as_str()).collect();
      let o_refs: Vec<&str> = obs.iter().map(|s| s.as_str()).collect();
      let check = crate::agent_guidance::check_progress(&f_refs, &o_refs, iteration, max);
      Json(serde_json::to_value(check).unwrap_or(serde_json::json!({"error": "serialization failed"})))
  }
  ```
  To:
  ```rust
  async fn handle_agent_check_progress(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
      let factors: Vec<String> = req.get("success_factors")
          .and_then(|v| serde_json::from_value(v.clone()).ok())
          .unwrap_or_default();
      let obs: Vec<String> = req.get("observations")
          .and_then(|v| serde_json::from_value(v.clone()).ok())
          .unwrap_or_default();
      let iteration = req.get("iteration").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
      let max = req.get("max_iterations").and_then(|v| v.as_u64()).unwrap_or(20) as u32;
      let tiers_spent = req.get("tiers_spent").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
      let cost_of_wrong_execution = req.get("cost_of_wrong_execution").and_then(|v| v.as_f64()).unwrap_or(1.0);
      let f_refs: Vec<&str> = factors.iter().map(|s| s.as_str()).collect();
      let o_refs: Vec<&str> = obs.iter().map(|s| s.as_str()).collect();
      let check = crate::agent_guidance::check_progress(&f_refs, &o_refs, iteration, max, tiers_spent, cost_of_wrong_execution);
      Json(serde_json::to_value(check).unwrap_or(serde_json::json!({"error": "serialization failed"})))
  }
  ```

- [ ] **Step 3: Update `handle_agent_should_exit` (line 7132)**

  Change:
  ```rust
  async fn handle_agent_should_exit(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
      let iteration = req.get("iteration").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
      let max = req.get("max_iterations").and_then(|v| v.as_u64()).unwrap_or(20) as u32;
      let progress = req.get("progress_ratio").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
      let surprise = req.get("surprise_signal").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
      let decision = crate::agent_guidance::should_exit(iteration, max, progress, surprise);
      Json(serde_json::to_value(decision).unwrap_or(serde_json::json!({"error": "serialization failed"})))
  }
  ```
  To:
  ```rust
  async fn handle_agent_should_exit(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
      let iteration = req.get("iteration").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
      let max = req.get("max_iterations").and_then(|v| v.as_u64()).unwrap_or(20) as u32;
      let progress = req.get("progress_ratio").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
      let surprise = req.get("surprise_signal").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
      let tiers_spent = req.get("tiers_spent").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
      let decision = crate::agent_guidance::should_exit(iteration, max, progress, surprise, tiers_spent);
      Json(serde_json::to_value(decision).unwrap_or(serde_json::json!({"error": "serialization failed"})))
  }
  ```

- [ ] **Step 4: Full build + all lib tests**

  ```sh
  cargo build --no-default-features --features petgraph_backend 2>&1 | grep "^error" | head -10
  cargo test  --no-default-features --features petgraph_backend --lib 2>&1 | tail -5
  ```

  Expected: `0 errors` build; `test result: ok. N passed; 0 failed`.

- [ ] **Step 5: Commit**

  ```sh
  git add src/agent_guidance.rs src/web_server.rs
  git commit -m "feat: wire ClarifyRouteInfo into lifecycle gates (AC-L1..L8)"
  ```

---

## Task 6: MCP Python server — schemas + handlers

**Files:** `sdk/mcp/server.py`

Three schema blocks (lines 904–944) and three handler functions (lines 1768–1788).

- [ ] **Step 1: Update `plan_validation` schema (line 906-916)**

  Add after `"success_factors"` property block, still inside `"properties"`:
  ```python
  "tiers_spent": {
      "type": "integer",
      "default": 0,
      "description": "Self-prompt rungs spent so far for this goal (0 = none spent). Drives clarify_route tier in response.",
  },
  ```

- [ ] **Step 2: Update `check_progress` schema (line 921-929)**

  Add after `"max_iterations"` property block, still inside `"properties"`:
  ```python
  "tiers_spent": {
      "type": "integer",
      "default": 0,
      "description": "Self-prompt rungs spent for this goal. Drives clarify_route.tier in response when uncertainty_detected.",
  },
  "cost_of_wrong_execution": {
      "type": "number",
      "default": 1.0,
      "description": "Relative cost of acting on a wrong assumption (0.0–2.0). Higher biases toward asking the user once the ladder is spent.",
  },
  ```

- [ ] **Step 3: Update `should_exit` schema (line 935-944)**

  Add after `"surprise_signal"` property block, still inside `"properties"`:
  ```python
  "tiers_spent": {
      "type": "integer",
      "default": 0,
      "description": "Self-prompt rungs spent. Populates clarify_exhausted=true when tiers_spent>=MAX_CLARIFY_TIERS and progress stalled.",
  },
  ```

- [ ] **Step 4: Update `handle_plan_validation` (line 1768)**

  Change:
  ```python
  def handle_plan_validation(args: dict) -> str:
      r = _req("POST", "/agent/plan-validation", {"success_factors": args.get("success_factors", [])})
      return json.dumps(r)
  ```
  To:
  ```python
  def handle_plan_validation(args: dict) -> str:
      r = _req("POST", "/agent/plan-validation", {
          "success_factors": args.get("success_factors", []),
          "tiers_spent": args.get("tiers_spent", 0),
      })
      return json.dumps(r)
  ```

- [ ] **Step 5: Update `handle_check_progress` (line 1772)**

  Change:
  ```python
  def handle_check_progress(args: dict) -> str:
      r = _req("POST", "/agent/check-progress", {
          "success_factors": args.get("success_factors", []),
          "observations": args.get("observations", []),
          "iteration": args.get("iteration", 0),
          "max_iterations": args.get("max_iterations", 20),
      })
      return json.dumps(r)
  ```
  To:
  ```python
  def handle_check_progress(args: dict) -> str:
      r = _req("POST", "/agent/check-progress", {
          "success_factors": args.get("success_factors", []),
          "observations": args.get("observations", []),
          "iteration": args.get("iteration", 0),
          "max_iterations": args.get("max_iterations", 20),
          "tiers_spent": args.get("tiers_spent", 0),
          "cost_of_wrong_execution": args.get("cost_of_wrong_execution", 1.0),
      })
      return json.dumps(r)
  ```

- [ ] **Step 6: Update `handle_should_exit` (line 1781)**

  Change:
  ```python
  def handle_should_exit(args: dict) -> str:
      r = _req("POST", "/agent/should-exit", {
          "iteration": args.get("iteration", 0),
          "max_iterations": args.get("max_iterations", 20),
          "progress_ratio": args.get("progress_ratio", 0.0),
          "surprise_signal": args.get("surprise_signal", 0.0),
      })
      return json.dumps(r)
  ```
  To:
  ```python
  def handle_should_exit(args: dict) -> str:
      r = _req("POST", "/agent/should-exit", {
          "iteration": args.get("iteration", 0),
          "max_iterations": args.get("max_iterations", 20),
          "progress_ratio": args.get("progress_ratio", 0.0),
          "surprise_signal": args.get("surprise_signal", 0.0),
          "tiers_spent": args.get("tiers_spent", 0),
      })
      return json.dumps(r)
  ```

- [ ] **Step 7: Verify MCP tool count unchanged (still 62)**

  ```sh
  cd sdk/python && python -c "
  import re
  txt = open('../../sdk/mcp/server.py').read()
  names = re.findall(r'\"name\": \"([^\"]+)\"', txt)
  print(f'tool+resource names found: {len(names)}')
  "
  ```

  The test `test_mcp_tool_surface.py` asserts exactly 62 tools. We're not adding new tools, only adding params to 3 existing ones — count must stay at 62.

---

## Task 7: Python test AC-L7

**Files:** `sdk/python/tests/test_lifecycle_clarify_mcp.py` (create new)

AC-L7 verifies the MCP handler constructs the REST body correctly with `tiers_spent`.

- [ ] **Step 1: Create the test file**

  ```python
  """AC-L7: MCP lifecycle clarify handler passes tiers_spent through to REST body."""
  import json
  from unittest.mock import patch, MagicMock


  def _make_req_capture():
      """Return a mock _req and a list that captures calls."""
      calls = []

      def fake_req(method, path, body=None, **kwargs):
          calls.append({"method": method, "path": path, "body": body})
          return {"ok": True}

      return fake_req, calls


  def test_check_progress_passes_tiers_spent():
      import importlib, sys
      # Patch _req before importing handler logic via the module
      fake_req, calls = _make_req_capture()
      with patch.dict("sys.modules", {}):
          import sdk.mcp.server as srv  # noqa: F401 — imported for side effects
      # Call handler directly with tiers_spent=2
      with patch("sdk.mcp.server._req", side_effect=fake_req):
          import sdk.mcp.server as srv2
          srv2.handle_check_progress({
              "success_factors": ["tests_passing"],
              "observations": ["started"],
              "iteration": 5,
              "max_iterations": 20,
              "tiers_spent": 2,
              "cost_of_wrong_execution": 0.8,
          })
      assert len(calls) == 1
      body = calls[0]["body"]
      assert body["tiers_spent"] == 2
      assert body["cost_of_wrong_execution"] == 0.8


  def test_plan_validation_passes_tiers_spent():
      fake_req, calls = _make_req_capture()
      with patch("sdk.mcp.server._req", side_effect=fake_req):
          import sdk.mcp.server as srv
          srv.handle_plan_validation({
              "success_factors": ["tests_passing"],
              "tiers_spent": 1,
          })
      assert calls[0]["body"]["tiers_spent"] == 1


  def test_should_exit_passes_tiers_spent():
      fake_req, calls = _make_req_capture()
      with patch("sdk.mcp.server._req", side_effect=fake_req):
          import sdk.mcp.server as srv
          srv.handle_should_exit({
              "iteration": 5,
              "max_iterations": 20,
              "progress_ratio": 0.3,
              "surprise_signal": 0.1,
              "tiers_spent": 3,
          })
      assert calls[0]["body"]["tiers_spent"] == 3
  ```

  **Note:** The mock/import approach above is fragile for the way `sdk/mcp/server.py` is structured. If the server module has top-level side effects or uses `_req` as a module-level name that's hard to patch from outside, prefer this simpler pattern:

  ```python
  """Simpler AC-L7: directly inspect handler body construction without import gymnastics."""
  import json, sys, types, unittest.mock as mock

  def _load_handlers():
      """Load only the handler functions from server.py with _req mocked."""
      fake = mock.MagicMock(return_value={"ok": True})
      # Patch before module load
      with mock.patch.dict(sys.modules, {}):
          import importlib
          with mock.patch("builtins.__import__", side_effect=lambda *a, **k: __import__(*a, **k)):
              pass
      # Directly exec the source with _req replaced
      src = open("sdk/mcp/server.py").read()
      ns = {"_req": fake, "json": json}
      exec(compile(src, "server.py", "exec"), ns)
      return ns, fake

  def test_check_progress_tiers_spent_forwarded():
      ns, fake_req = _load_handlers()
      ns["handle_check_progress"]({"success_factors": ["f"], "observations": ["o"],
                                    "iteration": 5, "max_iterations": 20,
                                    "tiers_spent": 2, "cost_of_wrong_execution": 0.8})
      call_body = fake_req.call_args[0][2]
      assert call_body["tiers_spent"] == 2
      assert call_body["cost_of_wrong_execution"] == 0.8

  def test_plan_validation_tiers_spent_forwarded():
      ns, fake_req = _load_handlers()
      ns["handle_plan_validation"]({"success_factors": ["f"], "tiers_spent": 1})
      assert fake_req.call_args[0][2]["tiers_spent"] == 1

  def test_should_exit_tiers_spent_forwarded():
      ns, fake_req = _load_handlers()
      ns["handle_should_exit"]({"iteration": 5, "max_iterations": 20,
                                 "progress_ratio": 0.3, "surprise_signal": 0.0,
                                 "tiers_spent": 3})
      assert fake_req.call_args[0][2]["tiers_spent"] == 3
  ```

  Use whichever pattern works given the module's import structure. The critical assertion is `call_body["tiers_spent"] == <expected_int>`.

- [ ] **Step 2: Run the new test**

  ```sh
  cd sdk/python && pytest tests/test_lifecycle_clarify_mcp.py -v 2>&1 | tail -15
  ```

  Expected: `3 passed`.

- [ ] **Step 3: Run full Python suite**

  ```sh
  cd sdk/python && pytest tests/ -v 2>&1 | tail -10
  ```

  Expected: all existing tests still pass (doctor, mcp_tool_surface, passive_observers).

- [ ] **Step 4: Commit**

  ```sh
  git add sdk/mcp/server.py sdk/python/tests/test_lifecycle_clarify_mcp.py
  git commit -m "feat: MCP lifecycle tools pass tiers_spent + cost_of_wrong_execution to REST (AC-L7)"
  ```

---

## Task 8: Update SKILL.md

**Files:** `sdk/python/hipcortex/install/SKILL.md`

PROGRESS and EXIT lines currently show 4-param signatures. Update to show the new params.

- [ ] **Step 1: Find PROGRESS line (around line 48)**

  Change:
  ```
  PROGRESS: check_progress(success_factors, observations, iteration, max_iterations)
  ```
  To:
  ```
  PROGRESS: check_progress(success_factors, observations, iteration, max_iterations, tiers_spent, cost_of_wrong_execution)
            → clarify_route: {action, tier, rationale} when uncertainty_detected; increment tiers_spent after each SelfPrompt rung
  ```

- [ ] **Step 2: Find EXIT line (around line 52)**

  Change:
  ```
  EXIT:     should_exit(iteration, max_iterations, progress_ratio, surprise_signal)
  ```
  To:
  ```
  EXIT:     should_exit(iteration, max_iterations, progress_ratio, surprise_signal, tiers_spent)
            → clarify_exhausted: true when tiers_spent>=MAX_CLARIFY_TIERS and progress stalled — informational, not a success/fail signal
  ```

- [ ] **Step 3: Commit**

  ```sh
  git add sdk/python/hipcortex/install/SKILL.md
  git commit -m "docs(SKILL.md): show tiers_spent params in PROGRESS and EXIT sections"
  ```

---

## Task 9: Fix HC offline — Layer 1 (global MCP config)

**Files:** `~/.claude/mcp.json`

The global config currently points to the stale 3.12.0 VSIX launcher at `c:\Users\user\.vscode\extensions\farmountain.hipcortex-memory-3.12.0\server\launcher.py`. Replace with the live project launcher (same as `.mcp.json`).

- [ ] **Step 1: Update `~/.claude/mcp.json`**

  Replace the entire file content with:
  ```json
  {
    "mcpServers": {
      "hipcortex": {
        "type": "stdio",
        "command": "C:/Users/user/AppData/Local/Microsoft/WindowsApps/PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0/python.exe",
        "args": [
          "D:/all_projects/HipCortex/scripts/hipcortex_mcp_launcher.py"
        ],
        "env": {
          "HIPCORTEX_URL": "http://localhost:3030",
          "HIPCORTEX_TIMEOUT": "15",
          "HIPCORTEX_ACTOR": "kaggle_winning_agentic_workflow"
        }
      }
    }
  }
  ```

  The `HIPCORTEX_ACTOR` from the old config is preserved so existing memory attribution stays consistent.

- [ ] **Step 2: Verify launcher exists**

  ```sh
  ls D:/all_projects/HipCortex/scripts/hipcortex_mcp_launcher.py
  ```

  If file not found: check `scripts/` directory. If empty: the MCP launcher should be at the path used by `.mcp.json`.

---

## Task 10: Fix HC offline — Layer 2 (SessionStart hook + server auto-start script)

**Files:** `scripts/hipcortex_start_if_dead.py` (create), `~/.claude/settings.json` (modify)

- [ ] **Step 1: Create `scripts/hipcortex_start_if_dead.py`**

  ```python
  #!/usr/bin/env python3
  """SessionStart hook: ensure HipCortex REST backend is alive on port 3030."""
  import os, socket, subprocess, sys, time

  HIPCORTEX_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
  SERVER_BIN = os.path.join(HIPCORTEX_DIR, "target", "release", "webserver.exe")
  PORT = int(os.getenv("HIPCORTEX_PORT", "3030"))
  LOG_PATH = os.path.join(os.environ.get("TEMP", "C:/Temp"), "hipcortex-server.log")


  def _port_open(port: int) -> bool:
      try:
          with socket.create_connection(("127.0.0.1", port), timeout=1.0):
              return True
      except OSError:
          return False


  def main() -> None:
      if _port_open(PORT):
          print(f"[hipcortex] server already running on :{PORT}", flush=True)
          return

      if not os.path.isfile(SERVER_BIN):
          print(f"[hipcortex] binary not found at {SERVER_BIN} — skipping start", flush=True)
          return

      try:
          with open(LOG_PATH, "w") as log:
              subprocess.Popen(
                  [SERVER_BIN],
                  stdout=log,
                  stderr=subprocess.STDOUT,
                  cwd=HIPCORTEX_DIR,
                  creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
              )
      except OSError as exc:
          print(f"[hipcortex] failed to spawn binary: {exc}", flush=True)
          return

      for i in range(20):
          time.sleep(0.5)
          if _port_open(PORT):
              print(f"[hipcortex] server started (took {(i + 1) * 0.5:.1f}s)", flush=True)
              return

      print("[hipcortex] server did not come up in 10s — continuing without backend", flush=True)


  if __name__ == "__main__":
      main()
  ```

- [ ] **Step 2: Test script manually**

  ```sh
  python D:/all_projects/HipCortex/scripts/hipcortex_start_if_dead.py
  ```

  If server already running: prints `[hipcortex] server already running on :3030`.
  If binary missing: prints `[hipcortex] binary not found...`.
  If started: prints `[hipcortex] server started (took N.Ns)`.

- [ ] **Step 3: Add SessionStart hook to `~/.claude/settings.json`**

  In the `"SessionStart"` array (currently has 3 hook objects for caveman, headroom-init, headroom-session-start), add a 4th object **after the existing ones**:

  ```json
  {
    "matcher": "startup|resume",
    "hooks": [
      {
        "type": "command",
        "command": "\"C:/Users/user/AppData/Local/Microsoft/WindowsApps/PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0/python.exe\" \"D:/all_projects/HipCortex/scripts/hipcortex_start_if_dead.py\"",
        "timeout": 15,
        "statusMessage": "Ensuring HipCortex backend is running..."
      }
    ]
  }
  ```

  The hook must have `"timeout": 15` to cover the 10s wait in the script plus overhead.

- [ ] **Step 4: Commit the start script (not settings.json — it's outside the repo)**

  ```sh
  git add scripts/hipcortex_start_if_dead.py
  git commit -m "ops: add SessionStart server health script for HC offline auto-recovery"
  ```

---

## Task 11: Fix HC offline — Layer 3 (VS Code extension reconnect interval)

**Files:** `vscode-extension/src/extension.ts`

The extension must periodically re-check the backend and reconnect if it died (e.g. after VS Code restart or system sleep).

- [ ] **Step 1: Search for existing reconnect interval**

  ```sh
  grep -n "setInterval\|reconnect\|healthCheck\|healthInterval" vscode-extension/src/extension.ts | head -20
  ```

  If a `setInterval` calling `healthCheck()` or `reconnect()` is found: verify it runs at least every 30 seconds and calls `startServer()` or equivalent if the check fails. Skip Step 2 if already present.

- [ ] **Step 2: Add reconnect interval if absent**

  Find the `activate` function body (or the section where `startServer()` is called on activation). After the initial `startServer()` call, add:

  ```typescript
  // Reconnect if backend dies mid-session (e.g. after VS Code restart or sleep)
  const _healthInterval = setInterval(async () => {
      if (hipcortexService) {
          try {
              const ok = await hipcortexService.healthCheck();
              if (!ok) {
                  await hipcortexService.startServer();
              }
          } catch {
              // ignore — server may be starting
          }
      }
  }, 30_000);
  context.subscriptions.push({ dispose: () => clearInterval(_healthInterval) });
  ```

  Adapt method names to match what the existing `hipcortexService` object exposes (`healthCheck`, `startServer`, or equivalent). The dispose registration ensures the interval is cleared on extension deactivation.

- [ ] **Step 3: Build extension**

  ```sh
  cd vscode-extension && npm run compile 2>&1 | tail -10
  ```

  Expected: `0 errors`.

- [ ] **Step 4: Commit**

  ```sh
  git add vscode-extension/src/extension.ts
  git commit -m "fix(extension): add 30s health reconnect interval for mid-session backend recovery"
  ```

---

## Task 12: Full test suite + integration verification + push

- [ ] **Step 1: Run all Rust test suites**

  ```sh
  cargo test --no-default-features --features petgraph_backend --lib         2>&1 | tail -5
  cargo test --no-default-features --features petgraph_backend --test unit_suite         2>&1 | tail -5
  cargo test --no-default-features --features petgraph_backend --test integration_suite  2>&1 | tail -5
  cargo test --no-default-features --features petgraph_backend --test property_suite     2>&1 | tail -5
  ```

  Expected: all 4 suites `ok. N passed; 0 failed`.

- [ ] **Step 2: Run full Python test suite**

  ```sh
  cd sdk/python && pytest tests/ -v 2>&1 | tail -15
  ```

  Expected: all tests pass including the new `test_lifecycle_clarify_mcp.py`.

- [ ] **Step 3: AC checklist verification**

  Run each AC test in isolation to confirm:
  ```sh
  cargo test --no-default-features --features petgraph_backend --lib test_ac_l1 -- --nocapture
  cargo test --no-default-features --features petgraph_backend --lib test_ac_l2 -- --nocapture
  cargo test --no-default-features --features petgraph_backend --lib test_ac_l3 -- --nocapture
  cargo test --no-default-features --features petgraph_backend --lib test_ac_l4 -- --nocapture
  cargo test --no-default-features --features petgraph_backend --lib test_ac_l5 -- --nocapture
  cargo test --no-default-features --features petgraph_backend --lib test_ac_l6 -- --nocapture
  cargo test --no-default-features --features petgraph_backend --lib test_ac_l8 -- --nocapture
  ```

  All 7 must print `ok`.

- [ ] **Step 4: Verify AC-L1 spec invariant (clarify_route absent on happy path)**

  ```sh
  cargo test --no-default-features --features petgraph_backend --lib test_ac_l4 -- --nocapture
  ```

  Must output `clarify_route is None: true` or similar (use `--nocapture` to see `dbg!` if you add one).

- [ ] **Step 5: Final commit + push**

  ```sh
  git add -p   # stage any remaining changes
  git status   # verify nothing unexpected staged
  git commit -m "feat(v3.14.1): lifecycle clarification wiring + HC offline three-layer fix"
  git push origin main
  ```

  **Do NOT commit:**
  - `KARM_Kakeya_Abstraction_Action_Model_Handover.md`
  - `src/commercial.rs`
  - `src/bin/pro_server.rs`
  - `~/.claude/mcp.json` (outside repo)
  - `~/.claude/settings.json` (outside repo)

---

## Acceptance Criteria Traceability

| AC | Test | Task |
|----|------|------|
| AC-L1: `check_progress` uncertainty tiers_spent=0 → self_prompt T0 | `test_ac_l1_*` | Task 3 |
| AC-L2: tiers_spent=3 cost=1.0 → ask_user | `test_ac_l2_*` | Task 3 |
| AC-L3: tiers_spent=3 cost=0.1 → decline_ask | `test_ac_l3_*` | Task 3 |
| AC-L4: no uncertainty → clarify_route absent | `test_ac_l4_*` | Task 3 |
| AC-L5: plan_validation unknown factor tiers_spent=1 → T1_prior_art | `test_ac_l5_*` | Task 3 |
| AC-L6: should_exit tiers_spent≥MAX stalled → clarify_exhausted=true | `test_ac_l6_*` | Task 3 |
| AC-L7: MCP handler passes tiers_spent to REST | `test_lifecycle_clarify_mcp.py` | Task 6+7 |
| AC-L8: 4-iter ReAct sim T0→T1→T2→T3 escalation | `test_ac_l8_*` | Task 3 |
| AC-O1: HC online in non-VS Code Claude Code session | manual: `add_memory` from CLI | Task 9 |
| AC-O2: port 3030 killed → new session auto-starts via hook | manual: kill + restart | Task 10 |
| AC-O3: `hipcortex doctor` reports ok, version 3.14.0 | `pytest tests/test_doctor.py` | Task 10 |
