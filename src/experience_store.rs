//! ExperienceStore — 3-tier memory pyramid: Raw → Episode → Abstract.
//!
//! Chain-of-thought:
//!   Raw     = Active Temporal records (≤ 1000 hot). Dense, uncompressed experience.
//!   Episode = Skill or Belief records with non-empty evidence links. Compressed events.
//!   Abstract = Temporal records with action="consolidated" or target starting "summary:".
//!              Lossy compression, but evidence links preserved.
//!
//! This is a read view over MemoryStore — it does not own records. Call from_store()
//! to materialize tier counts. Consolidation is driven by mine_and_consolidate externally.

use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::persistence::MemoryBackend;
use uuid::Uuid;

pub const RAW_CAP: usize = 1000;
pub const EPISODE_CAP: usize = 100;
pub const ABSTRACT_CAP: usize = 10;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExperienceRecord {
    pub id: uuid::Uuid,
    /// (node_id, equation_tag) pairs for trajectory segment. None = pre-SCM record.
    pub causal_provenance: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone)]
pub struct ExperienceTier {
    pub raw: Vec<Uuid>,
    pub episode: Vec<Uuid>,
    pub abstract_ids: Vec<Uuid>,
}

/// Read-only view of hot store records classified into experience tiers.
pub struct ExperienceStore {
    tiers: ExperienceTier,
}

impl ExperienceStore {
    /// Classify all records in `store` belonging to `actor` into tiers.
    pub fn from_store<B: MemoryBackend + Send + Sync>(
        store: &MemoryStore<B>,
        actor: &str,
    ) -> Self {
        let mut raw = Vec::new();
        let mut episode = Vec::new();
        let mut abstract_ids = Vec::new();

        for r in store.all() {
            if r.actor != actor {
                continue;
            }
            match classify(r) {
                Tier::Raw => raw.push(r.id),
                Tier::Episode => episode.push(r.id),
                Tier::Abstract => abstract_ids.push(r.id),
            }
        }

        Self {
            tiers: ExperienceTier { raw, episode, abstract_ids },
        }
    }

    pub fn raw_count(&self) -> usize { self.tiers.raw.len() }
    pub fn episode_count(&self) -> usize { self.tiers.episode.len() }
    pub fn abstract_count(&self) -> usize { self.tiers.abstract_ids.len() }

    pub fn total_hot(&self) -> usize {
        self.raw_count() + self.episode_count() + self.abstract_count()
    }

    /// Fraction of hot records that are compressed (episode + abstract).
    pub fn compression_ratio(&self) -> f64 {
        let total = self.total_hot();
        if total == 0 { return 0.0; }
        (self.episode_count() + self.abstract_count()) as f64 / total as f64
    }

    /// True if raw tier exceeds cap — caller should trigger consolidation.
    pub fn raw_pressure(&self) -> bool {
        self.raw_count() >= RAW_CAP
    }

    pub fn tiers(&self) -> &ExperienceTier { &self.tiers }

    /// Search episode + abstract tiers for records matching query substring in target.
    pub fn search_compressed<B: MemoryBackend + Send + Sync>(
        &self,
        store: &MemoryStore<B>,
        query: &str,
    ) -> Vec<MemoryRecord> {
        let compressed_ids: std::collections::HashSet<Uuid> = self
            .tiers.episode.iter()
            .chain(&self.tiers.abstract_ids)
            .copied()
            .collect();
        store
            .all()
            .iter()
            .filter(|r| compressed_ids.contains(&r.id) && r.target.contains(query))
            .cloned()
            .collect()
    }
}

enum Tier { Raw, Episode, Abstract }

fn classify(r: &MemoryRecord) -> Tier {
    if r.record_type == MemoryType::Temporal
        && (r.action == "consolidated" || r.target.starts_with("summary:"))
    {
        return Tier::Abstract;
    }
    if (r.record_type == MemoryType::Skill || r.record_type == MemoryType::Belief)
        && !r.evidence.is_empty()
    {
        return Tier::Episode;
    }
    Tier::Raw
}
