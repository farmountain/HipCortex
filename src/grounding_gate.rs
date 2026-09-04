//! GroundingGate (v2.3.0) — blocks instrumental planning when env is ungrounded.
//!
//! Chain-of-thought:
//!   An agent entering a new workspace has zero real observations. The WM twin
//!   may predict entity states via Kalman, but those are PredictedOnly.
//!   GroundingGate activates when coverage(Ê; goal predicates) < τ_c = 0.6
//!   or any goal-relevant entity is Virgin/Stale or epistemic > τ_e = 0.5.
//!   While active: only Probe intents are legal; instrumental ops are Denied.
//!   Exit: all goal-relevant entities have n_observations ≥ 4 (epistemic ≤ 0.5)
//!   and coverage ≥ τ_c. Else Q10 escalates to user (human is the last sensor).

use crate::action_intent::{
    ActionIntent, EntityContactRecord, GroundingStatus, IntentStatus, MAPPED_OBS_THRESHOLD,
};
use crate::memory_record::MemoryRecord;

pub const TAU_C: f32 = 0.6; // coverage threshold — fraction of goal entities that must be Mapped
pub const TAU_E: f32 = 0.5; // max allowed epistemic uncertainty per entity (= 1/sqrt(4+1))

/// Computes the coverage fraction of goal_entities that have grounding_status == Mapped.
/// Returns 1.0 if goal_entities is empty (trivially grounded).
pub fn coverage_from_records(
    goal_entities: &[&str],
    contact_fn: impl Fn(&str) -> Option<EntityContactRecord>,
) -> f32 {
    if goal_entities.is_empty() {
        return 1.0;
    }
    let mapped = goal_entities.iter().filter(|&&e| {
        contact_fn(e).map(|c| c.n_observations >= MAPPED_OBS_THRESHOLD).unwrap_or(false)
    }).count();
    mapped as f32 / goal_entities.len() as f32
}

/// Whether any goal-relevant entity is Virgin or Stale.
pub fn has_virgin_or_stale(
    goal_entities: &[&str],
    contact_fn: impl Fn(&str) -> Option<EntityContactRecord>,
) -> bool {
    goal_entities.iter().any(|&e| {
        match contact_fn(e).map(|c| c.grounding_status) {
            None => true, // not tracked = effectively Virgin
            Some(GroundingStatus::Virgin | GroundingStatus::Stale) => true,
            _ => false,
        }
    })
}

/// Main gate: true = grounding active (instrumental planning blocked).
pub struct GroundingGate;

impl GroundingGate {
    /// Returns true if grounding is active.
    pub fn is_active(
        goal_entities: &[&str],
        contact_fn: impl Fn(&str) -> Option<EntityContactRecord>,
    ) -> bool {
        if goal_entities.is_empty() {
            return false;
        }
        let coverage = coverage_from_records(goal_entities, |e| contact_fn(e));
        if coverage < TAU_C {
            return true;
        }
        // Also block if any entity's epistemic uncertainty exceeds τ_e
        let max_epistemic = goal_entities.iter().filter_map(|&e| {
            contact_fn(e).map(|c| c.epistemic())
        }).fold(0.0f32, f32::max);
        max_epistemic > TAU_E
    }

    /// Pick the highest-priority probe target: entity with worst epistemic (1/sqrt(n+1)).
    pub fn top_probe_target(
        goal_entities: &[&str],
        contact_fn: impl Fn(&str) -> Option<EntityContactRecord>,
    ) -> Option<String> {
        goal_entities.iter().max_by(|&&a, &&b| {
            let ea = contact_fn(a).map(|c| c.epistemic()).unwrap_or(1.0);
            let eb = contact_fn(b).map(|c| c.epistemic()).unwrap_or(1.0);
            ea.partial_cmp(&eb).unwrap_or(std::cmp::Ordering::Equal)
        }).map(|&s| s.to_string())
    }

    /// Expire Open/InFlight intents past their deadline. Returns list of expired intent ids.
    pub fn expire_stale(intents: &mut Vec<ActionIntent>) -> Vec<uuid::Uuid> {
        let mut expired = Vec::new();
        for intent in intents.iter_mut() {
            if intent.is_expired() {
                intent.status = IntentStatus::Expired;
                expired.push(intent.id);
            }
        }
        expired
    }

    /// Extract entities that are Virgin or never contacted from a Temporal record slice.
    /// Used by report to build the "ungrounded" list for Q8.
    pub fn ungrounded_entities_from_store(
        temporals: &[MemoryRecord],
        contacts_fn: impl Fn(&str) -> Option<EntityContactRecord>,
    ) -> Vec<String> {
        let mut entities: std::collections::HashSet<String> = std::collections::HashSet::new();
        for rec in temporals {
            let tag = &rec.target;
            if tag.starts_with("probe_entity:") {
                let entity = tag.trim_start_matches("probe_entity:");
                entities.insert(entity.to_string());
            }
        }
        entities.into_iter().filter(|e| {
            contacts_fn(e).map(|c| matches!(c.grounding_status, GroundingStatus::Virgin | GroundingStatus::Stale)).unwrap_or(true)
        }).collect()
    }
}
