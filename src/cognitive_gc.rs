//! Provenance-aware Cognitive Garbage Collector.
//! When a MemoryRecord decays to relevance_score <= 0.0, the GC checks
//! whether any Goal or Belief references it via the `evidence` field.
//! Referenced records move to the ArchiveStore; unreferenced records are deleted.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// MDL sparsity threshold above which a Law is always retained.
///
/// A Law's `mdl_score` measures how much description-length (compression) it earns.
/// At or above this value the Law justifies its own storage cost regardless of how
/// many Goals/Beliefs currently reference it, so [`CognitiveGC::gc_action_for_law`]
/// returns [`GcAction::Keep`]. The comparison is inclusive (`>=`).
pub const MDL_KEEP_THRESHOLD: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub enum GcAction {
    /// Record is still live — decay score not yet 0.
    Keep,
    /// Record is unreferenced — hard delete.
    Delete,
    /// Record is referenced by at least one Goal or Belief — move to archive.
    Archive,
}

pub struct CognitiveGC {
    /// Map from record_id → set of referencing Goal/Belief record IDs
    references: HashMap<Uuid, HashSet<Uuid>>,
}

impl CognitiveGC {
    pub fn new() -> Self {
        Self {
            references: HashMap::new(),
        }
    }

    /// Register that `referencing_id` holds `record_id` in its evidence[].
    pub fn register_reference(&mut self, record_id: Uuid, referencing_id: Uuid) {
        self.references
            .entry(record_id)
            .or_default()
            .insert(referencing_id);
    }

    /// Remove all references held by `referencing_id` (called when Goal/Belief is deleted).
    pub fn deregister_referencing(&mut self, referencing_id: Uuid) {
        self.references.retain(|_, refs| {
            refs.remove(&referencing_id);
            !refs.is_empty()
        });
    }

    /// Determine GC action for a record whose relevance_score has reached 0.
    pub fn gc_action(&self, record_id: Uuid) -> GcAction {
        match self.references.get(&record_id) {
            Some(refs) if !refs.is_empty() => GcAction::Archive,
            _ => GcAction::Delete,
        }
    }

    /// GC decision for Laws based on MDL (Minimum Description Length) sparsity pressure.
    ///
    /// Laws that pay for themselves in compression — `mdl_score >= MDL_KEEP_THRESHOLD` —
    /// are always kept, even when no Goal/Belief currently references them: their value is
    /// the sparsity they buy, not their in-degree. Below the threshold the standard
    /// reference-based policy applies: `Archive` if still referenced (audit trail), `Delete`
    /// if orphaned.
    pub fn gc_action_for_law(&self, record_id: Uuid, mdl_score: f64) -> GcAction {
        if mdl_score >= MDL_KEEP_THRESHOLD {
            return GcAction::Keep;
        }
        match self.references.get(&record_id) {
            Some(refs) if !refs.is_empty() => GcAction::Archive,
            _ => GcAction::Delete,
        }
    }

    /// Bulk-register references from stored records' evidence fields.
    /// Call on startup to rebuild the reference map.
    pub fn rebuild_from_records(&mut self, records: &[crate::memory_record::MemoryRecord]) {
        self.references.clear();
        for r in records {
            for &evidence_id in &r.evidence {
                self.register_reference(evidence_id, r.id);
            }
        }
    }
}

impl Default for CognitiveGC {
    fn default() -> Self {
        Self::new()
    }
}
