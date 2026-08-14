//! ExecutionGate trait — injectable seam between L0 decision logic and external L4 runtime.
//! Default implementation: DecisionEngine (in self_model).

use crate::self_model::{Decision, DecisionContext, ResourceUsage};

/// Gate that approves or rejects operations before execution.
/// Implement this trait to provide a custom execution policy from L4.
pub trait ExecutionGate: Send + Sync {
    fn evaluate(
        &mut self,
        operation: &str,
        context: &DecisionContext,
        success_rate: f64,
        resources: &ResourceUsage,
        health_score: f64,
    ) -> Decision;

    fn record_outcome(&mut self, operation: &str, approved: bool);
    fn min_utility(&self) -> f64;
}
