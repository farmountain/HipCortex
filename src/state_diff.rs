use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    memory_store::MemoryStore,
    persistence::MemoryBackend,
    tx_log::{TxKind, TxLog},
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefDelta {
    /// Belief IDs asserted (JTMS → In) in this tx range.
    pub asserted: Vec<Uuid>,
    /// Belief IDs retracted (JTMS → Out) in this tx range.
    pub retracted: Vec<Uuid>,
    /// Number of AssertJustification operations.
    pub justifications_asserted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthDelta {
    /// True if a ForgetActor (GDPR wipe) occurred in this tx range.
    pub gdpr_wipe_occurred: bool,
    /// Number of consolidation passes triggered.
    pub consolidation_passes: u32,
    /// Number of workspace operations (open + merge).
    pub workspace_ops: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStateDiff {
    pub from_tx: u64,
    pub to_tx: u64,
    pub timestamp_range: (u64, u64),
    pub tx_count: u64,
    pub memory_delta: MemoryDelta,
    pub belief_delta: BeliefDelta,
    pub world_model_delta: WorldModelDelta,
    pub health_delta: HealthDelta,
    pub causal_attributions: Vec<CausalAttributionPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryDelta {
    pub added: Vec<Uuid>,
    pub archived: Vec<Uuid>,
    pub updated: Vec<Uuid>,
    pub net_delta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorldModelDelta {
    pub observations_added: u32,
    pub distributions_updated: u32,
    pub causal_edges_added: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAttributionPath {
    pub record_id: Uuid,
    pub tx_id: u64,
    pub trigger_action: String,
    /// Confidence of the record at query time (0.0 if since archived).
    pub confidence_shift: f32,
}

/// Replay TxLog entries in [from_tx, to_tx] and fold into TxStateDiff.
/// Cap: to_tx - from_tx > 10_000 returns Err.
pub fn compute_tx_diff<B: MemoryBackend>(
    log: &TxLog,
    from_tx: u64,
    to_tx: u64,
    store: &MemoryStore<B>,
) -> Result<TxStateDiff, String> {
    if to_tx.saturating_sub(from_tx) > 10_000 {
        return Err("tx range too large — cap at 10,000".to_string());
    }
    let entries = log.query_range(from_tx, to_tx)?;

    let mut delta = MemoryDelta::default();
    let mut belief_delta = BeliefDelta::default();
    let mut wm = WorldModelDelta::default();
    let mut health_delta = HealthDelta::default();
    let mut attributions: Vec<CausalAttributionPath> = Vec::new();
    let mut ts_min = u64::MAX;
    let mut ts_max = 0u64;

    for entry in &entries {
        if entry.timestamp_ms < ts_min {
            ts_min = entry.timestamp_ms;
        }
        if entry.timestamp_ms > ts_max {
            ts_max = entry.timestamp_ms;
        }

        match &entry.kind {
            TxKind::MemoryAdd | TxKind::GoalCreate => {
                delta.added.extend_from_slice(&entry.record_ids);
                for &rid in &entry.record_ids {
                    let conf = store.find_by_id(rid).map(|r| r.confidence).unwrap_or(0.0);
                    attributions.push(CausalAttributionPath {
                        record_id: rid,
                        tx_id: entry.tx_id,
                        trigger_action: format!("{:?}", entry.kind),
                        confidence_shift: conf,
                    });
                }
            }
            TxKind::BeliefAssert => {
                delta.added.extend_from_slice(&entry.record_ids);
                belief_delta.asserted.extend_from_slice(&entry.record_ids);
                belief_delta.justifications_asserted += 1;
                for &rid in &entry.record_ids {
                    let conf = store.find_by_id(rid).map(|r| r.confidence).unwrap_or(0.0);
                    attributions.push(CausalAttributionPath {
                        record_id: rid,
                        tx_id: entry.tx_id,
                        trigger_action: "BeliefAssert".to_string(),
                        confidence_shift: conf,
                    });
                }
            }
            TxKind::BeliefRetract => {
                delta.archived.extend_from_slice(&entry.record_ids);
                belief_delta.retracted.extend_from_slice(&entry.record_ids);
                for &rid in &entry.record_ids {
                    attributions.push(CausalAttributionPath {
                        record_id: rid,
                        tx_id: entry.tx_id,
                        trigger_action: "BeliefRetract".to_string(),
                        confidence_shift: 0.0, // retracted — no longer live
                    });
                }
            }
            TxKind::MemoryArchive | TxKind::MemoryDelete => {
                delta.archived.extend_from_slice(&entry.record_ids);
            }
            TxKind::Consolidate => {
                delta.archived.extend_from_slice(&entry.record_ids);
                health_delta.consolidation_passes += 1;
            }
            TxKind::MemoryUpdate | TxKind::GoalStatusChange => {
                delta.updated.extend_from_slice(&entry.record_ids);
            }
            TxKind::WorldModelObserve => {
                wm.observations_added += entry.record_ids.len() as u32;
                for &rid in &entry.record_ids {
                    attributions.push(CausalAttributionPath {
                        record_id: rid,
                        tx_id: entry.tx_id,
                        trigger_action: "WorldModelObserve".to_string(),
                        confidence_shift: 0.0,
                    });
                }
            }
            TxKind::WorldModelUpdate => {
                wm.distributions_updated += entry.record_ids.len() as u32;
            }
            TxKind::ForgetActor => {
                delta.archived.extend_from_slice(&entry.record_ids);
                health_delta.gdpr_wipe_occurred = true;
            }
            TxKind::ArchiveRecord => {
                delta.archived.extend_from_slice(&entry.record_ids);
            }
            TxKind::WorkspaceOp => {
                health_delta.workspace_ops += 1;
            }
        }
    }

    delta.net_delta = delta.added.len() as i64 - delta.archived.len() as i64;
    let ts_range = if entries.is_empty() {
        (0, 0)
    } else {
        (ts_min, ts_max)
    };

    Ok(TxStateDiff {
        from_tx,
        to_tx,
        timestamp_range: ts_range,
        tx_count: entries.len() as u64,
        memory_delta: delta,
        belief_delta,
        world_model_delta: wm,
        health_delta,
        causal_attributions: attributions,
    })
}

impl TxStateDiff {
    pub fn empty(cursor: u64) -> Self {
        Self {
            from_tx: cursor,
            to_tx: cursor,
            timestamp_range: (0, 0),
            tx_count: 0,
            memory_delta: MemoryDelta::default(),
            belief_delta: BeliefDelta::default(),
            world_model_delta: WorldModelDelta::default(),
            health_delta: HealthDelta::default(),
            causal_attributions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_state_diff_empty_cursor() {
        let diff = TxStateDiff::empty(42);
        assert_eq!(diff.from_tx, 42);
        assert_eq!(diff.to_tx, 42);
        assert_eq!(diff.tx_count, 0);
        assert!(diff.causal_attributions.is_empty());
    }
}
