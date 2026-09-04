//! BeliefExecutive — single mutation authority for Belief confidence + JTMS label.
//!
//! Chain-of-thought: Two separate paths previously mutated Belief state — BeliefInvalidator
//! decayed confidence, jtms::propagate_retraction set JtmsLabel::Out — with no coordination.
//! This left beliefs where label=In but confidence=0.05 (Q3 counted them as valid assumptions).
//! BeliefExecutive is the single point that keeps both fields coherent atomically.

use uuid::Uuid;

use crate::memory_store::MemoryStore;
use crate::persistence::MemoryBackend;

/// Confidence below this threshold triggers JTMS Out-cascade on decay.
const ARCHIVE_THRESHOLD: f32 = 0.2;

pub struct BeliefExecutive;

impl BeliefExecutive {
    /// Decay a Belief's confidence to `new_conf`.
    /// If `new_conf < ARCHIVE_THRESHOLD`, also propagates JTMS Out-cascade so that
    /// `JtmsLabel` and `confidence` always agree.
    /// Returns IDs of beliefs marked Out (empty when above threshold).
    pub fn decay<B: MemoryBackend>(
        store: &mut MemoryStore<B>,
        id: Uuid,
        new_conf: f32,
    ) -> Vec<Uuid> {
        let _ = store.update_record(id, None, None, Some(new_conf), None, None);
        if new_conf < ARCHIVE_THRESHOLD {
            crate::jtms::propagate_retraction(store, id, None, "belief_executive")
        } else {
            vec![]
        }
    }

    /// Retract a belief via JTMS Out-cascade. Also clamps confidence to 0.0 on the
    /// root belief so that `confidence` agrees with the retracted label.
    /// Returns IDs of all beliefs marked Out (including root + cascade).
    pub fn retract<B: MemoryBackend>(
        store: &mut MemoryStore<B>,
        id: Uuid,
        tx: Option<&crate::tx_log::TxLog>,
        actor: &str,
    ) -> Vec<Uuid> {
        let _ = store.update_record(id, None, None, Some(0.0), None, None);
        crate::jtms::propagate_retraction(store, id, tx, actor)
    }
}
