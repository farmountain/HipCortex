//! Phase-4: SubstrateDaemon — 8-stage cognitive loop.
//! Stages: Observe → Reflect → Plan → CriticVeto → Predict → Act → Update → ExitCheck.
//!
//! Chain-of-thought:
//!   subscribe_with_config(actor, cognitive, config) → spawn thread → run 8 stages per iteration.
//!   CriticVeto at iteration 0 always passes (CriticGate invariant).
//!   stop() sets AtomicBool the thread checks in ExitCheck.
//!   stage_counts[0..7] tracks how many times each stage completed.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use uuid::Uuid;

/// Configuration for the cognitive maintenance loop.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CognitiveLoopConfig {
    /// Seconds to sleep between iterations (0 for tests).
    pub interval_secs: u64,
    /// Consolidation pressure threshold (0.0–1.0); above this Act stage consolidates.
    pub pressure_threshold: f32,
    /// Minimum motif frequency for AutoConsolidate.
    pub min_consolidation_frequency: usize,
    /// Iteration cap; None = run indefinitely.
    pub max_iterations: Option<u32>,
}

impl Default for CognitiveLoopConfig {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            pressure_threshold: 0.7,
            min_consolidation_frequency: 3,
            max_iterations: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DaemonStatus {
    Running,
    Stopped,
}

/// Live view of a daemon handle — returned by `status()` and serialised for REST.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HandleInfo {
    pub id: Uuid,
    pub actor: String,
    pub started_at: SystemTime,
    pub iterations: u32,
    pub status: DaemonStatus,
    /// Per-stage completion counts (indices 0–7: Observe..ExitCheck).
    pub stage_counts: Vec<u32>,
}

struct HandleState {
    id: Uuid,
    actor: String,
    started_at: SystemTime,
    iterations: Arc<AtomicU32>,
    stopped: Arc<std::sync::atomic::AtomicBool>,
    stage_counts: Arc<Mutex<[u32; 8]>>,
}

/// Registry of all active daemon handles (held in `AppState` behind `Arc<Mutex<_>>`).
pub struct SubstrateDaemon {
    handles: HashMap<Uuid, HandleState>,
}

impl SubstrateDaemon {
    pub fn new() -> Self {
        Self { handles: HashMap::new() }
    }

    /// Spawn with default config (backward-compatible).
    pub fn subscribe<B>(
        &mut self,
        actor: String,
        cognitive: Arc<crate::cognitive_state::CognitiveHandle<B>>,
    ) -> Uuid
    where
        B: crate::persistence::MemoryBackend + Send + Sync + 'static,
    {
        self.subscribe_with_config(actor, cognitive, CognitiveLoopConfig::default())
    }

    /// Spawn a background 8-stage cognitive loop thread and return its handle ID.
    pub fn subscribe_with_config<B>(
        &mut self,
        actor: String,
        cognitive: Arc<crate::cognitive_state::CognitiveHandle<B>>,
        config: CognitiveLoopConfig,
    ) -> Uuid
    where
        B: crate::persistence::MemoryBackend + Send + Sync + 'static,
    {
        let id = Uuid::new_v4();
        let iterations = Arc::new(AtomicU32::new(0));
        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stage_counts = Arc::new(Mutex::new([0u32; 8]));

        let iter_clone = Arc::clone(&iterations);
        let stop_clone = Arc::clone(&stopped);
        let sc_clone = Arc::clone(&stage_counts);
        let actor_clone = actor.clone();

        std::thread::Builder::new()
            .name(format!("substrate-{}", &id.to_string()[..8]))
            .spawn(move || {
                let mut loop_iter = 0u32;
                let mut consecutive_veto_count = 0u32;
                loop {
                    // Stage 0: Observe — purge expired records, take snapshot
                    if let Ok(mut ms) = cognitive.memory.lock() {
                        ms.purge_expired();
                    }
                    let snapshot = cognitive.snapshot(&actor_clone);
                    if let Ok(mut sc) = sc_clone.lock() { sc[0] += 1; }
                    // SelfModel steers loop config: health → effective_veto_threshold + synthesis_mode.
                    let loop_cfg = cognitive.self_model.recommend_loop_config();
                    let _ = &consecutive_veto_count; // suppress unused-variable warning when no goal active

                    // Stage 1: Reflect — assess pressure; find highest-priority InProgress goal (Gap 1)
                    let pressure = snapshot
                        .as_ref()
                        .map(|s| s.self_model.consolidation_pressure)
                        .unwrap_or(0.0);
                    // Find first InProgress goal with ≥1 success_factors (skip unclarified goals).
                    // Goals with empty factors bypass the GoalNotClarified gate if added via
                    // Pending→InProgress transition — daemon must not act on or mark them Succeeded.
                    let active_goal: Option<(uuid::Uuid, crate::payloads::GoalPayload)> =
                        cognitive.memory.lock().ok().and_then(|ms| {
                            ms.search_by_goal_status(&actor_clone, "InProgress")
                                .into_iter()
                                .find_map(|rec| {
                                    let payload: crate::payloads::GoalPayload =
                                        serde_json::from_value(rec.metadata.clone())
                                            .unwrap_or_default();
                                    if payload.success_factors.is_empty() {
                                        None // skip unclarified goal
                                    } else {
                                        Some((rec.id, payload))
                                    }
                                })
                        });
                    // Gap E: Autonomous goal synthesis — when no clarified InProgress goal,
                    // synthesize a new goal from WM entity uncertainty (daemon generates novel goals).
                    let active_goal = if active_goal.is_none() {
                        let uncertain = cognitive.world.read().ok()
                            .and_then(|wm| wm.most_uncertain_entity());
                        if let Some(ref entity_name) = uncertain {
                            let already_exists = cognitive.memory.lock().ok()
                                .map(|ms| {
                                    ms.all().iter().any(|r| {
                                        r.record_type == crate::memory_record::MemoryType::Goal
                                            && r.target == *entity_name
                                            && r.actor == actor_clone
                                    })
                                })
                                .unwrap_or(true);
                            if !already_exists {
                                use crate::payloads::{GoalPayload, GoalStatus, SuccessFactor};
                                let payload = GoalPayload {
                                    target_state: format!("stabilize_{}", entity_name),
                                    success_factors: vec![SuccessFactor {
                                        name: "entity_uncertainty_below_threshold".to_string(),
                                        weight: 1.0,
                                        satisfied: false,
                                    }],
                                    status: GoalStatus::InProgress,
                                    max_react_iterations: 3,
                                    ..Default::default()
                                };
                                if let Ok(meta) = serde_json::to_value(&payload) {
                                    let goal_rec = crate::memory_record::MemoryRecord::new(
                                        crate::memory_record::MemoryType::Goal,
                                        actor_clone.clone(),
                                        "synthesize".to_string(),
                                        entity_name.clone(),
                                        meta,
                                    );
                                    let goal_id = goal_rec.id;
                                    if let Ok(mut ms) = cognitive.memory.lock() {
                                        let _ = ms.add(goal_rec);
                                    }
                                    Some((goal_id, payload))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        active_goal
                    };
                    // Stage 1 clarify: run ClarifyEngine on any unclarified InProgress goal (P0-A).
                    if let Ok(mut ms) = cognitive.memory.lock() {
                        let unclarified: Vec<uuid::Uuid> = ms
                            .search_by_goal_status(&actor_clone, "InProgress")
                            .into_iter()
                            .filter_map(|rec| {
                                let p: crate::payloads::GoalPayload =
                                    serde_json::from_value(rec.metadata.clone()).unwrap_or_default();
                                if p.success_factors.is_empty() { Some(rec.id) } else { None }
                            })
                            .collect();
                        for gid in unclarified {
                            let _ = crate::clarify_engine::ClarifyEngine::run(
                                &mut ms, gid, &actor_clone,
                                crate::clarify_engine::ClarifyTrigger::EmptyAC,
                                None,
                            );
                        }
                    }
                    if let Ok(mut sc) = sc_clone.lock() { sc[1] += 1; }

                    // Stage 1b: OOD anomaly detection — Mahalanobis severity > threshold on the
                    // most uncertain entity → targeted CreditAssign("ood_shift:entity_id").
                    // Only the anomalous entity's causal equations are blamed; unrelated beliefs
                    // remain In. One CreditAssign per daemon tick to avoid log flooding.
                    if let Ok(wm) = cognitive.world.read() {
                        if let Some(ref entity_id) = wm.most_uncertain_entity() {
                            if let Ok(anomalies) = wm.get_entity_anomalies(entity_id) {
                                for anomaly in &anomalies {
                                    if anomaly.severity > anomaly.threshold {
                                        let signal = format!(
                                            "ood_shift:entity={entity_id} severity={:.2}",
                                            anomaly.severity
                                        );
                                        let _ = cognitive.transact(
                                            crate::cognitive_state::CognitiveDelta::CreditAssign(
                                                crate::world_model_enhanced::causal::FailureSignal::ExplicitFail(signal),
                                            ),
                                            &actor_clone,
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    // Stage 2: Plan — determine required actions
                    let should_consolidate = pressure > config.pressure_threshold;
                    if let Ok(mut sc) = sc_clone.lock() { sc[2] += 1; }

                    // Stage 3: CriticVeto — only against real active goals.
                    // Idle daemon (no goal) must not veto maintenance: dummy-goal always passes.
                    // When vetoed, writes Decision{rejected, rationale_chain} to memory.
                    let vetoed = if let Some((ref goal_id_s3, ref goal_payload)) = active_goal {
                        use crate::loop_gates::{CriticDecision, CriticGate};
                        match CriticGate::evaluate_with_threshold(goal_payload, "daemon_step", loop_iter, loop_cfg.effective_veto_threshold) {
                            CriticDecision::Rejected { rationale } => {
                                use crate::memory_record::{MemoryRecord, MemoryType};
                                use crate::payloads::DecisionPayload;
                                let dp = DecisionPayload {
                                    option_chosen: "rejected".to_string(),
                                    rationale_chain: rationale,
                                    confidence: 0.0,
                                    ..Default::default()
                                };
                                if let (Ok(mut ms), Ok(meta)) = (
                                    cognitive.memory.lock(),
                                    serde_json::to_value(&dp),
                                ) {
                                    let rec = MemoryRecord::new(
                                        MemoryType::Decision,
                                        actor_clone.clone(),
                                        "critic_veto".to_string(),
                                        format!("iter={loop_iter}"),
                                        meta,
                                    );
                                    let _ = ms.add(rec);
                                }
                                consecutive_veto_count += 1;
                                // Veto is a revision event: CreditAssign the failure (P0-C).
                                let _ = cognitive.transact(
                                    crate::cognitive_state::CognitiveDelta::CreditAssign(
                                        crate::world_model_enhanced::causal::FailureSignal::ExplicitFail(
                                            format!("critic_veto iter={loop_iter}"),
                                        ),
                                    ),
                                    &actor_clone,
                                );
                                // After ≥3 consecutive vetoes, run ClarifyEngine to self-resolve (P0-A).
                                if consecutive_veto_count >= 3 {
                                    if let Ok(mut ms) = cognitive.memory.lock() {
                                        let _ = crate::clarify_engine::ClarifyEngine::run(
                                            &mut ms, *goal_id_s3, &actor_clone,
                                            crate::clarify_engine::ClarifyTrigger::RepeatedVeto {
                                                veto_count: consecutive_veto_count,
                                            },
                                            None,
                                        );
                                    }
                                }
                                true
                            }
                            CriticDecision::Approved { .. } => {
                                consecutive_veto_count = 0;
                                false
                            }
                        }
                    } else {
                        false // idle daemon — no goal to veto against, maintenance always runs
                    };
                    if let Ok(mut sc) = sc_clone.lock() { sc[3] += 1; }

                    // Stage 4: Predict — WM prediction for next state (saved for verifier).
                    let prediction = cognitive
                        .world
                        .read()
                        .ok()
                        .and_then(|wm| wm.predict_next_state(&actor_clone, "daemon_step").ok());
                    if let Ok(mut sc) = sc_clone.lock() { sc[4] += 1; }

                    // Stage 5: Act — all acts gated by critic veto.
                    if !vetoed {
                        // AutoConsolidate when pressure warrants it.
                        if should_consolidate {
                            let _ = cognitive.transact(
                                crate::cognitive_state::CognitiveDelta::AutoConsolidate {
                                    min_frequency: config.min_consolidation_frequency,
                                },
                                &actor_clone,
                            );
                        }
                        // Drive active goal: FullCycle exhausts loop in one tick; StepByStep
                        // advances one ReAct iteration per tick, keeping goal InProgress across
                        // daemon ticks so CriticGate can veto at iter ≥ 1.
                        if let Some((goal_id, goal_payload)) = &active_goal {
                            use crate::loop_engine::ReactEngine;
                            use crate::payloads::GoalExecutionMode;
                            let mut engine = ReactEngine::new();
                            if let Ok(mut ms) = cognitive.memory.lock() {
                                match goal_payload.execution_mode {
                                    GoalExecutionMode::StepByStep => {
                                        let _ = engine.run_one_step(&mut ms, *goal_id, 0u32);
                                    }
                                    GoalExecutionMode::FullCycle => {
                                        let _ = engine.run(&mut ms, *goal_id, 0u32);
                                    }
                                }
                            }
                        }
                        // Stage 5→6: VerifierGate — if WM had a prediction but goal made no
                        // progress this tick, treat as verifier mismatch and fire CreditAssign.
                        // This makes prediction/observation discrepancy a revision event (not skipped tick).
                        if loop_iter > 0 {
                            if let (Some(ref predicted_state), Some((ref goal_id_v, ref pre_payload))) =
                                (&prediction, &active_goal)
                            {
                                let pre_satisfied =
                                    pre_payload.success_factors.iter().filter(|f| f.satisfied).count();
                                let post_satisfied = cognitive
                                    .memory
                                    .lock()
                                    .ok()
                                    .and_then(|ms| ms.find_by_id(*goal_id_v).cloned())
                                    .and_then(|r| {
                                        serde_json::from_value::<crate::payloads::GoalPayload>(
                                            r.metadata.clone(),
                                        )
                                        .ok()
                                    })
                                    .map(|p| p.success_factors.iter().filter(|f| f.satisfied).count())
                                    .unwrap_or(pre_satisfied);
                                // Mismatch: WM predicted something but no new factors satisfied.
                                let observed = if post_satisfied > pre_satisfied {
                                    "goal_advancing"
                                } else {
                                    "goal_stalling"
                                };
                                use crate::loop_gates::{VerifierGate, VerifierResult};
                                if let VerifierResult::Mismatch { predicted, observed: obs } =
                                    VerifierGate::check(Some("goal_advancing"), observed)
                                {
                                    let wm_top = predicted_state
                                        .probabilities
                                        .iter()
                                        .max_by(|a, b| {
                                            a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                                        })
                                        .map(|(s, _)| s.as_str())
                                        .unwrap_or("unknown");
                                    let _ = cognitive.transact(
                                        crate::cognitive_state::CognitiveDelta::CreditAssign(
                                            crate::world_model_enhanced::causal::FailureSignal::ExplicitFail(
                                                format!(
                                                    "verifier_mismatch predicted={predicted} observed={obs} wm_top={wm_top}",
                                                ),
                                            ),
                                        ),
                                        &actor_clone,
                                    );
                                    // If all success_factors already satisfied (pre-success): clarify.
                                    if pre_satisfied == pre_payload.success_factors.len()
                                        && !pre_payload.success_factors.is_empty()
                                    {
                                        if let Ok(mut ms) = cognitive.memory.lock() {
                                            let _ = crate::clarify_engine::ClarifyEngine::run(
                                                &mut ms,
                                                *goal_id_v,
                                                &actor_clone,
                                                crate::clarify_engine::ClarifyTrigger::PreSuccess,
                                                None,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Gap C: OLS-weight SCM rewrite — use computed coefficient, not placeholder.
                        if let Some((drifted_node, ols_weight)) =
                            cognitive.self_model.most_drifted_node_with_weight()
                        {
                            const OLS_DRIFT_THRESHOLD: f64 = 0.3;
                            if ols_weight > OLS_DRIFT_THRESHOLD {
                                let _ = cognitive.transact(
                                    crate::cognitive_state::CognitiveDelta::RewriteStructuralEquation {
                                        node_id: drifted_node,
                                        new_weights: vec![ols_weight],
                                    },
                                    &actor_clone,
                                );
                            }
                        }
                    }
                    if let Ok(mut sc) = sc_clone.lock() { sc[5] += 1; }

                    // Stage 6: Update — write daemon_step Temporal; snapshot WM entity state (Gap 8)
                    if let Ok(mut ms) = cognitive.memory.lock() {
                        use crate::memory_record::{MemoryRecord, MemoryType};
                        let rec = MemoryRecord::new(
                            MemoryType::Temporal,
                            actor_clone.clone(),
                            "daemon_step".to_string(),
                            format!("iter={loop_iter}"),
                            serde_json::json!({
                                "vetoed": vetoed,
                                "consolidated": !vetoed && should_consolidate,
                            }),
                        );
                        let _ = ms.add(rec);

                        // Gap 8: write one Temporal per WM entity (continuous dynamics bridge)
                        // Gap 4-verifier: annotate uncertain entity states — VerifierGate operates on
                        // strings and cannot block continuous Kalman writes, so we annotate instead.
                        if let Ok(wm) = cognitive.world.read() {
                            let most_uncertain = wm.most_uncertain_entity();
                            for (entity_name, mean_vec) in wm.entity_mean_vectors() {
                                let snap = MemoryRecord::new(
                                    MemoryType::Temporal,
                                    actor_clone.clone(),
                                    "wm_state_snapshot".to_string(),
                                    entity_name.clone(),
                                    serde_json::json!({ "mean": mean_vec }),
                                );
                                let _ = ms.add(snap);
                                // Annotate when this entity is the most uncertain (trace > 1.0).
                                if most_uncertain.as_deref() == Some(entity_name.as_str()) {
                                    let annot = MemoryRecord::new(
                                        MemoryType::Belief,
                                        actor_clone.clone(),
                                        "entity_state_uncertain".to_string(),
                                        entity_name.clone(),
                                        serde_json::json!({ "covariance_trace_above_threshold": true }),
                                    );
                                    let _ = ms.add(annot);
                                }
                            }
                        }
                    }
                    if let Ok(mut sc) = sc_clone.lock() { sc[6] += 1; }

                    // Stage 7: ExitCheck — if active goal fully satisfied, mark Succeeded (Gap 1)
                    if let Some((goal_id, _)) = &active_goal {
                        if let Ok(mut ms) = cognitive.memory.lock() {
                            if let Some(goal_rec) = ms.find_by_id(*goal_id) {
                                if let Ok(mut payload) =
                                    serde_json::from_value::<crate::payloads::GoalPayload>(
                                        goal_rec.metadata.clone(),
                                    )
                                {
                                    if payload.success_factors.iter().all(|f| f.satisfied)
                                        && !matches!(
                                            payload.status,
                                            crate::payloads::GoalStatus::Succeeded
                                        )
                                    {
                                        payload.status = crate::payloads::GoalStatus::Succeeded;
                                        if let Ok(v) = serde_json::to_value(&payload) {
                                            let _ = ms.update_record(
                                                *goal_id,
                                                None, None, None, None,
                                                Some(v),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    loop_iter += 1;
                    iter_clone.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut sc) = sc_clone.lock() { sc[7] += 1; }

                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if config.max_iterations.map(|m| loop_iter >= m).unwrap_or(false) {
                        break;
                    }
                    if config.interval_secs > 0 {
                        std::thread::sleep(std::time::Duration::from_secs(config.interval_secs));
                    }
                }
            })
            .expect("substrate thread spawn");

        self.handles.insert(
            id,
            HandleState {
                id,
                actor,
                started_at: SystemTime::now(),
                iterations,
                stopped,
                stage_counts,
            },
        );
        id
    }

    /// Get a snapshot of handle state.
    pub fn status(&self, id: Uuid) -> Option<HandleInfo> {
        let s = self.handles.get(&id)?;
        let status = if s.stopped.load(Ordering::Relaxed) {
            DaemonStatus::Stopped
        } else {
            DaemonStatus::Running
        };
        let stage_counts = s
            .stage_counts
            .lock()
            .ok()
            .map(|sc| sc.to_vec())
            .unwrap_or_else(|| vec![0u32; 8]);
        Some(HandleInfo {
            id: s.id,
            actor: s.actor.clone(),
            started_at: s.started_at,
            iterations: s.iterations.load(Ordering::Relaxed),
            status,
            stage_counts,
        })
    }

    /// Signal the background thread to stop after its current iteration.
    pub fn stop(&self, id: Uuid) -> bool {
        if let Some(s) = self.handles.get(&id) {
            s.stopped.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Count of handles that have not been stopped.
    pub fn active_count(&self) -> usize {
        self.handles
            .values()
            .filter(|s| !s.stopped.load(Ordering::Relaxed))
            .count()
    }
}

impl Default for SubstrateDaemon {
    fn default() -> Self {
        Self::new()
    }
}
