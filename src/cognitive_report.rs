//! CognitiveStateReport — single-call aggregator answering all 10 cognitive questions.
//!
//! GET /v1/cognitive/report → build_report(store, actor)

use crate::memory_record::MemoryType;
use crate::payloads::{BeliefPayload, DecisionPayload, GoalPayload, GoalStatus};
use crate::persistence::MemoryBackend;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSummary {
    pub id: Uuid,
    pub target_state: String,
    pub status: String,
    pub urgency: f64,
    pub current_iteration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefSummary {
    pub id: Uuid,
    pub proposition: String,
    pub confidence: f32,
    pub epistemic_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSummary {
    pub id: Uuid,
    pub option_chosen: String,
    pub confidence: f64,
    pub goal_id: Option<Uuid>,
    pub rationale_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureSummary {
    pub id: Uuid,
    pub target_state: String,
    pub iterations_run: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintySummary {
    /// Beliefs with confidence < 0.6 (open questions)
    pub uncertain_beliefs: Vec<BeliefSummary>,
    /// Count of Beliefs recently invalidated
    pub invalidated_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    pub goal_id: Option<Uuid>,
    pub goal_target: Option<String>,
    pub recommended_op: String,
    pub rationale: String,
}

/// Answers all 10 cognitive questions in one struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveStateReport {
    pub actor: String,
    // Q1: What is the goal?
    pub active_goals: Vec<GoalSummary>,
    // Q2: What have we learned?
    pub learned_beliefs: Vec<BeliefSummary>,
    // Q3: What assumptions are still valid?
    pub valid_assumptions: Vec<BeliefSummary>,
    // Q4 + Q5: What decisions have been made and why?
    pub recent_decisions: Vec<DecisionSummary>,
    // Q6: What failed?
    pub recent_failures: Vec<FailureSummary>,
    // Q7: What abstractions have emerged?
    pub emergent_abstractions: Vec<BeliefSummary>,
    // Q8: What remains uncertain?
    pub open_uncertainties: UncertaintySummary,
    // Q9: What actions are currently authorized?
    pub authorized_actions: Vec<String>,
    // Q10: What should happen next?
    pub next_recommendation: NextAction,
}

pub fn build_report<B: MemoryBackend>(
    store: &crate::memory_store::MemoryStore<B>,
    actor: &str,
    health: f32,
) -> CognitiveStateReport {
    // Q1 — active goals
    let active_goals: Vec<GoalSummary> = store
        .all_by_type(MemoryType::Goal)
        .into_iter()
        .filter(|r| r.actor == actor)
        .filter_map(|r| {
            let p: GoalPayload = serde_json::from_value(r.metadata.clone()).ok()?;
            if p.status != GoalStatus::Pending && p.status != GoalStatus::InProgress {
                return None;
            }
            Some(GoalSummary {
                id: r.id,
                target_state: p.target_state,
                status: format!("{:?}", p.status),
                urgency: p.urgency,
                current_iteration: p.current_iteration,
            })
        })
        .collect();

    // Q2 — collect raw beliefs retaining JTMS label for Q3 filter (P0-E).
    let all_belief_pairs: Vec<(crate::payloads::JtmsLabel, BeliefSummary)> = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter_map(|r| {
            let p: BeliefPayload = serde_json::from_value(r.metadata.clone()).ok()?;
            let label = p.jtms_label.clone();
            Some((label, BeliefSummary {
                id: r.id,
                proposition: p.proposition,
                confidence: p.confidence,
                epistemic_status: format!("{:?}", p.epistemic_status),
            }))
        })
        .collect();

    let all_beliefs: Vec<BeliefSummary> =
        all_belief_pairs.iter().map(|(_, b)| b.clone()).collect();
    // Q2: JTMS-gated — only In-labelled beliefs above 0.3 qualify as "learned".
    // Out beliefs excluded even at high confidence (split state prevention).
    let learned_beliefs: Vec<BeliefSummary> =
        all_belief_pairs
            .iter()
            .filter(|(label, b)| matches!(label, crate::payloads::JtmsLabel::In) && b.confidence > 0.3)
            .map(|(_, b)| b.clone())
            .collect();

    // Q3 — valid assumptions: JTMS In authoritative; Unknown+0.5 included but marked Provisional.
    // Exclude beliefs whose contact_kind == PredictedOnly (Kalman fill-in ≠ valid assumption).
    let predicted_only_ids: std::collections::HashSet<uuid::Uuid> = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter_map(|r| {
            let p: BeliefPayload = serde_json::from_value(r.metadata.clone()).ok()?;
            if matches!(p.contact_kind, Some(crate::action_intent::ContactKind::PredictedOnly)) {
                Some(r.id)
            } else {
                None
            }
        })
        .collect();
    let valid_assumptions: Vec<BeliefSummary> = all_belief_pairs
        .iter()
        .filter(|(label, b)| {
            !predicted_only_ids.contains(&b.id)
                && (matches!(label, crate::payloads::JtmsLabel::In)
                    || (matches!(label, crate::payloads::JtmsLabel::Unknown) && b.confidence >= 0.5))
        })
        .map(|(label, b)| {
            if matches!(label, crate::payloads::JtmsLabel::Unknown) {
                BeliefSummary {
                    epistemic_status: format!("Provisional({})", b.epistemic_status),
                    ..b.clone()
                }
            } else {
                b.clone()
            }
        })
        .collect();

    // Q4+Q5 — recent decisions (last 10)
    let recent_decisions: Vec<DecisionSummary> = store
        .all_by_type(MemoryType::Decision)
        .into_iter()
        .rev()
        .take(10)
        .filter_map(|r| {
            let p: DecisionPayload = serde_json::from_value(r.metadata.clone()).ok()?;
            Some(DecisionSummary {
                id: r.id,
                option_chosen: p.option_chosen,
                confidence: p.confidence,
                goal_id: r.derived_from,
                rationale_chain: p.rationale_chain,
            })
        })
        .collect();

    // Q6 — failures: failed goals + CreditAssign reflexions (broken structural equations).
    let mut recent_failures: Vec<FailureSummary> = store
        .all_by_type(MemoryType::Goal)
        .into_iter()
        .filter(|r| r.actor == actor)
        .filter_map(|r| {
            let p: GoalPayload = serde_json::from_value(r.metadata.clone()).ok()?;
            if p.status != GoalStatus::Failed {
                return None;
            }
            Some(FailureSummary {
                id: r.id,
                target_state: p.target_state,
                iterations_run: p.current_iteration,
            })
        })
        .rev()
        .take(7)
        .collect::<Vec<_>>();
    // Add CreditAssign Reflexion records as structural failure signals.
    let credit_failures: Vec<FailureSummary> = store
        .all_by_type(MemoryType::Reflexion)
        .into_iter()
        .filter(|r| r.actor == actor && r.action == "credit_assign")
        .rev()
        .take(3)
        .map(|r| FailureSummary {
            id: r.id,
            target_state: format!("credit_assign:{}", r.target),
            iterations_run: 0,
        })
        .collect();
    recent_failures.extend(credit_failures);
    recent_failures.truncate(10);

    // Q7 — emergent abstractions: emerge-sensor beliefs + Skill records + high-conf derived beliefs.
    let mut emergent_abstractions: Vec<BeliefSummary> = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter(|r| r.action == "emerge")
        .filter_map(|r| {
            let p: BeliefPayload = serde_json::from_value(r.metadata.clone()).ok()?;
            Some(BeliefSummary {
                id: r.id,
                proposition: p.proposition,
                confidence: p.confidence,
                epistemic_status: format!("{:?}", p.epistemic_status),
            })
        })
        .collect();
    // Skill records are reusable procedure abstractions learned from causal motifs.
    let skill_abstractions: Vec<BeliefSummary> = store
        .all_by_type(MemoryType::Skill)
        .into_iter()
        .filter_map(|r| {
            let p: crate::payloads::SkillPayload = serde_json::from_value(r.metadata.clone()).ok()?;
            Some(BeliefSummary {
                id: r.id,
                proposition: format!("skill:{}", p.procedure),
                confidence: r.confidence,
                epistemic_status: "Skill".to_string(),
            })
        })
        .collect();
    emergent_abstractions.extend(skill_abstractions);
    // High-confidence beliefs with derived_from set are law-like derived abstractions.
    let derived_abstractions: Vec<BeliefSummary> = all_belief_pairs
        .iter()
        .filter(|(_, b)| b.confidence >= 0.7)
        .filter(|(_, b)| store.find_by_id(b.id).map(|r| r.derived_from.is_some()).unwrap_or(false))
        .map(|(_, b)| b.clone())
        .collect();
    emergent_abstractions.extend(derived_abstractions);

    // Q8 — uncertainties: Unknown-labelled beliefs are uncertain regardless of confidence;
    // low-confidence In/Out beliefs also qualify.
    let uncertain_beliefs: Vec<BeliefSummary> =
        all_belief_pairs
            .iter()
            .filter(|(label, b)| matches!(label, crate::payloads::JtmsLabel::Unknown) || b.confidence < 0.6)
            .map(|(_, b)| b.clone())
            .collect();
    let invalidated_count = store
        .find_by_action("belief_invalidated")
        .len();
    // Expired intents = host silence — first-class uncertainty holes.
    let expired_intent_count = store
        .all_by_type(MemoryType::Intent)
        .iter()
        .filter(|r| {
            r.metadata.get("status").and_then(|s| s.as_str()) == Some("Expired")
        })
        .count();
    let open_uncertainties = UncertaintySummary {
        uncertain_beliefs,
        invalidated_count: invalidated_count + expired_intent_count,
    };

    // Q9 — authorized actions filtered by current goal + real health from SelfModel.
    let active_goal_id = active_goals.first().map(|g| g.id);
    let authorized_actions =
        crate::action_registry::list_authorized_contextual(active_goal_id, actor, false, health);

    // Q10 — next recommendation wired to SynthesisMode + ClarifyEngine status.
    let synthesis_mode = if health < 0.3 {
        "Escalate"
    } else if health > 0.8 {
        "Autonomous"
    } else {
        "Balanced"
    };
    // Check if ClarifyEngine has a pending clarify_needed for the active goal.
    let clarify_pending = active_goal_id
        .map(|gid| {
            store.all_by_type(MemoryType::Belief).into_iter().any(|r| {
                r.actor == actor && r.action == "clarify_needed" && r.derived_from == Some(gid)
            })
        })
        .unwrap_or(false);
    let next_goal = crate::goal_scheduler::GoalScheduler::next(store, actor);
    // Q10 — check grounding before react_loop: probe-first if open intents are pending.
    let open_intent_recs = store.all_by_type(MemoryType::Intent);
    let has_open_intents = open_intent_recs.iter().any(|r| {
        matches!(
            r.metadata.get("status").and_then(|s| s.as_str()),
            Some("Open") | Some("InFlight")
        )
    });
    let has_expired_intents = open_intent_recs.iter().any(|r| {
        r.metadata.get("status").and_then(|s| s.as_str()) == Some("Expired")
    });
    let top_probe_entity: Option<String> = open_intent_recs.iter()
        .filter(|r| matches!(r.metadata.get("status").and_then(|s| s.as_str()), Some("Open") | Some("InFlight")))
        .find_map(|r| r.metadata.get("target_entity").and_then(|v| v.as_str()).map(|s| s.to_string()));

    let next_recommendation = if clarify_pending {
        NextAction {
            goal_id: active_goal_id,
            goal_target: active_goals.first().map(|g| g.target_state.clone()),
            recommended_op: "clarify_goal".to_string(),
            rationale: format!("ClarifyEngine: goal requires clarification (mode={synthesis_mode})"),
        }
    } else if synthesis_mode == "Escalate" {
        NextAction {
            goal_id: active_goal_id,
            goal_target: active_goals.first().map(|g| g.target_state.clone()),
            recommended_op: "escalate_to_user".to_string(),
            rationale: format!("health={health:.2} — system health critical, escalate before proceeding"),
        }
    } else if has_open_intents {
        NextAction {
            goal_id: active_goal_id,
            goal_target: active_goals.first().map(|g| g.target_state.clone()),
            recommended_op: top_probe_entity
                .map(|e| format!("probe_entity:{e}"))
                .unwrap_or_else(|| "ground_workspace".to_string()),
            rationale: format!("mode={synthesis_mode} — open probe intents pending, await host receipts before planning"),
        }
    } else if has_expired_intents {
        NextAction {
            goal_id: active_goal_id,
            goal_target: active_goals.first().map(|g| g.target_state.clone()),
            recommended_op: "escalate_to_user".to_string(),
            rationale: format!("mode={synthesis_mode} — host silence (expired intents), human is last sensor"),
        }
    } else if let Some(gid) = next_goal {
        let target = store
            .find_by_id(gid)
            .and_then(|r| serde_json::from_value::<GoalPayload>(r.metadata.clone()).ok())
            .map(|p| p.target_state)
            .unwrap_or_default();
        NextAction {
            goal_id: Some(gid),
            goal_target: Some(target),
            recommended_op: "react_loop".to_string(),
            rationale: format!("mode={synthesis_mode} — highest urgency/cost ratio among active goals"),
        }
    } else {
        NextAction {
            goal_id: None,
            goal_target: None,
            recommended_op: "query_memory".to_string(),
            rationale: format!("mode={synthesis_mode} — no active goals, review state"),
        }
    };

    CognitiveStateReport {
        actor: actor.to_string(),
        active_goals,
        learned_beliefs,
        valid_assumptions,
        recent_decisions,
        recent_failures,
        emergent_abstractions,
        open_uncertainties,
        authorized_actions,
        next_recommendation,
    }
}
