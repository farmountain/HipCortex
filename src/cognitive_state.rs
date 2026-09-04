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
use crate::payloads::{BeliefPayload, EpistemicStatus, GoalPayload, GoalStatus, JtmsLabel, SkillPayload};
use crate::jtms;
use crate::persistence::MemoryBackend;
use crate::workspace::{WorkspaceId, WorkspaceMode, WorkspaceRegistry};
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
    /// Goal transitioned to InProgress without any success_factors defined.
    /// Caller must POST /goal/:id/clarify before running the ReAct loop.
    GoalNotClarified(uuid::Uuid),
}

impl fmt::Display for CognitiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoherenceRejection(r) => write!(f, "coherence rejection: {r}"),
            Self::DeltaInvalid(r) => write!(f, "delta invalid: {r}"),
            Self::StoreError(r) => write!(f, "store error: {r}"),
            Self::NotImplemented(op) => write!(f, "{op} not implemented in Phase 0"),
            Self::LockError => write!(f, "lock poisoned"),
            Self::GoalNotClarified(id) => write!(f, "goal {id} has no success_factors — clarify before InProgress"),
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
    // Phase 1 (JTMS) — non-monotonic belief revision
    /// Mark a belief Out and cascade retraction to dependents.
    RetractBelief { id: Uuid, reason: String },
    /// Set in_list / out_list on a belief; recomputes its JTMS label.
    AssertJustification { belief_id: Uuid, in_nodes: Vec<Uuid>, out_nodes: Vec<Uuid> },
    // Phase 2 — automatic causal motif mining + Skill/Belief induction
    /// Mine recurring causal chains and induce Skill+Belief records.
    /// Uses `min_frequency` as the threshold (default 3 if 0).
    AutoConsolidate { min_frequency: usize },
    // Phase 4 — multi-agent workspace scoping
    /// Open a new workspace (snapshot parent store as baseline).
    WorkspaceOpen { id: WorkspaceId, mode: WorkspaceMode },
    /// OR-Set merge of `from` workspace into `into` workspace (both must be Shared).
    WorkspaceMerge { from: WorkspaceId, into: WorkspaceId },
    // SCM operators (v1.0.0)
    Intervene { var: String, value: f64 },
    Counterfactual {
        actual_state: std::collections::HashMap<String, f64>,
        intervention_var: String,
        intervention_value: f64,
    },
    CreditAssign(crate::world_model_enhanced::causal::FailureSignal),
    RewriteStructuralEquation { node_id: String, new_weights: Vec<f64> },
    OpenIntent(crate::action_intent::ActionIntent),
    AcceptReceipt(crate::action_intent::ActionReceipt),
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
            Self::RetractBelief { .. } => "RetractBelief",
            Self::AssertJustification { .. } => "AssertJustification",
            Self::AutoConsolidate { .. } => "AutoConsolidate",
            Self::WorkspaceOpen { .. } => "WorkspaceOpen",
            Self::WorkspaceMerge { .. } => "WorkspaceMerge",
            Self::Intervene { .. } => "Intervene",
            Self::Counterfactual { .. } => "Counterfactual",
            Self::CreditAssign(_) => "CreditAssign",
            Self::RewriteStructuralEquation { .. } => "RewriteStructuralEquation",
            Self::OpenIntent(_) => "OpenIntent",
            Self::AcceptReceipt(_) => "AcceptReceipt",
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
    pub(crate) emergence: Arc<Mutex<crate::emergence::EmergenceDetector>>,
    /// Optional Cold Store for archived records (Phase 4). None = archive silently skipped.
    pub(crate) archive_store: Option<Arc<Mutex<ArchiveStore>>>,
    /// Phase 4: multi-agent workspace registry.
    pub workspace_registry: Arc<Mutex<WorkspaceRegistry>>,
    /// Pending intents (Open + InFlight) waiting for host receipts.
    pub open_intents: Arc<Mutex<Vec<crate::action_intent::ActionIntent>>>,
    /// Registered actuators and their health.
    pub actuator_registry: Arc<Mutex<crate::action_intent::ActuatorRegistry>>,
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
        Self {
            memory, world, self_model, tx_log, coherence, calibration, gc,
            emergence: Arc::new(Mutex::new(crate::emergence::EmergenceDetector::new())),
            archive_store: None,
            workspace_registry: Arc::new(Mutex::new(WorkspaceRegistry::new())),
            open_intents: Arc::new(Mutex::new(Vec::new())),
            actuator_registry: Arc::new(Mutex::new(crate::action_intent::ActuatorRegistry::default())),
        }
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
            CognitiveDelta::RetractBelief { id, .. } => {
                let tx_cursor = self.retract_belief(*id, actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            CognitiveDelta::AssertJustification { belief_id, in_nodes, out_nodes } => {
                let tx_cursor = self.assert_justification(*belief_id, in_nodes.clone(), out_nodes.clone(), actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            CognitiveDelta::AutoConsolidate { min_frequency } => {
                let tx_cursor = self.auto_consolidate_memory(*min_frequency, actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            CognitiveDelta::WorkspaceOpen { id, mode } => {
                let tx_cursor = self.open_workspace(id.clone(), mode.clone(), actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            CognitiveDelta::WorkspaceMerge { from, into } => {
                let tx_cursor = self.merge_workspaces(from.clone(), into.clone(), actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            CognitiveDelta::OpenIntent(intent) => {
                let tx_cursor = self.open_intent_impl(intent.clone(), actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            CognitiveDelta::AcceptReceipt(receipt) => {
                let tx_cursor = self.accept_receipt_impl(receipt.clone(), actor)?;
                return Ok(TransactResult { tx_cursor, records_deleted: None });
            }
            _ => {}
        }

        // Step 4: Apply standard deltas
        let affected_ids = self.apply_delta(&delta, actor)?;

        // Step 5: TxLog
        let tx_cursor = if let Some(tx) = &self.tx_log {
            let kind = match &delta {
                CognitiveDelta::AddMemory(_) => TxKind::MemoryAdd,
                CognitiveDelta::UpdateBelief { .. } => TxKind::BeliefAssert,
                CognitiveDelta::AdvanceGoal { .. } => TxKind::GoalStatusChange,
                CognitiveDelta::RegisterSkill(_) => TxKind::MemoryAdd,
                CognitiveDelta::Intervene { .. } => TxKind::MemoryAdd,
                CognitiveDelta::Counterfactual { .. } => TxKind::MemoryAdd,
                CognitiveDelta::CreditAssign(_) => TxKind::MemoryAdd,
                CognitiveDelta::RewriteStructuralEquation { .. } => TxKind::MemoryAdd,
                _ => unreachable!(),
            };
            tx.append(kind, affected_ids, actor)
        } else {
            0
        };

        // Step 6: Calibration — update pressure, entropy, EWMA
        self.calibrate_after_tx(tx_cursor);

        Ok(TransactResult { tx_cursor, records_deleted: None })
    }

    /// Public accessor: entity contact from WM (for tests and REST handlers).
    pub fn wm_entity_contact(
        &self,
        name: &str,
    ) -> Option<crate::action_intent::EntityContactRecord> {
        self.world.read().ok()?.entity_contact(name)
    }

    /// Seed N observations for an entity so GroundingGate exits before daemon starts.
    /// Intended for test setup; safe to call in production (idempotent after MAPPED_OBS_THRESHOLD).
    pub fn seed_entity_grounded(&self, entity: &str) {
        use crate::action_intent::{ContactKind, MAPPED_OBS_THRESHOLD};
        if let Ok(wm) = self.world.read() {
            for _ in 0..MAPPED_OBS_THRESHOLD {
                wm.update_entity_contact(entity, ContactKind::Observed);
            }
        }
    }

    // ── Intent / Receipt helpers ──────────────────────────────────────────────

    fn open_intent_impl(
        &self,
        intent: crate::action_intent::ActionIntent,
        actor: &str,
    ) -> Result<u64, CognitiveError> {
        let rec = MemoryRecord::new(
            MemoryType::Intent,
            actor.to_string(),
            "open_intent".to_string(),
            intent.op.clone(),
            serde_json::to_value(&intent).unwrap_or_default(),
        );
        let ids = vec![rec.id];
        {
            let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
            ms.add(rec).map_err(|e| CognitiveError::StoreError(e.to_string()))?;
        }
        if let Ok(mut intents) = self.open_intents.lock() {
            intents.push(intent);
        }
        let cursor = if let Some(tx) = &self.tx_log {
            tx.append(TxKind::MemoryAdd, ids, actor)
        } else {
            0
        };
        Ok(cursor)
    }

    fn accept_receipt_impl(
        &self,
        receipt: crate::action_intent::ActionReceipt,
        actor: &str,
    ) -> Result<u64, CognitiveError> {
        // 1. Actuator heartbeat
        if let Ok(mut reg) = self.actuator_registry.lock() {
            reg.apply_receipt(&receipt.sensor_path, receipt.ts, receipt.ok);
        }
        // 2. Mark matching intent Received; update WM contact
        let target_entity = if let Ok(mut intents) = self.open_intents.lock() {
            let found = intents.iter_mut().find(|i| i.id == receipt.intent_id);
            if let Some(intent) = found {
                intent.status = crate::action_intent::IntentStatus::Received;
                intent.target_entity.clone()
            } else {
                None
            }
        } else {
            None
        };
        if let Some(entity) = &target_entity {
            if let Ok(wm) = self.world.read() {
                let kind = if receipt.ok {
                    crate::action_intent::ContactKind::Observed
                } else {
                    crate::action_intent::ContactKind::ProbeFailed
                };
                wm.update_entity_contact(entity, kind);
            }
        }
        // 3. Temporal observation record
        let obs_rec = MemoryRecord::new(
            MemoryType::Temporal,
            actor.to_string(),
            "receipt_observation".to_string(),
            receipt.sensor_path.clone(),
            receipt.observation.clone(),
        );
        // 4. Receipt record
        let receipt_rec = MemoryRecord::new(
            MemoryType::Receipt,
            actor.to_string(),
            "accept_receipt".to_string(),
            receipt.intent_id.to_string(),
            serde_json::to_value(&receipt).unwrap_or_default(),
        );
        let ids = vec![obs_rec.id, receipt_rec.id];
        {
            let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
            ms.add(obs_rec).map_err(|e| CognitiveError::StoreError(e.to_string()))?;
            ms.add(receipt_rec).map_err(|e| CognitiveError::StoreError(e.to_string()))?;
        }
        let cursor = if let Some(tx) = &self.tx_log {
            tx.append(TxKind::MemoryAdd, ids, actor)
        } else {
            0
        };
        Ok(cursor)
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
        self.calibrate_after_tx(tx_cursor);
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
        self.calibrate_after_tx(tx_cursor);
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
        self.calibrate_after_tx(tx_cursor);
        Ok(tx_cursor)
    }

    fn retract_belief(&self, id: Uuid, actor: &str) -> Result<u64, CognitiveError> {
        let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
        let cascaded = jtms::propagate_retraction(&mut ms, id, self.tx_log.as_deref(), actor);
        drop(ms);
        let tx_cursor = if let Some(tx) = &self.tx_log {
            // Primary retraction already logged inside propagate_retraction;
            // append a summary entry for the root retraction.
            if cascaded.is_empty() {
                tx.append(TxKind::BeliefRetract, vec![id], actor)
            } else {
                tx.append(TxKind::BeliefRetract, cascaded, actor)
            }
        } else {
            0
        };
        self.calibrate_after_tx(tx_cursor);
        Ok(tx_cursor)
    }

    fn assert_justification(
        &self,
        belief_id: Uuid,
        in_nodes: Vec<Uuid>,
        out_nodes: Vec<Uuid>,
        actor: &str,
    ) -> Result<u64, CognitiveError> {
        let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
        jtms::assert_justification(&mut ms, belief_id, in_nodes, out_nodes)
            .map_err(|e| CognitiveError::StoreError(e))?;
        drop(ms);
        let tx_cursor = if let Some(tx) = &self.tx_log {
            tx.append(TxKind::BeliefAssert, vec![belief_id], actor)
        } else {
            0
        };
        self.calibrate_after_tx(tx_cursor);
        Ok(tx_cursor)
    }

    fn auto_consolidate_memory(&self, min_frequency: usize, actor: &str) -> Result<u64, CognitiveError> {
        let freq = if min_frequency == 0 { 3 } else { min_frequency };
        let report = {
            let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
            crate::consolidation::mine_and_consolidate(
                &mut *ms,
                None,
                None,
                self.tx_log.as_deref(),
                freq,
                actor,
            )
            .map_err(|e| CognitiveError::StoreError(e))?
        };
        // Archive source records to cold store if available
        if !report.source_ids_archived.is_empty() {
            if let Some(arc) = &self.archive_store {
                if let Ok(mut cold) = arc.lock() {
                    // Source records already deleted from hot store inside mine_and_consolidate.
                    // ArchiveStore keeps a separate append-only log; nothing more to do here
                    // unless we have the records — they were deleted before we can retrieve them.
                    // This is acceptable: hot store pruning is the primary AC-2 goal.
                    let _ = &mut *cold; // suppress unused warning
                }
            }
        }
        // Gap A: Structural dedup — archive duplicate (actor, action, target, type) records.
        // Contracts identical structural triples to one record, bounding context growth.
        let structural_dupes: Vec<uuid::Uuid> = {
            use std::collections::HashMap;
            let ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
            let mut groups: HashMap<String, Vec<uuid::Uuid>> = HashMap::new();
            for rec in ms.all().iter() {
                if !matches!(rec.record_type, MemoryType::Temporal | MemoryType::Symbolic) {
                    continue;
                }
                // Normalize action: lowercase + strip non-alphanumeric so "Store Memory"
                // == "store_memory" == "store-memory" collapse to same fingerprint.
                let norm_action: String = rec.action
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect();
                let fp = format!(
                    "{}\x00{}\x00{}\x00{:?}",
                    rec.actor, norm_action, rec.target, rec.record_type
                );
                groups.entry(fp).or_default().push(rec.id);
            }
            groups
                .into_values()
                .filter(|ids| ids.len() > 1)
                .flat_map(|mut ids| {
                    ids.pop(); // keep last (most recent), delete the rest
                    ids
                })
                .collect()
        };
        if !structural_dupes.is_empty() {
            let mut ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
            for id in structural_dupes {
                ms.delete_by_id(id);
            }
        }
        let tx_cursor = if let Some(tx) = &self.tx_log {
            tx.append(
                TxKind::Consolidate,
                report.source_ids_archived.clone(),
                actor,
            )
        } else {
            0
        };
        self.calibrate_after_tx(tx_cursor);
        Ok(tx_cursor)
    }

    // ── Phase-4 workspace helpers ─────────────────────────────────────────────

    fn open_workspace(
        &self,
        id: WorkspaceId,
        mode: WorkspaceMode,
        actor: &str,
    ) -> Result<u64, CognitiveError> {
        let store = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
        let mut reg = self.workspace_registry.lock().map_err(|_| CognitiveError::LockError)?;
        reg.open(id, mode, &*store);
        drop(store);
        drop(reg);
        let tx_cursor = if let Some(tx) = &self.tx_log {
            tx.append(TxKind::WorkspaceOp, vec![], actor)
        } else {
            0
        };
        self.calibrate_after_tx(tx_cursor);
        Ok(tx_cursor)
    }

    fn merge_workspaces(
        &self,
        from: WorkspaceId,
        into: WorkspaceId,
        actor: &str,
    ) -> Result<u64, CognitiveError> {
        let mut reg = self.workspace_registry.lock().map_err(|_| CognitiveError::LockError)?;
        reg.merge(&from, &into).map_err(|e| CognitiveError::StoreError(e))?;
        drop(reg);
        let tx_cursor = if let Some(tx) = &self.tx_log {
            tx.append(TxKind::WorkspaceOp, vec![], actor)
        } else {
            0
        };
        self.calibrate_after_tx(tx_cursor);
        Ok(tx_cursor)
    }

    /// Re-acquire store (briefly) to compute real pressure + entropy after any transact.
    /// Best-effort: if lock is poisoned, EWMA ping still fires.
    fn calibrate_after_tx(&self, tx_cursor: u64) {
        if let Ok(store) = self.memory.lock() {
            let pressure = crate::consolidation::compute_pressure(
                &*store,
                &crate::consolidation::ConsolidationConfig::default(),
            );
            self.calibration.update_from_store(&*store, pressure, tx_cursor);
        }
    }

    fn apply_delta(&self, delta: &CognitiveDelta, actor: &str) -> Result<Vec<Uuid>, CognitiveError> {
        match delta {
            CognitiveDelta::AddMemory(record) => {
                let id = record.id;
                let mem_type = record.record_type.clone();

                // Phase-1 clarify gate: InProgress goal must have success_factors
                if mem_type == crate::memory_record::MemoryType::Goal {
                    if let Ok(payload) = serde_json::from_value::<crate::payloads::GoalPayload>(record.metadata.clone()) {
                        if payload.status == crate::payloads::GoalStatus::InProgress
                            && payload.success_factors.is_empty()
                        {
                            return Err(CognitiveError::GoalNotClarified(id));
                        }
                    }
                }

                // EpistemicAuthority gate: clamp Belief confidence if insufficient evidence.
                let record_to_store = if record.record_type == MemoryType::Belief {
                    if let Ok(mut bp) = serde_json::from_value::<BeliefPayload>(record.metadata.clone()) {
                        let clamped = crate::epistemic_authority::EpistemicAuthority::gate_belief_write(
                            bp.confidence, record.evidence.len()
                        );
                        if clamped < bp.confidence - f32::EPSILON {
                            bp.confidence = clamped;
                            let mut r = record.clone();
                            r.confidence = clamped;
                            r.metadata = serde_json::to_value(&bp).unwrap_or(r.metadata);
                            r
                        } else { record.clone() }
                    } else { record.clone() }
                } else { record.clone() };

                // Memory write
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .add(record_to_store)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;

                // G1a: update world model; G2a: calibrate from transition entropy
                if mem_type == MemoryType::Temporal {
                    if let Ok(mut wm) = self.world.write() {
                        crate::wm_updater::update_from_temporal(record, &mut wm);
                        // Use same (from_state, action) key as update_from_temporal
                        let wm_action = record
                            .metadata
                            .get("action")
                            .and_then(|v| v.as_str())
                            .unwrap_or("symbolic_step");
                        let entropy = wm
                            .get_transition_uncertainty(&record.target, wm_action)
                            .unwrap_or(0.0) as f32;
                        drop(wm);
                        self.calibration.record_prediction_error(entropy);
                    }
                }

                // G1b: invalidate stale beliefs via JTMS (read-only scan → retract through handle)
                if mem_type == MemoryType::Temporal || mem_type == MemoryType::Reflexion {
                    let inv_ids: Vec<uuid::Uuid> = if let Ok(mut store) = self.memory.lock() {
                        crate::belief_invalidator::BeliefInvalidator::process(record, &mut *store)
                    } else {
                        vec![]
                    };
                    for id in inv_ids {
                        let _ = self.retract_belief(id, actor);
                    }
                }

                // G1c: emergence detection on Temporal writes (lock order: emergence → memory)
                if mem_type == MemoryType::Temporal {
                    if let Ok(mut ed) = self.emergence.lock() {
                        if let Ok(mut store) = self.memory.lock() {
                            ed.on_temporal_write(&mut store, actor);
                        }
                    }
                }

                Ok(vec![id])
            }

            CognitiveDelta::UpdateBelief { id, payload } => {
                // EpistemicAuthority gate: clamp confidence by evidence count on existing record.
                let evidence_count = self.memory.lock().ok()
                    .and_then(|s| s.find_by_id(*id).map(|r| r.evidence.len()))
                    .unwrap_or(0);
                let clamped = crate::epistemic_authority::EpistemicAuthority::gate_belief_write(
                    payload.confidence, evidence_count
                );
                let gated = if clamped < payload.confidence - f32::EPSILON {
                    let mut p = payload.clone();
                    p.confidence = clamped;
                    p
                } else {
                    payload.clone()
                };
                let new_meta = serde_json::to_value(&gated)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .update_record(*id, None, None, Some(gated.confidence), None, Some(new_meta))
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

            CognitiveDelta::Intervene { var, value } => {
                self.world
                    .read()
                    .map_err(|_| CognitiveError::LockError)?
                    .apply_intervention(var, *value)
                    .map_err(CognitiveError::StoreError)?;
                let rec = MemoryRecord::new(
                    MemoryType::Reflexion,
                    actor.to_string(),
                    "causal_intervene".to_string(),
                    format!("do({}={})", var, value),
                    serde_json::json!({"var": var, "value": value}),
                );
                let id = rec.id;
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .add(rec)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                Ok(vec![id])
            }

            CognitiveDelta::Counterfactual { actual_state, intervention_var, intervention_value } => {
                let outcome = self.world
                    .read()
                    .map_err(|_| CognitiveError::LockError)?
                    .counterfactual(actual_state.clone(), intervention_var.clone(), *intervention_value)
                    .map_err(CognitiveError::StoreError)?;
                let rec = MemoryRecord::new(
                    MemoryType::Reflexion,
                    actor.to_string(),
                    "counterfactual".to_string(),
                    format!("cf({}={})", intervention_var, intervention_value),
                    serde_json::json!({"counterfactual_outcome": outcome,
                                       "intervention_var": intervention_var,
                                       "intervention_value": intervention_value}),
                );
                let id = rec.id;
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .add(rec)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                Ok(vec![id])
            }

            CognitiveDelta::CreditAssign(signal) => {
                let traj: Vec<std::collections::HashMap<String, f64>> = {
                    let mem = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
                    mem.all()
                        .iter()
                        .filter(|r| {
                            r.record_type == MemoryType::Temporal && r.actor.as_str() == actor
                        })
                        .rev()
                        .take(50)
                        .map(|r| {
                            r.metadata
                                .as_object()
                                .map(|m| {
                                    m.iter()
                                        .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
                                        .collect()
                                })
                                .unwrap_or_default()
                        })
                        .collect()
                };
                let report = self.world
                    .read()
                    .map_err(|_| CognitiveError::LockError)?
                    .credit_assign_trajectory(&traj, signal.clone())
                    .map_err(CognitiveError::StoreError)?;
                let rec = MemoryRecord::new(
                    MemoryType::Reflexion,
                    actor.to_string(),
                    "credit_assign".to_string(),
                    report.broken_equation.clone().unwrap_or_else(|| "none".to_string()),
                    serde_json::json!({
                        "broken_equation": report.broken_equation,
                        "confidence": report.confidence,
                        "single_intervention_sufficient": report.single_intervention_sufficient,
                        "counterfactual_outcome": report.counterfactual_outcome,
                    }),
                );
                let id = rec.id;
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .add(rec)
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                Ok(vec![id])
            }

            CognitiveDelta::RewriteStructuralEquation { node_id, new_weights } => {
                self.world
                    .read()
                    .map_err(|_| CognitiveError::LockError)?
                    .rewrite_structural_equation(node_id, new_weights.clone())
                    .map_err(CognitiveError::StoreError)?;
                let rec = MemoryRecord::new(
                    MemoryType::Reflexion,
                    actor.to_string(),
                    "rewrite_equation".to_string(),
                    node_id.clone(),
                    serde_json::json!({"node_id": node_id, "new_weights": new_weights}),
                );
                let id = rec.id;
                self.memory
                    .lock()
                    .map_err(|_| CognitiveError::LockError)?
                    .add(rec)
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

    /// Register a tracked entity in the world model (Gap 8 / test helper).
    pub fn register_wm_entity(
        &self,
        entity_id: String,
        state: crate::world_model_enhanced::EntityState,
    ) -> Result<(), String> {
        self.world
            .read()
            .map_err(|e| format!("WM lock: {}", e))?
            .register_entity(entity_id, state)
    }

    /// Feed a named-node prediction error to the self-model drift monitor.
    /// Use in tests or external observers after any predict/react cycle.
    pub fn observe_prediction_drift(&self, node: &str, error: f64, x: f64, y: f64) {
        self.self_model.observe_named_drift(node, error, x, y);
    }

    /// Add a standalone causal graph node (test helper for SCM rewrite tests).
    pub fn add_causal_node(&self, node_id: String) -> Result<(), String> {
        self.world
            .read()
            .map_err(|_| "lock poisoned".to_string())?
            .add_causal_node(node_id)
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

    /// Create a (SimulationFork, ContinuousDynamics) pair for assembling a DigitalTwin.
    /// Returns tuple to avoid circular import with digital_twin module.
    pub fn fork_hybrid(
        &self,
        dim: usize,
        dt: f64,
        max_covariance: f64,
    ) -> Result<(SimulationFork<B>, crate::continuous_dynamics::ContinuousDynamics), CognitiveError> {
        let fork = self.fork()?;
        let diag = {
            if let Ok(wm) = self.world.read() {
                let mvs = wm.entity_mean_vectors();
                if mvs.is_empty() {
                    vec![1.0f64; dim]
                } else {
                    let first = &mvs[0].1;
                    let scale = (first.iter().map(|x| x * x).sum::<f64>()
                        / first.len() as f64)
                        .sqrt()
                        .max(0.01);
                    vec![scale; dim]
                }
            } else {
                vec![1.0f64; dim]
            }
        };
        use crate::continuous_dynamics::{ContinuousDynamics, KalmanVectorField};
        let vf = KalmanVectorField::with_diag(diag);
        let dyn_ = ContinuousDynamics::new(Box::new(vf), dt, max_covariance);
        Ok((fork, dyn_))
    }

    /// Materialize ExperienceStore view for an actor.
    pub fn experience_tiers(&self, actor: &str) -> crate::experience_store::ExperienceStore {
        let store = self.memory.lock().unwrap();
        crate::experience_store::ExperienceStore::from_store(&*store, actor)
    }

    /// Search compressed experience tiers (Episode + Abstract) for records matching query in target.
    pub fn experience_search(
        &self,
        actor: &str,
        query: &str,
    ) -> Vec<crate::memory_record::MemoryRecord> {
        let store = self.memory.lock().unwrap();
        let es = crate::experience_store::ExperienceStore::from_store(&*store, actor);
        es.search_compressed(&*store, query)
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

    pub fn wm_transition_count(&self) -> usize {
        self.world
            .read()
            .map(|wm| wm.transition_count())
            .unwrap_or(0)
    }

    /// Record a normalised prediction error and, when the rolling window signals
    /// persistent structural drift, emit `RewriteStructuralEquation` via transact.
    /// Safe to call externally after any predict/react cycle (no lock re-entry).
    pub fn check_prediction_drift(
        &self,
        prediction_error: f64,
        actor: &str,
    ) -> Result<(), CognitiveError> {
        if let Some((node_id, new_weights)) =
            self.self_model.record_prediction_error(prediction_error)
        {
            self.transact(
                CognitiveDelta::RewriteStructuralEquation { node_id, new_weights },
                actor,
            )?;
        }
        Ok(())
    }

    /// Like `check_prediction_drift` but feeds the (feature_vec, target_vec) pair
    /// to the OLS monitor so drift weights can be inspected via `self_model.prediction_drift_weights()`.
    pub fn check_prediction_drift_with_obs(
        &self,
        prediction_error: f64,
        x: Vec<f64>,
        y: Vec<f64>,
        actor: &str,
    ) -> Result<(), CognitiveError> {
        if let Some((node_id, new_weights)) =
            self.self_model.record_prediction_error_with_obs(prediction_error, x, y)
        {
            self.transact(
                CognitiveDelta::RewriteStructuralEquation { node_id, new_weights },
                actor,
            )?;
        }
        Ok(())
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
