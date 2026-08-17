//! Phase 2+3: Copy-on-Write SimulationFork with bounded Kalman rollout.

use crate::cognitive_state::{
    BeliefDistribution, BeliefSummary, CognitiveDelta, CognitiveError, CognitiveSnapshot,
    GoalSnapshot, ProvenanceSummary, SelfStateView, SkillSnapshot, TemporalView, WorldStateView,
};
use crate::memory_record::MemoryType;
use crate::memory_store::MemoryStore;
use crate::payloads::{BeliefPayload, GoalPayload, SkillPayload};
use crate::persistence::{InMemoryBackend, MemoryBackend};
use crate::tx_log::{TxKind, TxLog};
use std::collections::HashMap;
use uuid::Uuid;

// ── Phase-3 rollout types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RolloutStep {
    pub step_index: u32,
    pub action: String,
    /// Per-entity uncertainty (entity_id → max variance across dims)
    pub uncertainty: HashMap<String, f32>,
    /// True if uncertainty halted rollout after this step
    pub halted: bool,
    pub fork_tx: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RolloutResult {
    pub steps: Vec<RolloutStep>,
    pub final_fork_tx: u64,
    pub halted_early: bool,
    pub halt_reason: Option<String>,
}

// ── SimulationFork ────────────────────────────────────────────────────────────

pub struct SimulationFork<B: MemoryBackend + Send + Sync + 'static> {
    pub id: Uuid,
    pub base_tx: u64,
    pub created_at: std::time::Instant,
    store: MemoryStore<InMemoryBackend>,
    tx_log: TxLog,
    steps: Vec<String>,
    /// Per-entity per-dimension variances — seeded from parent's WM at fork time.
    uncertainty: HashMap<String, Vec<f32>>,
    _marker: std::marker::PhantomData<B>,
}

const NOISE_FLOOR: f32 = 0.01;
const ROLLOUT_K_CAP: usize = 5;

impl<B: MemoryBackend + Send + Sync + 'static> SimulationFork<B> {
    /// Copy parent's live records into an isolated in-memory store.
    /// Seeds per-entity uncertainty from parent's WorldModelEnhanced.
    pub fn from_handle(
        handle: &crate::cognitive_state::CognitiveHandle<B>,
        base_tx: u64,
    ) -> Result<Self, CognitiveError> {
        let records = {
            let ms = handle.memory.lock().map_err(|_| CognitiveError::LockError)?;
            ms.all().to_vec()
        };
        let mut fork_store = MemoryStore::<InMemoryBackend>::new_in_memory();
        for r in records {
            fork_store
                .add(r)
                .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
        }
        let tmp = std::env::temp_dir().join(format!("hc-fork-{}.jsonl", Uuid::new_v4()));
        let tx_log = TxLog::open(tmp).map_err(CognitiveError::StoreError)?;

        // Seed uncertainty from parent WM entities; fallback to synthetic "world" entity
        let uncertainty = if let Ok(wm) = handle.world.read() {
            let diags = wm.entity_covariance_diagonals();
            if diags.is_empty() {
                let mut m = HashMap::new();
                m.insert("world".to_string(), vec![NOISE_FLOOR; 3]);
                m
            } else {
                diags
            }
        } else {
            let mut m = HashMap::new();
            m.insert("world".to_string(), vec![NOISE_FLOOR; 3]);
            m
        };

        Ok(Self {
            id: Uuid::new_v4(),
            base_tx,
            created_at: std::time::Instant::now(),
            store: fork_store,
            tx_log,
            steps: Vec::new(),
            uncertainty,
            _marker: std::marker::PhantomData,
        })
    }

    /// Record one action string in the fork's local tx log.
    pub fn step(&mut self, action: &str) -> Result<u64, CognitiveError> {
        if action.is_empty() {
            return Err(CognitiveError::DeltaInvalid("action required".into()));
        }
        self.steps.push(action.to_string());
        Ok(self.tx_log.append(TxKind::WorldModelObserve, vec![], "fork"))
    }

    /// Apply a CognitiveDelta to the fork's isolated store (AddMemory only in Phase 2+3).
    pub fn apply_delta(
        &mut self,
        delta: CognitiveDelta,
        actor: &str,
    ) -> Result<u64, CognitiveError> {
        match &delta {
            CognitiveDelta::AddMemory(r) => {
                self.store
                    .add(r.clone())
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
            }
            _ => {
                return Err(CognitiveError::NotImplemented(format!(
                    "fork apply_delta {}",
                    delta.label()
                )));
            }
        }
        Ok(self.tx_log.append(TxKind::MemoryAdd, vec![], actor))
    }

    /// Execute up to `actions.len()` steps (capped at ROLLOUT_K_CAP=5).
    /// Propagates diagonal Kalman uncertainty per step: P_{k+1}[i] = P_k[i] + noise_floor.
    /// Halts early if max variance across all entities exceeds sigma2_max.
    pub fn rollout(
        &mut self,
        actions: Vec<String>,
        sigma2_max: f32,
    ) -> Result<RolloutResult, CognitiveError> {
        if actions.is_empty() {
            return Err(CognitiveError::DeltaInvalid("actions must not be empty".into()));
        }
        let sigma2_max = sigma2_max.clamp(0.01, 1.0);
        let actions: Vec<String> = actions.into_iter().take(ROLLOUT_K_CAP).collect();

        let mut steps = Vec::new();
        let mut halted_early = false;
        let mut halt_reason: Option<String> = None;

        for (idx, action) in actions.iter().enumerate() {
            let fork_tx = self.step(action)?;

            // Propagate diagonal Kalman: P_{k+1}[i] = P_k[i] + noise_floor
            for variances in self.uncertainty.values_mut() {
                for v in variances.iter_mut() {
                    *v += NOISE_FLOOR;
                }
            }

            // Build per-entity uncertainty map (max variance per entity)
            let uncertainty_snapshot: HashMap<String, f32> = self
                .uncertainty
                .iter()
                .map(|(id, vars)| {
                    let max_var = vars.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    (id.clone(), max_var)
                })
                .collect();

            // Check halt condition
            let max_global = uncertainty_snapshot
                .values()
                .cloned()
                .fold(f32::NEG_INFINITY, f32::max);
            let halted = max_global > sigma2_max;

            steps.push(RolloutStep {
                step_index: idx as u32,
                action: action.clone(),
                uncertainty: uncertainty_snapshot,
                halted,
                fork_tx,
            });

            if halted {
                halted_early = true;
                halt_reason = Some(format!(
                    "uncertainty exceeded sigma2_max={sigma2_max} after step {idx}"
                ));
                break;
            }
        }

        let final_fork_tx = self.tx_log.current_tx();
        Ok(RolloutResult { steps, final_fork_tx, halted_early, halt_reason })
    }

    /// Build CognitiveSnapshot from fork's local records.
    pub fn snapshot(&self, actor: &str) -> Result<CognitiveSnapshot, CognitiveError> {
        let all = self.store.all();

        let temporal_recs: Vec<_> = all
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
            recent_actions: temporal_recs
                .iter()
                .rev()
                .take(5)
                .map(|r| r.action.clone())
                .collect(),
            temporal_span_ms,
        };

        let goals: Vec<GoalSnapshot> = all
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Goal
                    && r.status == "active"
                    && (actor.is_empty() || r.actor == actor)
            })
            .filter_map(|r| {
                serde_json::from_value::<GoalPayload>(r.metadata.clone())
                    .ok()
                    .map(|p| GoalSnapshot {
                        id: r.id,
                        target_state: p.target_state,
                        status: p.status,
                        iteration: p.current_iteration,
                    })
            })
            .collect();

        let skills: Vec<SkillSnapshot> = all
            .iter()
            .filter(|r| r.record_type == MemoryType::Skill)
            .filter_map(|r| {
                serde_json::from_value::<SkillPayload>(r.metadata.clone())
                    .ok()
                    .map(|p| SkillSnapshot { id: r.id, procedure: p.procedure })
            })
            .collect();

        let belief_summaries: Vec<BeliefSummary> = all
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Belief
                    && r.status == "active"
                    && (actor.is_empty() || r.actor == actor)
            })
            .filter_map(|r| {
                serde_json::from_value::<BeliefPayload>(r.metadata.clone())
                    .ok()
                    .map(|p| BeliefSummary {
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
            epistemic_entropy: 0.0,
            beliefs: belief_summaries,
        };

        let provenance = ProvenanceSummary {
            merkle_root_hex: self.store.merkle_root_hex(),
            record_count: self.store.record_count(),
            evidence_edge_count: self.store.evidence_edge_count(),
        };

        Ok(CognitiveSnapshot {
            id: Uuid::new_v4(),
            tx_cursor: self.tx_log.current_tx(),
            actor: actor.to_string(),
            temporal,
            world: WorldStateView { node_count: 0, edge_count: 0, dag_verified: true },
            self_model: SelfStateView {
                calibration_score: 1.0,
                prediction_error_ewma: 0.0,
                consolidation_pressure: 0.0,
                epistemic_entropy: 0.0,
                healthy: true,
            },
            goals,
            skills,
            beliefs,
            provenance,
        })
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > 60
    }

    pub fn fork_tx(&self) -> u64 {
        self.tx_log.current_tx()
    }

    pub fn steps_taken(&self) -> usize {
        self.steps.len()
    }
}
