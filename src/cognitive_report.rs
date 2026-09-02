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

    // Q2 — learned beliefs (conf > 0.3)
    let all_beliefs: Vec<BeliefSummary> = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
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

    let learned_beliefs: Vec<BeliefSummary> =
        all_beliefs.iter().filter(|b| b.confidence > 0.3).cloned().collect();

    // Q3 — valid assumptions (conf >= 0.5)
    let valid_assumptions: Vec<BeliefSummary> =
        all_beliefs.iter().filter(|b| b.confidence >= 0.5).cloned().collect();

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

    // Q6 — failures (last 10)
    let recent_failures: Vec<FailureSummary> = store
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
        .take(10)
        .collect::<Vec<_>>();

    // Q7 — emergent abstractions (Beliefs written by EmergenceDetector)
    let emergent_abstractions: Vec<BeliefSummary> = store
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

    // Q8 — uncertainties
    let uncertain_beliefs: Vec<BeliefSummary> =
        all_beliefs.iter().filter(|b| b.confidence < 0.6).cloned().collect();
    let invalidated_count = store
        .find_by_action("belief_invalidated")
        .len();
    let open_uncertainties = UncertaintySummary { uncertain_beliefs, invalidated_count };

    // Q9 — authorized actions filtered by current goal + health context
    let active_goal_id = active_goals.first().map(|g| g.id);
    let authorized_actions =
        crate::action_registry::list_authorized_contextual(active_goal_id, actor, false, 1.0);

    // Q10 — next recommendation
    let next_goal = crate::goal_scheduler::GoalScheduler::next(store, actor);
    let next_recommendation = if let Some(gid) = next_goal {
        let target = store
            .find_by_id(gid)
            .and_then(|r| serde_json::from_value::<GoalPayload>(r.metadata.clone()).ok())
            .map(|p| p.target_state)
            .unwrap_or_default();
        NextAction {
            goal_id: Some(gid),
            goal_target: Some(target),
            recommended_op: "react_loop".to_string(),
            rationale: "highest urgency/cost ratio among active goals".to_string(),
        }
    } else {
        NextAction {
            goal_id: None,
            goal_target: None,
            recommended_op: "query_memory".to_string(),
            rationale: "no active goals — review state".to_string(),
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
