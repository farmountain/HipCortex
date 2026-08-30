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
