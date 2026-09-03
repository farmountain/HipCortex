//! ClarifyEngine — self-prompting clarity loop for goal problem statements.
//!
//! Chain-of-thought:
//!   Triggered when: (a) goal has empty success_factors, (b) consecutive vetoes ≥ 3,
//!   or (c) pre-success check needed. Searches beliefs and the world model up to
//!   MAX_CLARIFY_ROUNDS times. If self-resolution found, writes Reflexion{self_clarified}
//!   and returns ClarifiedBySubstrate. If unresolved after MAX_ROUNDS, writes a single
//!   deduped Belief{clarify_needed} and returns NeedsUserClarification. Exit is guaranteed
//!   (no indefinite loop).

use uuid::Uuid;

const MAX_CLARIFY_ROUNDS: u32 = 3;

#[derive(Debug, Clone)]
pub enum ClarifyTrigger {
    EmptyAC,
    RepeatedVeto { veto_count: u32 },
    PreSuccess,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClarifyOutcome {
    ClarifiedBySubstrate,
    AlreadyClear,
    NeedsUserClarification,
}

pub struct ClarifyEngine;

impl ClarifyEngine {
    /// Run self-prompting clarification for `goal_id`.
    ///
    /// Searches existing beliefs for propositions that mention the goal's target
    /// state; if found, writes a Reflexion{self_clarified} and returns
    /// ClarifiedBySubstrate. Otherwise writes a single deduped Belief{clarify_needed}
    /// and returns NeedsUserClarification. MAX_CLARIFY_ROUNDS bounds iteration.
    pub fn run<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        trigger: ClarifyTrigger,
    ) -> ClarifyOutcome {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::{BeliefPayload, GoalPayload};

        // Read goal to get target_state for search.
        let Some(goal_rec) = store.find_by_id(goal_id) else {
            return ClarifyOutcome::NeedsUserClarification;
        };
        let goal: GoalPayload = match serde_json::from_value(goal_rec.metadata.clone()) {
            Ok(p) => p,
            Err(_) => return ClarifyOutcome::NeedsUserClarification,
        };

        // If goal already has non-empty AC, no clarification needed.
        if !goal.success_factors.is_empty() && matches!(trigger, ClarifyTrigger::EmptyAC) {
            return ClarifyOutcome::AlreadyClear;
        }

        // Deduplicate: skip if clarify_needed belief already exists for this goal.
        let already_flagged = store
            .all_by_type(MemoryType::Belief)
            .into_iter()
            .any(|r| {
                r.actor == actor
                    && r.action == "clarify_needed"
                    && r.derived_from == Some(goal_id)
            });
        if already_flagged {
            return ClarifyOutcome::NeedsUserClarification;
        }

        let target = &goal.target_state;

        // Self-prompting: search beliefs up to MAX_CLARIFY_ROUNDS for resolution cues.
        let mut round = 0u32;
        while round < MAX_CLARIFY_ROUNDS {
            round += 1;
            let beliefs = store.all_by_type(MemoryType::Belief);
            let resolution = beliefs.into_iter().find(|r| {
                if let Ok(bp) = serde_json::from_value::<BeliefPayload>(r.metadata.clone()) {
                    bp.proposition.contains(target.as_str())
                        || bp.proposition.contains("clarify")
                } else {
                    false
                }
            });

            if let Some(belief_rec) = resolution {
                // Found substrate knowledge — write Reflexion{self_clarified}.
                let note = format!(
                    "trigger={:?} round={} resolved_by=belief:{}",
                    trigger, round, belief_rec.id
                );
                let rec = MemoryRecord::new(
                    MemoryType::Reflexion,
                    actor.to_string(),
                    "self_clarified".to_string(),
                    goal_id.to_string(),
                    serde_json::json!({ "note": note }),
                );
                let mut r = rec;
                r.derived_from = Some(goal_id);
                let _ = store.add(r);
                return ClarifyOutcome::ClarifiedBySubstrate;
            }
        }

        // Unresolved after MAX_CLARIFY_ROUNDS — write single Belief{clarify_needed}.
        let bp = crate::payloads::BeliefPayload {
            proposition: format!("goal {} requires user clarification (trigger={:?})", goal_id, trigger),
            confidence: 0.1,
            ..Default::default()
        };
        if let Ok(meta) = serde_json::to_value(&bp) {
            let mut rec = MemoryRecord::new(
                MemoryType::Belief,
                actor.to_string(),
                "clarify_needed".to_string(),
                goal_id.to_string(),
                meta,
            );
            rec.derived_from = Some(goal_id);
            let _ = store.add(rec);
        }
        ClarifyOutcome::NeedsUserClarification
    }
}
