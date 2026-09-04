//! Phase-2: Executive gates for the ReAct loop and cognitive daemon.
//!
//! Chain-of-thought:
//!   CriticGate — evaluates goal progress before the ACT phase.
//!     iteration 0 always passes (bootstrapping grace).
//!     iterations > 0: if fraction of satisfied success_factors < VETO_THRESHOLD → Rejected.
//!     Rejected → caller writes Decision{action="rejected"} and skips that iteration's act.
//!
//!   VerifierGate — checks predicted vs observed state before committing a Temporal record.
//!     If WM predicts a next state and it disagrees with the observation → Mismatch.
//!     Mismatch → caller writes Belief{action="verifier_mismatch"}, skips Temporal write.

use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::payloads::GoalPayload;
use crate::persistence::MemoryBackend;
use uuid::Uuid;

pub const CRITIC_VETO_THRESHOLD: f32 = 0.25;

// ─── CriticGate ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum CriticDecision {
    Approved { confidence: f32 },
    Rejected { rationale: Vec<String> },
}

pub struct CriticGate;

impl CriticGate {
    /// Evaluate with an explicit veto threshold (from `SelfModel::recommend_loop_config`).
    /// Iteration 0 always passes regardless of threshold.
    pub fn evaluate_with_threshold(
        goal: &GoalPayload,
        proposed_action: &str,
        iteration: u32,
        veto_threshold: f32,
    ) -> CriticDecision {
        if iteration == 0 {
            return CriticDecision::Approved { confidence: 1.0 };
        }
        let total = goal.success_factors.len().max(1);
        let satisfied = goal.success_factors.iter().filter(|f| f.satisfied).count();
        let fraction = satisfied as f32 / total as f32;
        if fraction < veto_threshold {
            CriticDecision::Rejected {
                rationale: vec![
                    format!(
                        "{}/{} success_factors satisfied — below veto threshold {:.2}",
                        satisfied, total, veto_threshold
                    ),
                    format!(
                        "action '{}' vetoed at iteration {} — no meaningful progress",
                        proposed_action, iteration
                    ),
                ],
            }
        } else {
            CriticDecision::Approved { confidence: fraction }
        }
    }

    /// Evaluate with the default static threshold. Backward-compatible wrapper.
    pub fn evaluate(goal: &GoalPayload, proposed_action: &str, iteration: u32) -> CriticDecision {
        Self::evaluate_with_threshold(goal, proposed_action, iteration, CRITIC_VETO_THRESHOLD)
    }
}

// ─── VerifierGate ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum VerifierResult {
    Consistent,
    Mismatch { predicted: String, observed: String },
}

pub struct VerifierGate;

impl VerifierGate {
    /// Compare a WM prediction against what was actually observed.
    /// `predicted` — most likely next state from the world model; `None` means no prediction.
    /// A `None` predicted always returns Consistent (first iteration or no WM data).
    pub fn check(predicted: Option<&str>, observed: &str) -> VerifierResult {
        match predicted {
            Some(p) if p != observed => VerifierResult::Mismatch {
                predicted: p.to_string(),
                observed: observed.to_string(),
            },
            _ => VerifierResult::Consistent,
        }
    }

    /// Atomic variant: writes Temporal{verifier_mismatch_observed} on mismatch so
    /// cognitive_report Q2 (recent Temporal observations) can see the discrepancy.
    pub fn check_and_record<B: MemoryBackend>(
        store: &mut MemoryStore<B>,
        predicted: Option<&str>,
        observed: &str,
        actor: &str,
        goal_id: Option<Uuid>,
    ) -> VerifierResult {
        let result = Self::check(predicted, observed);
        if let VerifierResult::Mismatch { ref predicted, .. } = result {
            let mut temporal = MemoryRecord::new(
                MemoryType::Temporal,
                actor.to_string(),
                "verifier_mismatch_observed".to_string(),
                observed.to_string(),
                serde_json::json!({ "predicted": predicted }),
            );
            temporal.derived_from = goal_id;
            let _ = store.add(temporal);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    fn goal_with(satisfied: usize, total: usize) -> GoalPayload {
        GoalPayload {
            target_state: "t".into(),
            status: GoalStatus::InProgress,
            success_factors: (0..total)
                .map(|i| SuccessFactor {
                    name: format!("f{i}"),
                    weight: 1.0,
                    satisfied: i < satisfied,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn critic_iter0_always_approves() {
        let goal = goal_with(0, 4);
        assert!(matches!(CriticGate::evaluate(&goal, "act", 0), CriticDecision::Approved { .. }));
    }

    #[test]
    fn critic_below_threshold_rejects() {
        let goal = goal_with(0, 4); // 0/4 = 0.0 < 0.25
        assert!(matches!(CriticGate::evaluate(&goal, "act", 1), CriticDecision::Rejected { .. }));
    }

    #[test]
    fn critic_at_or_above_threshold_approves() {
        let goal = goal_with(1, 4); // 1/4 = 0.25 == threshold → approve
        assert!(matches!(CriticGate::evaluate(&goal, "act", 1), CriticDecision::Approved { .. }));
    }

    #[test]
    fn verifier_none_prediction_consistent() {
        assert!(matches!(VerifierGate::check(None, "state-A"), VerifierResult::Consistent));
    }

    #[test]
    fn verifier_matching_consistent() {
        assert!(matches!(VerifierGate::check(Some("state-A"), "state-A"), VerifierResult::Consistent));
    }

    #[test]
    fn verifier_mismatch_detected() {
        assert!(matches!(
            VerifierGate::check(Some("state-A"), "state-B"),
            VerifierResult::Mismatch { .. }
        ));
    }
}
