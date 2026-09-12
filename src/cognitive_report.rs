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
    /// True when every field was filtered to `actor`.
    ///
    /// A `false` value means the caller did not name an actor, so the report is
    /// deliberately **empty** rather than a cross-actor aggregate. Silently
    /// falling back to a magic actor would look like a working report while
    /// answering a different question than the caller asked — the failure mode
    /// H4/D9 calls out as *"worse than empty: it looks like it works"*. Clients
    /// must check this flag before trusting the body.
    pub actor_scoped: bool,
    // Q1: What is the goal?
    pub active_goals: Vec<GoalSummary>,
    // Q2: What have we learned?
    //
    // H6/WP8: this is a **count**, and the beliefs themselves live in
    // `learned_beliefs_detail`. The report answers "what have we learned?" on
    // behalf of an autonomous loop, and a loop that must decide whether it has
    // learned enough wants the cardinality, not the payload. Returning the
    // array under the question's own key made the cost of asking Q2 grow with
    // how much the system had learned, so the one question whose answer is a
    // single integer was the most expensive one to ask.
    //
    // Note on the compatibility clause: the spec's WP8 row says the arrays are
    // "retained under explicit `*_detail` keys" and its acceptance row says
    // "existing consumers unaffected". On a JSON wire those two cannot both
    // hold for the *same* key — a key is either a number or an array. The
    // resolution taken here is the first clause (explicit naming), because it
    // is the specific instruction, and "unaffected" is honoured by keeping
    // every element reachable at a predictable key rather than by freezing the
    // shape. All in-repo consumers were updated in the same change; AC-Q2 and
    // AC-R4 still assert on the same sets, only via `*_detail`.
    pub learned_beliefs: usize,
    /// The beliefs counted by `learned_beliefs` (Q2), in full.
    pub learned_beliefs_detail: Vec<BeliefSummary>,
    // Q3: What assumptions are still valid?
    pub valid_assumptions: Vec<BeliefSummary>,
    // Q4 + Q5: What decisions have been made and why?
    pub recent_decisions: Vec<DecisionSummary>,
    // Q6: What failed?
    pub recent_failures: Vec<FailureSummary>,
    // Q7: What abstractions have emerged?
    //
    // H6/WP8: count, with the array under `emergent_abstractions_detail`. Same
    // reasoning as Q2 — Q7 is asked every loop iteration, and its answer is a
    // cardinality ("have abstractions formed yet?"), not a catalogue.
    pub emergent_abstractions: usize,
    /// The abstractions counted by `emergent_abstractions` (Q7), in full.
    pub emergent_abstractions_detail: Vec<BeliefSummary>,
    // Q8: What remains uncertain?
    pub open_uncertainties: UncertaintySummary,
    // Q9: What actions are currently authorized?
    pub authorized_actions: Vec<String>,
    // Q10: What should happen next?
    pub next_recommendation: NextAction,
    /// WP10/§3.5 — why the substrate did or did not ask the user to clarify.
    ///
    /// The clarification ladder (T0 environment → T1 prior art → T2 causal → T3 ask)
    /// is already *recorded* in the ledger, one `Reflexion` per rung, and therefore
    /// already visible on `GET /goal/:id/trace`. That is enough for a forensic
    /// reader but not for a human skimming the report, who sees only
    /// `next_recommendation.recommended_op == "clarify_goal"` with no way to tell
    /// whether the ladder was tried or skipped. This field is that missing "why",
    /// flattened to one entry per rung: `{tier, outcome, evidence}`.
    ///
    /// Empty when there is no active goal, or when the goal never entered the
    /// ladder — an empty ladder is a statement ("nothing was ambiguous enough to
    /// trigger it"), not an error.
    pub clarify_ladder: Vec<crate::clarify_engine::LadderRung>,
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
    // Actor-scoped: questions are asked on behalf of one actor, so another actor's
    // beliefs must never be reported as this actor's knowledge.
    let all_belief_pairs: Vec<(crate::payloads::JtmsLabel, BeliefSummary)> = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter(|r| r.actor == actor)
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

    // Q2: JTMS-gated — only In-labelled beliefs above 0.3 qualify as "learned".
    // Out beliefs excluded even at high confidence (split state prevention).
    let learned_beliefs_detail: Vec<BeliefSummary> =
        all_belief_pairs
            .iter()
            .filter(|(label, b)| matches!(label, crate::payloads::JtmsLabel::In) && b.confidence > 0.3)
            .map(|(_, b)| b.clone())
            .collect();
    // H6/WP8: the count is derived from the detail vector, never maintained
    // alongside it, so the two can never disagree.
    let learned_beliefs = learned_beliefs_detail.len();

    // Q3 — valid assumptions: JTMS In authoritative; Unknown+0.5 included but marked Provisional.
    // Exclude beliefs whose contact_kind == PredictedOnly (Kalman fill-in ≠ valid assumption).
    let predicted_only_ids: std::collections::HashSet<uuid::Uuid> = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter(|r| r.actor == actor)
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

    // Q4+Q5 — recent decisions (last 10), actor-scoped
    let recent_decisions: Vec<DecisionSummary> = store
        .all_by_type(MemoryType::Decision)
        .into_iter()
        .filter(|r| r.actor == actor)
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
    let mut emergent_abstractions_detail: Vec<BeliefSummary> = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter(|r| r.actor == actor && r.action == "emerge")
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
        .filter(|r| r.actor == actor)
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
    emergent_abstractions_detail.extend(skill_abstractions);
    // High-confidence beliefs with derived_from set are law-like derived abstractions.
    let derived_abstractions: Vec<BeliefSummary> = all_belief_pairs
        .iter()
        .filter(|(_, b)| b.confidence >= 0.7)
        .filter(|(_, b)| store.find_by_id(b.id).map(|r| r.derived_from.is_some()).unwrap_or(false))
        .map(|(_, b)| b.clone())
        .collect();
    emergent_abstractions_detail.extend(derived_abstractions);
    // H6/WP8: derived from the detail vector — the count cannot drift from it.
    let emergent_abstractions = emergent_abstractions_detail.len();

    // Q8 — uncertainties: Unknown-labelled beliefs are uncertain regardless of confidence;
    // low-confidence In/Out beliefs also qualify.
    let uncertain_beliefs: Vec<BeliefSummary> =
        all_belief_pairs
            .iter()
            .filter(|(label, b)| matches!(label, crate::payloads::JtmsLabel::Unknown) || b.confidence < 0.6)
            .map(|(_, b)| b.clone())
            .collect();
    // Actor-scoped: another actor's invalidated beliefs are not this actor's uncertainty.
    let invalidated_count = store
        .find_by_actor(actor)
        .iter()
        .filter(|r| r.action == "belief_invalidated")
        .count();
    // Expired intents = host silence — first-class uncertainty holes.
    // Also count Open/InFlight intents whose deadline_ms is already past (C6: runner silence).
    let now_ms = chrono::Utc::now().timestamp_millis();
    let expired_intent_count = store
        .all_by_type(MemoryType::Intent)
        .iter()
        .filter(|r| r.actor == actor)
        .filter(|r| {
            let status = r.metadata.get("status").and_then(|s| s.as_str());
            status == Some("Expired") || {
                matches!(status, Some("Open") | Some("InFlight"))
                    && r.metadata.get("deadline_ms")
                        .and_then(|v| v.as_i64())
                        .map(|d| d < now_ms)
                        .unwrap_or(false)
            }
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
    // clarify_pending: explicit Belief{clarify_needed} OR goal with empty success_factors (C2).
    let clarify_pending = active_goal_id
        .map(|gid| {
            let has_belief = store.all_by_type(MemoryType::Belief).into_iter().any(|r| {
                r.actor == actor && r.action == "clarify_needed" && r.derived_from == Some(gid)
            });
            let empty_factors = store
                .find_by_id(gid)
                .and_then(|r| serde_json::from_value::<GoalPayload>(r.metadata.clone()).ok())
                .map(|p| p.success_factors.is_empty())
                .unwrap_or(false);
            has_belief || empty_factors
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
    } else if store
        .all_by_type(MemoryType::Goal)
        .into_iter()
        .filter(|r| r.actor == actor)
        .any(|r| {
            serde_json::from_value::<GoalPayload>(r.metadata.clone())
                .map(|p| matches!(p.status, GoalStatus::Succeeded))
                .unwrap_or(false)
        })
    {
        // C7: recently-succeeded goal — honest task_complete recommendation (not query_memory).
        NextAction {
            goal_id: None,
            goal_target: None,
            recommended_op: "task_complete".to_string(),
            rationale: format!("mode={synthesis_mode} — all success_factors satisfied, goal succeeded"),
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
        actor_scoped: true,
        active_goals,
        learned_beliefs,
        learned_beliefs_detail,
        valid_assumptions,
        recent_decisions,
        recent_failures,
        emergent_abstractions,
        emergent_abstractions_detail,
        open_uncertainties,
        authorized_actions,
        next_recommendation,
        // WP10/§3.5: the ladder for the goal Q10 is actually recommending action on.
        clarify_ladder: active_goal_id
            .map(|gid| crate::clarify_engine::ClarifyEngine::ladder_rungs(store, gid))
            .unwrap_or_default(),
    }
}

/// The report returned when the caller did not name an actor.
///
/// Every question is answered with *nothing* and `actor_scoped` is `false`, so
/// the response cannot be mistaken for a global view. H4/D9: the alternative —
/// picking a default actor — produces an answer that looks correct and is not,
/// which is exactly the bug this type exists to prevent.
///
/// `health` is still reported via `next_recommendation.rationale` so a caller
/// debugging an omitted parameter can see the system is alive.
pub fn build_unscoped_report(health: f32) -> CognitiveStateReport {
    CognitiveStateReport {
        actor: String::new(),
        actor_scoped: false,
        active_goals: Vec::new(),
        learned_beliefs: 0,
        learned_beliefs_detail: Vec::new(),
        valid_assumptions: Vec::new(),
        recent_decisions: Vec::new(),
        recent_failures: Vec::new(),
        emergent_abstractions: 0,
        emergent_abstractions_detail: Vec::new(),
        open_uncertainties: UncertaintySummary {
            uncertain_beliefs: Vec::new(),
            invalidated_count: 0,
        },
        authorized_actions: Vec::new(),
        next_recommendation: NextAction {
            goal_id: None,
            goal_target: None,
            recommended_op: "supply_actor".to_string(),
            rationale: format!(
                "no `actor` query parameter was supplied, so no memory was read; \
                 every field is empty by construction (health={health:.3})"
            ),
        },
        // An unscoped report reads no memory, so it cannot show a ladder. Empty
        // for the same reason every other field is empty.
        clarify_ladder: Vec::new(),
    }
}

