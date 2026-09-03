//! Epistemic Authority — gates Belief confidence writes by evidence count.
//!
//! Tiers: Observer (0 evidence → max 0.50), Contributor-low (1-2 → 0.65),
//! Contributor (3-6 → 0.80), Authority (7+ → uncapped).
//! Actor track record: ratio of In vs (In+Out) beliefs for an actor.

use crate::memory_record::MemoryType;
use crate::payloads::{BeliefPayload, JtmsLabel};
use crate::memory_store::MemoryStore;
use crate::persistence::MemoryBackend;

pub struct EpistemicAuthority;

impl EpistemicAuthority {
    /// Return allowed confidence. May clamp requested_confidence down.
    /// Callers must store the returned value, not the original.
    pub fn gate_belief_write(requested: f32, evidence_count: usize) -> f32 {
        let cap = match evidence_count {
            0 => 0.5,
            1..=2 => 0.65,
            3..=6 => 0.8,
            _ => 1.0,
        };
        requested.min(cap)
    }

    /// Ratio of actor's JtmsLabel::In beliefs vs total (In + Out). 1.0 if no history.
    pub fn actor_track_record<B: MemoryBackend>(store: &MemoryStore<B>, actor: &str) -> f32 {
        let beliefs: Vec<BeliefPayload> = store
            .all_by_type(MemoryType::Belief)
            .into_iter()
            .filter(|r| r.actor == actor)
            .filter_map(|r| serde_json::from_value(r.metadata.clone()).ok())
            .collect();

        if beliefs.is_empty() {
            return 1.0;
        }

        let in_count = beliefs.iter().filter(|p| p.jtms_label == JtmsLabel::In).count();
        let out_count = beliefs.iter().filter(|p| p.jtms_label == JtmsLabel::Out).count();
        let total = in_count + out_count;

        if total == 0 { 1.0 } else { in_count as f32 / total as f32 }
    }
}
