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
    /// Goal distance [0, 1]: fraction of goal weight not yet satisfied. 1.0 = no goal or no progress.
    #[serde(default = "one_f32")]
    pub goal_distance: f32,
    /// True if goal distance increased for 2+ consecutive steps at this point.
    #[serde(default)]
    pub drift_detected: bool,
}

fn one_f32() -> f32 { 1.0 }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RolloutResult {
    pub steps: Vec<RolloutStep>,
    pub final_fork_tx: u64,
    pub halted_early: bool,
    pub halt_reason: Option<String>,
    /// True if goal-distance drift was detected during rollout.
    #[serde(default)]
    pub drift_alarm: bool,
    /// Step index at which drift was first detected.
    #[serde(default)]
    pub drift_at_step: Option<u32>,
}

/// Extended rollout result combining discrete steps with continuous trajectory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridRolloutResult {
    /// Discrete simulation result.
    pub base: RolloutResult,
    /// One state vector per discrete step from the continuous integrator.
    pub continuous_trajectory: Vec<Vec<f64>>,
    /// True if continuous dynamics halted due to max_covariance being exceeded.
    pub continuous_halted: bool,
    /// Final sigma_norm at end of rollout.
    pub continuous_sigma_norm: f64,
    /// Causal node IDs active during rollout, for provenance tracking.
    pub causal_nodes: Option<Vec<String>>,
}

/// Returns true if the rollout result has a drift alarm AND the final goal distance exceeds `threshold`.
pub fn drift_gate(result: &RolloutResult, threshold: f32) -> bool {
    if !result.drift_alarm {
        return false;
    }
    result.steps.last().map(|s| s.goal_distance > threshold).unwrap_or(false)
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
/// Fraction of current variance added as process noise each step: P_{k+1} = P_k + max(α*P_k, Q_floor).
/// This is the diagonal discrete Lyapunov recursion for a mildly unstable system.
const KALMAN_Q_ALPHA: f32 = 0.1;
/// Number of consecutive goal-distance increases that trigger a drift alarm.
const DRIFT_CONSECUTIVE_THRESHOLD: usize = 2;

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
    ///
    /// Kalman covariance: P_{k+1}[entity][dim] = P_k[entity][dim] + max(α*P_k[dim], NOISE_FLOOR)
    /// — entity-specific proportional process noise (diagonal discrete Lyapunov).
    ///
    /// Goal drift: auto-finds any active Goal in fork store, computes goal_distance per step
    /// (fraction of total goal weight that is unsatisfied). DRIFT_CONSECUTIVE_THRESHOLD
    /// consecutive increases trigger drift_alarm.
    ///
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

        // Pre-scan fork store for the first active Goal to track drift against.
        let goal_factors: Vec<(f32, bool)> = {
            let all = self.store.all();
            all.iter()
                .find(|r| r.record_type == MemoryType::Goal && r.status == "active")
                .and_then(|r| serde_json::from_value::<GoalPayload>(r.metadata.clone()).ok())
                .map(|p| {
                    p.success_factors
                        .iter()
                        .map(|f| (f.weight, f.satisfied))
                        .collect()
                })
                .unwrap_or_default()
        };
        let total_goal_weight: f32 = goal_factors.iter().map(|(w, _)| w).sum();

        let mut steps = Vec::new();
        let mut halted_early = false;
        let mut halt_reason: Option<String> = None;
        let mut prev_goal_distance: Option<f32> = None;
        let mut drift_streak: usize = 0;
        let mut drift_alarm = false;
        let mut drift_at_step: Option<u32> = None;

        for (idx, action) in actions.iter().enumerate() {
            let fork_tx = self.step(action)?;

            // Propagate diagonal Kalman: P_{k+1}[i] = P_k[i] + max(α*P_k[i], NOISE_FLOOR)
            for variances in self.uncertainty.values_mut() {
                for v in variances.iter_mut() {
                    *v += (*v * KALMAN_Q_ALPHA).max(NOISE_FLOOR);
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

            // Goal-distance: fraction of total weight that is unsatisfied
            let goal_distance = if total_goal_weight > 0.0 {
                let unsatisfied: f32 = goal_factors
                    .iter()
                    .filter(|(_, sat)| !sat)
                    .map(|(w, _)| w)
                    .sum();
                (unsatisfied / total_goal_weight).clamp(0.0, 1.0)
            } else {
                1.0 // no goal in store = maximum distance
            };

            // Drift detection: track consecutive increases
            let step_drift = if let Some(prev) = prev_goal_distance {
                if goal_distance > prev {
                    drift_streak += 1;
                } else {
                    drift_streak = 0;
                }
                if drift_streak >= DRIFT_CONSECUTIVE_THRESHOLD && !drift_alarm {
                    drift_alarm = true;
                    drift_at_step = Some(idx as u32);
                }
                drift_alarm
            } else {
                false
            };
            prev_goal_distance = Some(goal_distance);

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
                goal_distance,
                drift_detected: step_drift,
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
        Ok(RolloutResult { steps, final_fork_tx, halted_early, halt_reason, drift_alarm, drift_at_step })
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

    /// Expose fork's internal store records for sync-back by DigitalTwin.
    pub fn all_records(&self) -> Vec<crate::memory_record::MemoryRecord> {
        self.store.all().to_vec()
    }

    /// Hybrid rollout: discrete steps interleaved with continuous RK4 integration.
    pub fn rollout_hybrid(
        &mut self,
        actions: Vec<String>,
        sigma2_max: f32,
        dynamics: Option<crate::continuous_dynamics::ContinuousDynamics>,
        causal_nodes: Option<Vec<String>>,
    ) -> Result<HybridRolloutResult, CognitiveError> {
        use crate::continuous_dynamics::DynamicsContext;
        let mut dyn_ = dynamics;
        let base = self.rollout(actions, sigma2_max)?;

        let dim = dyn_.as_ref().map(|d| d.dim()).unwrap_or(0);
        let mut cont_state: Vec<f64> = vec![0.0; dim];
        let mut trajectory: Vec<Vec<f64>> = Vec::new();
        let mut continuous_halted = false;

        if let Some(ref mut d) = dyn_ {
            let ctx = DynamicsContext {
                entity_states: &[],
                resource_vec: &[],
                tx_cursor: self.tx_log.current_tx(),
            };
            for (i, _step) in base.steps.iter().enumerate() {
                match d.step(i as f64 * d.dt, &cont_state, &ctx) {
                    Ok(ns) => {
                        cont_state = ns.clone();
                        trajectory.push(ns);
                    }
                    Err(_) => {
                        trajectory.push(cont_state.clone());
                        continuous_halted = true;
                    }
                }
            }
            continuous_halted = continuous_halted || d.is_halted();
        }

        let continuous_sigma_norm = dyn_.as_ref().map(|d| d.sigma_norm()).unwrap_or(0.0);

        Ok(HybridRolloutResult {
            base,
            continuous_trajectory: trajectory,
            continuous_halted,
            continuous_sigma_norm,
            causal_nodes,
        })
    }
}
