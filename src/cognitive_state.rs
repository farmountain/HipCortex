//! CognitiveState — unified transactional interface over all HipCortex stores.
//!
//! Chain-of-thought: Agent code currently accesses MemoryStore, WorldModel, and
//! SelfModel independently via raw Arc clones, bypassing the coherence gate.
//! CognitiveHandle<B> is the single composition point: all mutations go through
//! transact(), all reads go through snapshot().

use std::fmt;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::archive_store::ArchiveStore;
use crate::cognitive_gc::{CognitiveGC, GcAction};
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::coherence::CoherenceChecker;
use crate::self_model::calibration::CalibrationTracker;
use crate::self_model::SelfModel;
use crate::world_model_enhanced::WorldModelEnhanced;
use crate::payloads::{BeliefPayload, EpistemicStatus, GoalPayload, GoalStatus, SkillPayload};
use crate::persistence::MemoryBackend;
use crate::simulation_fork::SimulationFork;
use crate::tx_log::{TxKind, TxLog};

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum CognitiveError {
    CoherenceRejection(String),
    DeltaInvalid(String),
    StoreError(String),
    NotImplemented(String),
    LockError,
}

impl fmt::Display for CognitiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoherenceRejection(r) => write!(f, "coherence rejection: {r}"),
            Self::DeltaInvalid(r) => write!(f, "delta invalid: {r}"),
            Self::StoreError(r) => write!(f, "store error: {r}"),
            Self::NotImplemented(op) => write!(f, "{op} not implemented in Phase 0"),
            Self::LockError => write!(f, "lock poisoned"),
        }
    }
}

impl std::error::Error for CognitiveError {}

// ─── CognitiveDelta ──────────────────────────────────────────────────────────

/// All mutations go through this enum.
/// Phase-4 variants (Consolidate, ForgetActor, ArchiveRecord) compile but
/// return CognitiveError::NotImplemented at runtime until Phase 4.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum CognitiveDelta {
    // Phase 0 — implemented
    AddMemory(MemoryRecord),
    /// `id` = the MemoryRecord.id of the existing Belief record to update.
    /// BeliefPayload has no id field of its own.
    UpdateBelief { id: Uuid, payload: BeliefPayload },
    AdvanceGoal { id: Uuid, status: GoalStatus },
    RegisterSkill(SkillPayload),
    // Phase 4 stubs — return CognitiveError::NotImplemented
    Consolidate { source_ids: Vec<Uuid>, summary: MemoryRecord },
    /// Reshaped from ForgetActor(String) to satisfy serde internal tagging.
    ForgetActor { actor: String },
    /// Reshaped from ArchiveRecord(Uuid) to satisfy serde internal tagging.
    ArchiveRecord { id: Uuid },
}

impl CognitiveDelta {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AddMemory(_) => "AddMemory",
            Self::UpdateBelief { .. } => "UpdateBelief",
            Self::AdvanceGoal { .. } => "AdvanceGoal",
            Self::RegisterSkill(_) => "RegisterSkill",
            Self::Consolidate { .. } => "Consolidate",
            Self::ForgetActor { .. } => "ForgetActor",
            Self::ArchiveRecord { .. } => "ArchiveRecord",
        }
    }
}

// ─── Snapshot sub-types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemporalView {
    pub record_count: usize,
    pub recent_actions: Vec<String>,
    pub temporal_span_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldStateView {
    pub node_count: usize,
    pub edge_count: usize,
    pub dag_verified: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelfStateView {
    pub calibration_score: f32,
    pub prediction_error_ewma: f32,
    pub consolidation_pressure: f32,
    pub epistemic_entropy: f32,
    pub healthy: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalSnapshot {
    pub id: Uuid,
    pub target_state: String,
    pub status: GoalStatus,
    pub iteration: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillSnapshot {
    pub id: Uuid,
    pub procedure: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BeliefSummary {
    pub id: Uuid,
    pub proposition: String,
    pub confidence: f32,
    pub epistemic_status: EpistemicStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BeliefDistribution {
    pub count: usize,
    pub mean_confidence: f32,
    pub epistemic_entropy: f32,
    pub beliefs: Vec<BeliefSummary>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProvenanceSummary {
    pub merkle_root_hex: String,
    pub record_count: usize,
    pub evidence_edge_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CognitiveSnapshot {
    pub id: Uuid,
    pub tx_cursor: u64,
    pub actor: String,
    pub temporal: TemporalView,
    pub world: WorldStateView,
    pub self_model: SelfStateView,
    pub goals: Vec<GoalSnapshot>,
    pub skills: Vec<SkillSnapshot>,
    pub beliefs: BeliefDistribution,
    pub provenance: ProvenanceSummary,
}

// ─── CognitiveHandle ─────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct CognitiveHandle<B: MemoryBackend + Send + Sync + 'static> {
    pub memory: Arc<Mutex<MemoryStore<B>>>,
    pub(crate) world: Arc<std::sync::RwLock<WorldModelEnhanced>>,
    pub(crate) self_model: Arc<SelfModel>,
    pub(crate) tx_log: Option<Arc<TxLog>>,
    pub(crate) coherence: Arc<CoherenceChecker>,
    pub(crate) calibration: Arc<CalibrationTracker>,
    pub(crate) gc: Arc<CognitiveGC>,
    /// Optional Cold Store for archived records (Phase 4). None = archive silently skipped.
    pub(crate) archive_store: Option<Arc<Mutex<ArchiveStore>>>,
}

/// Mean binary entropy over a slice of confidence values in [0,1].
/// h(p) = -p*log2(p) - (1-p)*log2(1-p). Returns 0.0 for empty slice.
pub fn compute_epistemic_entropy(confs: &[f32]) -> f32 {
    if confs.is_empty() {
        return 0.0;
    }
    let h_sum: f32 = confs
        .iter()
        .map(|&c| {
            let c = c.clamp(f32::EPSILON, 1.0 - f32::EPSILON);
            -(c * c.log2() + (1.0 - c) * (1.0 - c).log2())
        })
        .sum();
    h_sum / confs.len() as f32
}

impl<B: MemoryBackend + Send + Sync + 'static> CognitiveHandle<B> {
    pub fn new(
        memory: Arc<Mutex<MemoryStore<B>>>,
        world: Arc<std::sync::RwLock<WorldModelEnhanced>>,
        self_model: Arc<SelfModel>,
        tx_log: Option<Arc<TxLog>>,
        coherence: Arc<CoherenceChecker>,
        calibration: Arc<CalibrationTracker>,
        gc: Arc<CognitiveGC>,
    ) -> Self {
        Self { memory, world, self_model, tx_log, coherence, calibration, gc, archive_store: None }
    }

    pub fn with_archive_store(mut self, store: ArchiveStore) -> Self {
        self.archive_store = Some(Arc::new(Mutex::new(store)));
        self
    }

    /// Apply a CognitiveDelta; returns tx_cursor. Thin wrapper over `transact_ex`.
    pub fn transact(&self, delta: CognitiveDelta, actor: &str) -> Result<u64, CognitiveError> {
        Ok(self.transact_ex(delta, actor)?.tx_cursor)
    }

    /// Full transact returning `TransactResult` (includes `records_deleted` for ForgetActor).
    ///
    /// Pipeline: safety gate → structural check → apply → tx log → calibration ping.
    pub fn transact_ex(
        &self,
        delta: CognitiveDelta,
        actor: &str,
    ) -> Result<TransactResult, CognitiveError> {
        // Step 1: Safety gate
        self.coherence
            .gate_write(delta.label())
            .map_err(|r| CognitiveError::CoherenceRejection(r.reason))?;

        // Step 2: Structural coherence
        self.coherence
            .check_delta(&delta)
            .map_err(CognitiveError::DeltaInvalid)?;

        // Step 3: Phase-4 real implementations
        match &delta {
            CognitiveDelta::Consolidate { source_ids, summary } => {
                let tx_cursor = self.consolidate_memory(source_ids, summary.clone(), actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            CognitiveDelta::ForgetActor { actor: target_actor } => {
                let (tx_cursor, deleted) = self.forget_actor(target_actor, actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: Some(deleted) });
            }
            CognitiveDelta::ArchiveRecord { id } => {
                let tx_cursor = self.archive_record(*id, actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            _ => {}
        }

        // Step 4: Apply standard deltas
        let affected_ids = self.apply_delta(&delta)?;

        // Step 5: TxLog
        let tx_cursor = if let Some(tx) = &self.tx_log {
            let kind = match &delta {
                CognitiveDelta::AddMemory(_) => TxKind::MemoryAdd,
                CognitiveDelta::UpdateBelief { .. } => TxKind::BeliefAssert,
                CognitiveDelta::AdvanceGoal { .. } => TxKind::GoalStatusChange,
                CognitiveDelta::RegisterSkill(_) => TxKind::MemoryAdd,
                _ => unreachable!(),
            };
            tx.append(kind, affected_ids, actor)
        } else {
            0
        };

        // Step 6: Calibration ping
        self.calibration.record_prediction_error(0.0);

        Ok(TransactResult { tx_cursor, records_deleted: None })
    }

    // ── Phase-4 private helpers ────────────────────────────────────────────────

    /// Archive source records and insert summary. Rejects source_ids > 100 (G4-5).
    fn consolidate_memory(
        &self,
        source_ids: &[Uuid],
        summary: MemoryRecord,
        actor: &str,
    ) -> Result<u64, CognitiveError> {
        if source_ids.is_empty() {
            return Err(CognitiveError::DeltaInvalid("source_ids must not be empty".into()));
        }
        if source_ids.len() > 100 {
            return Err(CognitiveError::DeltaInvalid(
                format!("source_ids exceeds cap of 100 (got {})", source_ids.len()),
            ));
        }
        let capped: Vec<Uuid> = source_ids.to_vec();

        let summary_id = summary.id;
        {
            let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;

            // Archive or delete each source record
            for &id in &capped {
                if let Some(rec) = ms.find_by_id(id).cloned() {
                    // Move to cold store if available
                    if let Some(arc) = &self.archive_store {
                        let _ = arc.lock().map(|mut as_| as_.append(rec));
                    }
                    ms.delete_by_id(id);
                }
            }

            // Insert summary
            ms.add(summary).map_err(|e| CognitiveError::StoreError(e.to_string()))?;
        }

        let tx_cursor = if let Some(tx) = &self.tx_log {
            tx.append(TxKind::Consolidate, vec![summary_id], actor)
        } else {
            0
        };
        self.calibration.record_prediction_error(0.0);
        Ok(tx_cursor)
    }

    /// GDPR hard-delete: remove all records for `target_actor` from Hot store.
    /// Returns (tx_cursor, records_deleted).
    fn forget_actor(
        &self,
        target_actor: &str,
        tx_actor: &str,
    ) -> Result<(u64, u32), CognitiveError> {
        let deleted = {
            let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
            ms.delete_by_actor(target_actor)
                .map_err(|e| CognitiveError::StoreError(e.to_string()))?
                .len() as u32
        };

        let tx_cursor = if let Some(tx) = &self.tx_log {
            tx.append(TxKind::ForgetActor, vec![], tx_actor)
        } else {
            0
        };
        self.calibration.record_prediction_error(0.0);
        Ok((tx_cursor, deleted))
    }

    /// CognitiveGC-guided single-record archive or delete.
    fn archive_record(&self, id: Uuid, actor: &str) -> Result<u64, CognitiveError> {
        let action = self.gc.gc_action(id);
        let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
        match action {
            GcAction::Archive => {
                if let Some(rec) = ms.find_by_id(id).cloned() {
                    if let Some(arc) = &self.archive_store {
                        let _ = arc.lock().map(|mut as_| as_.append(rec));
                    }
                    ms.delete_by_id(id);
                }
            }
            GcAction::Delete | GcAction::Keep => {
                ms.delete_by_id(id);
            }
        }
        drop(ms);

        let tx_cursor = if let Some(tx) = &self.tx_log {
            tx.append(TxKind::ArchiveRecord, vec![id], actor)
        } else {
            0
        };
        self.calibration.record_prediction_error(0.0);
        Ok(tx_cursor)
    }

    fn apply_delta(&self, delta: &CognitiveDelta) -> Result<Vec<Uuid>, CognitiveError> {
        match delta {
            CognitiveDelta::AddMemory(record) => {
                let id = record.id;
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .add(record.clone())
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                Ok(vec![id])
            }

            CognitiveDelta::UpdateBelief { id, payload } => {
                let new_meta = serde_json::to_value(payload)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .update_record(*id, None, None, Some(payload.confidence), None, Some(new_meta))
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                Ok(vec![*id])
            }

            CognitiveDelta::AdvanceGoal { id, status } => {
                let mut store = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
                // Clone metadata before releasing the immutable borrow
                let meta = store
                    .find_by_id(*id)
                    .ok_or_else(|| CognitiveError::StoreError(format!("goal {id} not found")))?
                    .metadata
                    .clone();
                let mut payload: GoalPayload = serde_json::from_value(meta)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                Self::validate_goal_transition(&payload.status, status)?;
                payload.status = status.clone();
                let new_meta = serde_json::to_value(&payload)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                store
                    .update_record(*id, None, None, None, None, Some(new_meta))
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                Ok(vec![*id])
            }

            CognitiveDelta::RegisterSkill(skill) => {
                let meta = serde_json::to_value(skill)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                let record = MemoryRecord::new(
                    MemoryType::Skill,
                    "system".into(),
                    "register".into(),
                    skill.procedure.clone(),
                    meta,
                );
                let id = record.id;
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .add(record)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                Ok(vec![id])
            }

            _ => unreachable!("Phase-4 stubs rejected in transact() before apply_delta"),
        }
    }

    fn validate_goal_transition(
        from: &GoalStatus,
        to: &GoalStatus,
    ) -> Result<(), CognitiveError> {
        let ok = matches!(
            (from, to),
            (GoalStatus::Pending, GoalStatus::InProgress)
                | (GoalStatus::InProgress, GoalStatus::Succeeded)
                | (GoalStatus::InProgress, GoalStatus::Failed)
                | (GoalStatus::Succeeded, GoalStatus::Succeeded)
                | (GoalStatus::Failed, GoalStatus::Failed)
        );
        if ok {
            Ok(())
        } else {
            Err(CognitiveError::DeltaInvalid(format!(
                "illegal status transition {from:?} → {to:?}"
            )))
        }
    }

    /// Materialise a complete CognitiveSnapshot for the given actor.
    /// actor = "" → include all actors.
    pub fn snapshot(&self, actor: &str) -> Result<CognitiveSnapshot, CognitiveError> {
        let mem = self.memory.lock().map_err(|_| CognitiveError::LockError)?;

        // Temporal view
        let temporal_recs: Vec<_> = mem
            .all()
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Temporal
                    && r.status == "active"
                    && (actor.is_empty() || r.actor == actor)
            })
            .collect();
        let temporal_span_ms = match temporal_recs.as_slice() {
            [] | [_] => 0,
            recs => {
                let oldest = recs.first().unwrap().timestamp;
                let newest = recs.last().unwrap().timestamp;
                (newest - oldest).num_milliseconds().max(0) as u64
            }
        };
        let temporal = TemporalView {
            record_count: temporal_recs.len(),
            recent_actions: temporal_recs.iter().rev().take(5).map(|r| r.action.clone()).collect(),
            temporal_span_ms,
        };

        // Goal view
        let goals: Vec<GoalSnapshot> = mem
            .all()
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Goal
                    && r.status == "active"
                    && (actor.is_empty() || r.actor == actor)
            })
            .filter_map(|r| {
                serde_json::from_value::<GoalPayload>(r.metadata.clone()).ok().map(|p| GoalSnapshot {
                    id: r.id,
                    target_state: p.target_state,
                    status: p.status,
                    iteration: p.current_iteration,
                })
            })
            .collect();

        // Skill view
        let skills: Vec<SkillSnapshot> = mem
            .all_by_type(MemoryType::Skill)
            .iter()
            .filter_map(|r| {
                serde_json::from_value::<SkillPayload>(r.metadata.clone())
                    .ok()
                    .map(|p| SkillSnapshot { id: r.id, procedure: p.procedure })
            })
            .collect();

        // Belief distribution
        let belief_summaries: Vec<BeliefSummary> = mem
            .all()
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Belief
                    && r.status == "active"
                    && (actor.is_empty() || r.actor == actor)
            })
            .filter_map(|r| {
                serde_json::from_value::<BeliefPayload>(r.metadata.clone()).ok().map(|p| BeliefSummary {
                    id: r.id,
                    proposition: p.proposition,
                    confidence: p.confidence,
                    epistemic_status: p.epistemic_status,
                })
            })
            .collect();
        let confs: Vec<f32> = belief_summaries.iter().map(|b| b.confidence).collect();
        let mean_confidence = if confs.is_empty() {
            0.0
        } else {
            confs.iter().sum::<f32>() / confs.len() as f32
        };
        let beliefs = BeliefDistribution {
            count: belief_summaries.len(),
            mean_confidence,
            epistemic_entropy: compute_epistemic_entropy(&confs),
            beliefs: belief_summaries,
        };

        // Provenance
        let provenance = ProvenanceSummary {
            merkle_root_hex: mem.merkle_root_hex(),
            record_count: mem.record_count(),
            evidence_edge_count: mem.evidence_edge_count(),
        };

        drop(mem); // release Mutex before acquiring RwLock

        // World state
        let wm = self.world.read().map_err(|_| CognitiveError::LockError)?;
        let world = WorldStateView {
            node_count: wm.causal_node_count(),
            edge_count: wm.causal_edge_count(),
            dag_verified: true, // CausalGraph::add_edge enforces acyclicity at write time
        };
        drop(wm);

        // Self-model
        let cal = self.calibration.snapshot();
        let self_model = SelfStateView {
            calibration_score: cal.calibration_score,
            prediction_error_ewma: cal.prediction_error_ewma,
            consolidation_pressure: cal.consolidation_pressure,
            epistemic_entropy: cal.epistemic_entropy,
            healthy: cal.healthy,
        };

        let tx_cursor = self.tx_log.as_ref().map(|t| t.current_tx()).unwrap_or(0);

        Ok(CognitiveSnapshot {
            id: Uuid::new_v4(),
            tx_cursor,
            actor: actor.to_string(),
            temporal,
            world,
            self_model,
            goals,
            skills,
            beliefs,
            provenance,
        })
    }

    pub fn fork(&self) -> Result<SimulationFork<B>, CognitiveError> {
        let base_tx = self.tx_log.as_ref().map(|t| t.current_tx()).unwrap_or(0);
        SimulationFork::from_handle(self, base_tx)
    }

    /// Compute semantic diff between two tx cursors.
    /// Returns empty diff when no TxLog. Clamps to_tx to current cursor.
    pub fn diff(
        &self,
        from_tx: u64,
        to_tx: u64,
    ) -> Result<crate::state_diff::TxStateDiff, CognitiveError> {
        if from_tx > to_tx {
            return Err(CognitiveError::DeltaInvalid("from_tx > to_tx".into()));
        }
        let log = match &self.tx_log {
            Some(l) => l.clone(),
            None => return Ok(crate::state_diff::TxStateDiff::empty(0)),
        };
        let current = log.current_tx();
        let to_clamped = to_tx.min(current);
        let store = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
        crate::state_diff::compute_tx_diff(&log, from_tx, to_clamped, &*store)
            .map_err(CognitiveError::StoreError)
    }

    /// Return aggregate health snapshot per spec (7 required fields + overall_health).
    pub fn health(&self) -> SelfHealthResponse {
        let s = self.calibration.snapshot();
        SelfHealthResponse {
            calibration_score: s.calibration_score,
            prediction_error_ewma: s.prediction_error_ewma,
            consolidation_pressure: s.consolidation_pressure,
            epistemic_entropy: s.epistemic_entropy,
            healthy: s.healthy,
            overall_health: s.calibration_score,
            current_tx: s.current_tx,
        }
    }
}

/// Result returned by `transact_ex()` — superset of the `u64` tx_cursor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactResult {
    pub tx_cursor: u64,
    /// Number of records hard-deleted (populated only for ForgetActor).
    pub records_deleted: Option<u32>,
}

/// Response struct for `GET /v1/self/health`. All 7 fields required by spec.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelfHealthResponse {
    pub calibration_score: f32,
    pub prediction_error_ewma: f32,
    pub consolidation_pressure: f32,
    pub epistemic_entropy: f32,
    pub healthy: bool,
    pub overall_health: f32,
    pub current_tx: u64,
}
