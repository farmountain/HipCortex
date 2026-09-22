//! PolicyRegistry — in-memory store of active Policies by entity.
//!
//! Chain-of-thought: Policies are first-class citizens (MemoryType::Policy) that
//! codify "when trigger_condition, run action_fn" for a given entity. The registry
//! is the fast-path index DigitalTwin::step_with_policies() consults before applying
//! continuous dynamics: the highest-priority active Policy whose trigger fires
//! overrides the caller-supplied action. Registration of an empty-trigger Policy is
//! routed through the clarify ladder (route_uncertainty) so an under-specified
//! policy either self-prompts (proceed as always-active) or is refused at AskUser/
//! DeclineAsk rather than silently installing an ambiguous law.

use std::collections::HashMap;
use uuid::Uuid;

use crate::payloads::PolicyPayload;

/// A registered Policy: its record id plus the strongly-typed payload.
pub struct PolicyRecord {
    pub id: Uuid,
    pub payload: PolicyPayload,
}

/// In-memory index of Policies keyed by their record id.
pub struct PolicyRegistry {
    records: HashMap<Uuid, PolicyRecord>,
}

impl PolicyRegistry {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Register a PolicyRecord. If `trigger_condition` is empty, the registration is
    /// routed through `route_uncertainty`: an unspent ladder rung (SelfPrompt) or no
    /// uncertainty lets an always-active policy through; a spent ladder that resolves to
    /// AskUser / DeclineAsk skips registration (the trigger is too ambiguous to install).
    pub fn register_record(&mut self, record: PolicyRecord) {
        if record.payload.trigger_condition.is_empty() {
            use crate::agent_guidance::{route_uncertainty, ClarifyRoute};
            use crate::clarify_engine::MAX_CLARIFY_TIERS;
            let route = route_uncertainty(true, MAX_CLARIFY_TIERS, false, 1.0);
            if matches!(
                route,
                ClarifyRoute::AskUser { .. } | ClarifyRoute::DeclineAsk { .. }
            ) {
                return; // ambiguous trigger — skip registration
            }
            // SelfPrompt or NoClarification: proceed with empty condition (always-active policy)
        }
        self.records.insert(record.id, record);
    }

    /// Returns active Policies for `entity_id`, sorted by priority descending.
    pub fn active_for_entity(&self, entity_id: Uuid) -> Vec<&PolicyRecord> {
        let mut matching: Vec<&PolicyRecord> = self
            .records
            .values()
            .filter(|r| r.payload.entity_id == entity_id && r.payload.active)
            .collect();
        // Highest priority first. Reverse the u32 key for a descending sort.
        matching.sort_by_key(|r| std::cmp::Reverse(r.payload.priority));
        matching
    }

    /// Deactivate a policy by id. Returns true if the policy was found.
    pub fn deactivate(&mut self, policy_id: Uuid) -> bool {
        if let Some(rec) = self.records.get_mut(&policy_id) {
            rec.payload.active = false;
            true
        } else {
            false
        }
    }
}

impl Default for PolicyRegistry {
    fn default() -> Self {
        Self::new()
    }
}
