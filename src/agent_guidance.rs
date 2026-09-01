// Agent Lifecycle Self-Prompting: structured guidance at every stage of the ReAct loop.
// All functions return deterministic structured output — no LLM calls, no I/O.
// Use these at each lifecycle phase so the agent self-prompts before acting.

use serde::{Deserialize, Serialize};

// ── Phase 1: Goal Clarification ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct GoalClarification {
    pub task_summary: String,
    pub clarifying_questions: Vec<String>,
    pub suggested_success_factors: Vec<String>,
    pub suggested_max_iterations: u32,
    pub uncertainty_flags: Vec<String>,
    pub recommended_first_action: String,
}

/// Turns a vague task description into a structured GoalPayload scaffold.
/// Agent should ask the user the clarifying_questions if any are flagged as blocking.
pub fn clarify_goal(task: &str) -> GoalClarification {
    let t = task.to_lowercase();
    let is_vague = t.split_whitespace().count() < 8
        || t.contains("something") || t.contains("maybe") || t.contains("kind of")
        || t.contains("possibly") || t.contains("not sure");
    let is_web = t.contains("scrape") || t.contains("research") || t.contains("trip")
        || t.contains("hotel") || t.contains("itiner") || t.contains("flight");
    let is_dev = t.contains("build") || t.contains("implement") || t.contains("create")
        || t.contains("develop") || t.contains("deploy");
    let is_open_ended = t.contains("anything") || t.contains("whatever") || t.contains("help me")
        || t.contains("explore") || t.split_whitespace().count() < 4;

    let mut questions = Vec::new();
    let mut flags = Vec::new();
    let mut factors = Vec::new();

    if is_vague || is_open_ended {
        questions.push("What is the specific end state you consider 'done'?".into());
        flags.push("task_too_vague: success criteria unclear".into());
    }
    if is_dev {
        questions.push("What tech stack or constraints must be respected?".into());
        questions.push("How will you verify the feature works (test, demo, CI)?".into());
        factors.push("implementation_complete".into());
        factors.push("tests_passing".into());
        factors.push("code_reviewed".into());
    }
    if is_web {
        questions.push("Which data sources / websites are in scope?".into());
        questions.push("What is the output format (JSON, table, summary)?".into());
        factors.push("data_collected".into());
        factors.push("sources_verified".into());
        factors.push("output_formatted".into());
    }
    if !is_dev && !is_web {
        questions.push("What does a correct, complete answer look like?".into());
        factors.push("goal_achieved".into());
    }

    let max_iter = if is_dev { 50 } else if is_web { 30 } else { 20 };

    GoalClarification {
        task_summary: task.to_string(),
        clarifying_questions: questions,
        suggested_success_factors: factors,
        suggested_max_iterations: max_iter,
        uncertainty_flags: flags,
        recommended_first_action: if is_vague {
            "ask_user_clarifying_questions".into()
        } else {
            "call recommend_tools then get_live_beliefs".into()
        },
    }
}

// ── Phase 2: Validation Planning ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationStep {
    pub success_factor: String,
    pub verification_method: String,
    pub tool_or_command: String,
    pub expected_evidence: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationPlan {
    pub steps: Vec<ValidationStep>,
    pub unknown_factors: Vec<String>,
    pub validation_order: String,
}

/// Given a list of success_factors, returns a plan for how to verify each one.
pub fn plan_validation(success_factors: &[&str]) -> ValidationPlan {
    let mut steps = Vec::new();
    let mut unknown = Vec::new();

    for &factor in success_factors {
        let f = factor.to_lowercase();
        let step = if f.contains("test") || f.contains("passing") || f.contains("green") {
            ValidationStep {
                success_factor: factor.into(),
                verification_method: "run_tests".into(),
                tool_or_command: "cargo test / npm test / pytest".into(),
                expected_evidence: "test output shows 0 failures".into(),
            }
        } else if f.contains("deploy") || f.contains("running") || f.contains("live") {
            ValidationStep {
                success_factor: factor.into(),
                verification_method: "health_check".into(),
                tool_or_command: "curl /health or kubectl get pods".into(),
                expected_evidence: "HTTP 200 or pod status=Running".into(),
            }
        } else if f.contains("data") || f.contains("collect") || f.contains("scrape") {
            ValidationStep {
                success_factor: factor.into(),
                verification_method: "inspect_stored_records".into(),
                tool_or_command: "GET /memory/search-flat?query=<factor>".into(),
                expected_evidence: "≥1 Temporal record matching the factor".into(),
            }
        } else if f.contains("cost") || f.contains("budget") || f.contains("price") {
            ValidationStep {
                success_factor: factor.into(),
                verification_method: "numeric_check".into(),
                tool_or_command: "GET /memory/search-flat?query=cost".into(),
                expected_evidence: "memory record with numeric value in target field".into(),
            }
        } else if f.contains("review") || f.contains("approved") || f.contains("audit") {
            ValidationStep {
                success_factor: factor.into(),
                verification_method: "human_review_or_static_analysis".into(),
                tool_or_command: "cargo clippy / eslint / SAST tool".into(),
                expected_evidence: "0 warnings or reviewer approval recorded in substrate".into(),
            }
        } else if f.contains("auth") || f.contains("login") || f.contains("session") {
            ValidationStep {
                success_factor: factor.into(),
                verification_method: "e2e_auth_flow".into(),
                tool_or_command: "playwright test: login → protected route → logout".into(),
                expected_evidence: "test passes; session token stored in memory".into(),
            }
        } else {
            unknown.push(factor.to_string());
            ValidationStep {
                success_factor: factor.into(),
                verification_method: "substrate_record_check".into(),
                tool_or_command: "GET /memory/search-flat?query=<factor>".into(),
                expected_evidence: "Temporal record confirming the factor is satisfied".into(),
            }
        };
        steps.push(step);
    }

    ValidationPlan {
        steps,
        unknown_factors: unknown,
        validation_order: "validate in order: data/scrape → implementation → tests → deploy → review".into(),
    }
}

// ── Phase 3: Progress Check ───────────────────────────────────────────────────

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

/// Per-iteration check: given observations (stored Temporal record targets),
/// which success_factors are satisfied? What should the agent do next?
pub fn check_progress(
    success_factors: &[&str],
    observations: &[&str],
    iteration: u32,
    max_iterations: u32,
) -> ProgressCheck {
    let obs_lower: Vec<String> = observations.iter().map(|o| o.to_lowercase()).collect();

    let mut satisfied = Vec::new();
    let mut pending = Vec::new();

    for &factor in success_factors {
        let f_lower = factor.to_lowercase();
        let keywords: Vec<&str> = f_lower.split('_').collect();
        let is_satisfied = obs_lower.iter().any(|obs| {
            keywords.iter().all(|kw| kw.len() < 2 || obs.contains(*kw))
        });
        if is_satisfied {
            satisfied.push(factor.to_string());
        } else {
            pending.push(factor.to_string());
        }
    }

    let total = success_factors.len() as f32;
    let progress = if total == 0.0 { 1.0 } else { satisfied.len() as f32 / total };
    let pct_iters_used = if max_iterations == 0 { 0.0 } else { iteration as f32 / max_iterations as f32 };

    // Uncertainty: using more than half iterations but less than half factors satisfied
    let uncertainty_detected = pct_iters_used > 0.5 && progress < 0.5;
    let on_track = !uncertainty_detected;

    let recommended_action = if pending.is_empty() {
        "all_success_factors_satisfied: call should_exit to confirm completion".into()
    } else if uncertainty_detected {
        format!(
            "UNCERTAINTY: {:.0}% iters used, only {:.0}% factors satisfied. \
             Call POST /memory/reflect for substrate CoT, then POST /v1/loop/omega for attribution.",
            pct_iters_used * 100.0, progress * 100.0
        )
    } else {
        format!("continue: focus next iteration on satisfying: {}", pending.join(", "))
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
    }
}

// ── Phase 4: Exit Decision ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExitAction {
    Continue,
    Succeed,
    Fail,
    Escalate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExitDecision {
    pub action: ExitAction,
    pub rationale: String,
    pub next_step: String,
}

/// Determines whether the ReAct loop should continue, succeed, fail, or escalate.
/// Call at the end of every iteration. This is the only safe loop-exit gate.
///
/// Rules (evaluated in priority order):
///   1. progress == 1.0 → Succeed
///   2. iteration >= max → Fail (hard cap prevents indefinite loops)
///   3. High surprise (>0.8) with >50% iters used and <50% progress → Escalate
///   4. Zero progress after 25% of budget → Fail (stuck detection)
///   5. else → Continue
pub fn should_exit(
    iteration: u32,
    max_iterations: u32,
    progress_ratio: f32,
    surprise_signal: f32,
) -> ExitDecision {
    if progress_ratio >= 1.0 {
        return ExitDecision {
            action: ExitAction::Succeed,
            rationale: "all success_factors satisfied".into(),
            next_step: "store final summary via add_memory(Reflexion), then exit loop".into(),
        };
    }
    if iteration >= max_iterations {
        return ExitDecision {
            action: ExitAction::Fail,
            rationale: format!("reached max_iterations={max_iterations} with progress={:.0}%", progress_ratio * 100.0),
            next_step: "store partial results via add_memory, report remaining pending_factors to user".into(),
        };
    }
    let pct = if max_iterations == 0 { 0.0 } else { iteration as f32 / max_iterations as f32 };
    if surprise_signal > 0.8 && pct > 0.5 && progress_ratio < 0.5 {
        return ExitDecision {
            action: ExitAction::Escalate,
            rationale: format!(
                "high surprise ({:.2}) with {:.0}% budget used and only {:.0}% progress — task may be unsolvable without more context",
                surprise_signal, pct * 100.0, progress_ratio * 100.0
            ),
            next_step: "call POST /memory/reflect, run omega loop, then ask user for clarification or reduced scope".into(),
        };
    }
    if progress_ratio < 0.01 && pct > 0.25 {
        return ExitDecision {
            action: ExitAction::Fail,
            rationale: format!("zero progress after {:.0}% of budget — agent is stuck", pct * 100.0),
            next_step: "call clarify_goal to reframe, or call recommend_tools to check missing tools, then restart".into(),
        };
    }
    ExitDecision {
        action: ExitAction::Continue,
        rationale: format!(
            "iteration {}/{} — {:.0}% factors satisfied",
            iteration, max_iterations, progress_ratio * 100.0
        ),
        next_step: "observe via get_live_beliefs → reflect → act → store observation → check_progress".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clarify_vague_task() {
        let g = clarify_goal("help me with something");
        assert!(!g.clarifying_questions.is_empty());
        assert!(g.uncertainty_flags.iter().any(|f| f.contains("vague")));
        assert_eq!(g.recommended_first_action, "ask_user_clarifying_questions");
    }

    #[test]
    fn test_clarify_dev_task() {
        let g = clarify_goal("Build a full-stack authentication system with JWT and refresh tokens");
        assert!(g.suggested_success_factors.iter().any(|f| f.contains("test")));
        assert!(g.suggested_max_iterations >= 30);
    }

    #[test]
    fn test_plan_validation_known_factors() {
        let plan = plan_validation(&["tests_passing", "auth_system_tested", "data_collected"]);
        assert_eq!(plan.steps.len(), 3);
        assert!(plan.steps[0].tool_or_command.contains("test"));
        assert!(plan.steps[2].tool_or_command.contains("memory"));
    }

    #[test]
    fn test_check_progress_satisfied() {
        let factors = ["auth_complete", "tests_passing"];
        let obs = ["auth complete for JWT", "tests passing in CI"];
        let p = check_progress(&factors, &obs, 5, 50);
        assert!(p.progress_ratio > 0.0);
        assert!(!p.uncertainty_detected);
    }

    #[test]
    fn test_check_progress_uncertainty() {
        let factors = ["auth_complete", "tests_passing", "deploy_ready", "review_done"];
        let obs = ["started working on auth"];
        let p = check_progress(&factors, &obs, 30, 50); // 60% budget, minimal progress
        assert!(p.uncertainty_detected);
        assert!(p.recommended_action.contains("UNCERTAINTY"));
    }

    #[test]
    fn test_should_exit_succeed() {
        let d = should_exit(10, 50, 1.0, 0.1);
        assert_eq!(d.action, ExitAction::Succeed);
    }

    #[test]
    fn test_should_exit_max_iterations() {
        let d = should_exit(50, 50, 0.6, 0.2);
        assert_eq!(d.action, ExitAction::Fail);
        assert!(d.rationale.contains("max_iterations"));
    }

    #[test]
    fn test_should_exit_escalate() {
        let d = should_exit(30, 50, 0.3, 0.9); // 60% budget, 30% progress, high surprise
        assert_eq!(d.action, ExitAction::Escalate);
    }

    #[test]
    fn test_should_exit_stuck() {
        let d = should_exit(15, 50, 0.0, 0.2); // 30% budget, 0 progress
        assert_eq!(d.action, ExitAction::Fail);
        assert!(d.rationale.contains("stuck"));
    }

    #[test]
    fn test_should_exit_continue() {
        let d = should_exit(5, 50, 0.4, 0.3);
        assert_eq!(d.action, ExitAction::Continue);
    }
}
