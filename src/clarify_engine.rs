//! ClarifyEngine — self-prompting clarity loop for goal problem statements.
//!
//! Chain-of-thought:
//!   Triggered when: (a) goal has empty success_factors, (b) consecutive vetoes ≥ 3,
//!   or (c) pre-success check needed. Searches beliefs and the world model up to
//!   MAX_CLARIFY_ROUNDS times. If self-resolution found, writes Reflexion{self_clarified}
//!   and returns ClarifiedBySubstrate. If unresolved after MAX_ROUNDS, writes a single
//!   deduped Belief{clarify_needed} and returns NeedsUserClarification. Exit is guaranteed
//!   (no indefinite loop).
//!
//! v2.1.0: restate_if_env_changed — rewrites success_factors when environment signals
//! make them unachievable (negation + keyword overlap in recent Temporal records),
//! then writes Reflexion{goal_restated}. Called before the search loop so that
//! month-2 env changes are handled before clarification search fires.

use uuid::Uuid;

const MAX_CLARIFY_ROUNDS: u32 = 3;
const ENV_CHECK_WINDOW: usize = 20;
const FAILURE_KEYWORDS: &[&str] = &[
    "failed", "unavailable", "offline", "error", "timeout", "blocked", "unreachable",
];

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
    /// Detect environment-blocked success_factors and rewrite them in-store.
    ///
    /// Scans the last ENV_CHECK_WINDOW Temporal records for failure signals that
    /// overlap each unsatisfied success_factor name. If blocked: renames the factor
    /// to `{name}_when_available` and writes Reflexion{goal_restated}.
    /// Returns true if any factor was restated.
    pub fn restate_if_env_changed<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
    ) -> bool {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::GoalPayload;

        let goal_rec = match store.find_by_id(goal_id).cloned() {
            Some(r) => r,
            None => return false,
        };
        let mut goal: GoalPayload = match serde_json::from_value(goal_rec.metadata.clone()) {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Gather recent Temporal content as env signal evidence.
        let recent: Vec<String> = store
            .all_by_type(MemoryType::Temporal)
            .into_iter()
            .rev()
            .take(ENV_CHECK_WINDOW)
            .map(|r| format!("{} {}", r.action, r.target).to_lowercase())
            .collect();

        let mut any_restated = false;
        for factor in &mut goal.success_factors {
            if factor.satisfied {
                continue;
            }
            // Skip already-restated factors (idempotent).
            if factor.name.ends_with("_when_available") {
                continue;
            }

            let factor_keywords: Vec<String> = factor
                .name
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| s.len() > 2)
                .map(|s| s.to_lowercase())
                .collect();

            let env_blocked = recent.iter().any(|content| {
                let has_fail = FAILURE_KEYWORDS.iter().any(|kw| content.contains(kw));
                let has_kw = factor_keywords.iter().any(|kw| content.contains(kw.as_str()));
                has_fail && has_kw
            });

            if env_blocked {
                factor.name = format!("{}_when_available", factor.name);
                any_restated = true;
            }
        }

        if any_restated {
            if let Ok(meta) = serde_json::to_value(&goal) {
                let _ = store.update_record(goal_id, None, None, None, None, Some(meta));
            }
            let note = format!("environment-blocked success_factors restated for goal {}", goal_id);
            let mut rec = MemoryRecord::new(
                MemoryType::Reflexion,
                actor.to_string(),
                "goal_restated".to_string(),
                goal_id.to_string(),
                serde_json::json!({ "note": note }),
            );
            rec.derived_from = Some(goal_id);
            let _ = store.add(rec);
        }

        any_restated
    }

    /// Run self-prompting clarification for `goal_id`.
    ///
    /// First calls `restate_if_env_changed` — if the environment blocked any
    /// success_factors and restated them, returns ClarifiedBySubstrate immediately.
    /// Then searches existing beliefs for propositions that mention the goal's target
    /// state; if found, writes Reflexion{self_clarified} and returns ClarifiedBySubstrate.
    /// Otherwise writes a single deduped Belief{clarify_needed} and returns
    /// NeedsUserClarification. MAX_CLARIFY_ROUNDS bounds iteration.
    ///
    /// `wm` is accepted for future WM-aware env detection; currently unused (all env
    /// signals come from recent Temporal records which are already in-store).
    pub fn run<B: crate::persistence::MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        goal_id: Uuid,
        actor: &str,
        trigger: ClarifyTrigger,
        _wm: Option<&crate::world_model_enhanced::WorldModelEnhanced>,
    ) -> ClarifyOutcome {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::{BeliefPayload, GoalPayload};

        // Env-change restatement before belief search — if env signals blocked
        // a success_factor, the problem is restated rather than clarified.
        if Self::restate_if_env_changed(store, goal_id, actor) {
            return ClarifyOutcome::ClarifiedBySubstrate;
        }

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
        let bp = BeliefPayload {
            proposition: format!(
                "goal {} requires user clarification (trigger={:?})",
                goal_id, trigger
            ),
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
