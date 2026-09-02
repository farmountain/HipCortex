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
