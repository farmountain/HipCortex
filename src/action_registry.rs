//! ActionRegistry — enumerate all known op types and filter to currently authorized ones.
//!
//! call list_authorized(self_model) to get ops approved by DecisionEngine.

use crate::self_model::{DecisionContext, SelfModel};
use serde::{Deserialize, Serialize};

/// Known operation names in HipCortex.
pub const ALL_OPS: &[&str] = &[
    "store_memory",
    "query_memory",
    "react_loop",
    "world_model_rollout",
    "intervene",
    "counterfactual",
    "credit_assign",
    "archive_record",
    "memory_diff",
    "mgv_check",
    "cognitive_report",
    "provenance_query",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizedOp {
    pub op: String,
    pub confidence: f64,
    pub rationale: String,
}

/// Return ops filtered by current goal, workspace, and health context — no SelfModel required.
pub fn list_authorized_contextual(
    goal_id: Option<uuid::Uuid>,
    _actor: &str,
    has_active_workspace: bool,
    health_score: f32,
) -> Vec<String> {
    ALL_OPS
        .iter()
        .filter(|&&op| {
            if health_score < 0.4 && matches!(op, "archive_record" | "memory_diff") {
                return false;
            }
            if !has_active_workspace && op == "cognitive_report" {
                // workspace_merge only valid when workspace open; keep others
            }
            if goal_id.is_none() && matches!(op, "react_loop" | "credit_assign" | "world_model_rollout") {
                return false;
            }
            true
        })
        .map(|s| s.to_string())
        .collect()
}

/// A declared constraint on a world-model operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpConstraint {
    pub op: &'static str,
    pub requires_wm: bool,
    pub max_depth: usize,
    pub max_iterations: usize,
}

/// Declared world-model op constraints (Phase 3c). Caps match REST endpoint validation.
pub const WM_CONSTRAINTS: &[OpConstraint] = &[
    OpConstraint { op: "world_model_rollout",  requires_wm: true, max_depth: 10, max_iterations: 200 },
    OpConstraint { op: "counterfactual",       requires_wm: true, max_depth:  5, max_iterations:  50 },
    OpConstraint { op: "intervene",            requires_wm: true, max_depth:  3, max_iterations:  10 },
];

/// Return world-model ops that pass both `WM_CONSTRAINTS` and the SelfModel's DecisionEngine.
pub fn list_authorized_world_model(self_model: &SelfModel) -> Vec<AuthorizedOp> {
    WM_CONSTRAINTS
        .iter()
        .filter_map(|c| {
            let decision = self_model
                .can_execute(c.op, crate::self_model::DecisionContext::default_context())
                .ok()?;
            if decision.should_execute {
                Some(AuthorizedOp {
                    op: c.op.to_string(),
                    confidence: decision.confidence,
                    rationale: format!(
                        "{} (depth≤{} iter≤{})",
                        decision.rationale, c.max_depth, c.max_iterations
                    ),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Return all ops currently approved by the SelfModel's DecisionEngine.
pub fn list_authorized(self_model: &SelfModel) -> Vec<AuthorizedOp> {
    ALL_OPS
        .iter()
        .filter_map(|&op| {
            let decision = self_model
                .can_execute(op, DecisionContext::default_context())
                .ok()?;
            if decision.should_execute {
                Some(AuthorizedOp {
                    op: op.to_string(),
                    confidence: decision.confidence,
                    rationale: decision.rationale,
                })
            } else {
                None
            }
        })
        .collect()
}
