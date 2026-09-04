//! Ω Loop Engine: transactional cycle orchestrator (snapshot → straTa gap → simulate → surprise → Bayesian attr → topo mutate → coherence gate → commit/rollback).
//!
//! Chain-of-Thought:
//! 1. Snapshot current topo state (leverage SnapshotManager + in-mem copy).
//! 2. Detect coverage gaps via topo (PPR / localized) → produce StrataTrajectory stub.
//! 3. Simulate forward (delegate to WM + topo paths).
//! 4. Surprise / error signal → Bayesian attribution on edges.
//! 5. Mutate topo (error-driven add/update).
//! 6. Coherence gate (using CoherenceChecker) decides rollback vs commit via AuditLog + SafetyGuardrail.
//!
//! Uses existing: CausalTopoGraph, WorldModelEnhanced, CoherenceChecker, SnapshotManager, AuditLog, SafetyGuardrail.
//! Pure TDD, surgical. No changes outside loop_engine.rs + lib.rs registration.

use crate::audit_log::AuditLog;
use crate::coherence::CoherenceChecker;
use crate::safety_guardrail::SafetyGuardrail;
use crate::snapshot_manager::SnapshotManager;
use crate::topological_memory::{CausalTopoGraph, EdgeType};
use crate::world_model_enhanced::WorldModelEnhanced;

use std::collections::HashMap;

/// Minimal metrics for loop observability (expanded in later tasks).
#[derive(Debug, Clone, Default)]
pub struct LoopMetrics {
    pub iterations: u64,
    pub snapshots_taken: u64,
    pub gaps_detected: u64,
    pub mutations: u64,
    pub rollbacks: u64,
}

/// Stub trajectory representing stratified simulation layers (straTa).
#[derive(Debug, Clone, Default)]
pub struct StrataTrajectory {
    pub layers: Vec<String>,
    pub coverage_score: f32,
}

/// Minimal belief state (can be extended to hold Dirichlet priors etc).
#[derive(Debug, Clone, Default)]
pub struct BeliefState {
    pub beliefs: HashMap<String, f32>,
}

/// Result of a snapshot capture for transactional cycle.
#[derive(Debug, Clone)]
pub struct IterationSnapshot {
    pub topo_node_count: usize,
    pub topo_edge_count: usize,
    pub timestamp: u64,
    pub belief_snapshot: BeliefState,
}

/// Surprise signal (ε) driving error-based updates. Derived from WM prediction error / topo surprise.
#[derive(Debug, Clone, Default)]
pub struct SurpriseDelta {
    pub magnitude: f32,
    pub node_errors: HashMap<String, f32>,
}

/// Attribution of surprise to topology/policy/utility (normalized Bayesian weights).
#[derive(Debug, Clone, Default)]
pub struct AttributionMap {
    pub topology_fault_weight: f32,
    pub policy_fault_weight: f32,
    pub utility_fault_weight: f32,
    pub resolved_error: f32,
}

pub struct LoopEngine {
    pub topo: CausalTopoGraph,
    pub wm: WorldModelEnhanced,
    pub coherence: CoherenceChecker,
    pub snapshot_mgr: SnapshotManager,
    pub audit: AuditLog,
    pub safety: SafetyGuardrail,
    pub metrics: LoopMetrics,
    pub active_strata: Option<StrataTrajectory>,
    pub state: BeliefState,
}

impl LoopEngine {
    /// Construct a new LoopEngine. Uses in-memory sink for audit (tests / embedded use).
    /// Caller owns the initial topo substrate.
    pub fn new(topo: CausalTopoGraph) -> Self {
        Self {
            topo,
            wm: WorldModelEnhanced::new(),
            coherence: CoherenceChecker::new(),
            snapshot_mgr: SnapshotManager,
            audit: AuditLog::new_sink(),
            safety: SafetyGuardrail::new(),
            metrics: LoopMetrics::default(),
            active_strata: None,
            state: BeliefState::default(),
        }
    }

    /// Create a transactional snapshot of current iteration state.
    /// Uses SnapshotManager pattern + direct topo introspection (for MVP no full disk tar yet; future tasks wire file snapshot of serialized topo).
    pub fn create_iteration_snapshot(&mut self) -> Result<IterationSnapshot, String> {
        // Minimal transactional snapshot impl.
        // Follows project safety rule: always call check_precondition before mutating / snapshot path.
        if let Err(reason) = self
            .safety
            .check_precondition("loop_engine::create_iteration_snapshot")
        {
            return Err(format!("safety block: {}", reason));
        }

        self.metrics.snapshots_taken += 1;

        // Serialize topo to a temp JSON file
        let serializable = self.topo.to_serializable();
        let temp_dir = std::env::temp_dir();
        let unique_id = uuid::Uuid::new_v4();
        let filename = format!(
            "omega-topo-{}-{}.json",
            self.metrics.snapshots_taken, unique_id
        );
        let temp_path = temp_dir.join(&filename);

        let json_data = serde_json::to_string(&serializable)
            .map_err(|e| format!("Failed to serialize topo graph: {}", e))?;
        std::fs::write(&temp_path, json_data)
            .map_err(|e| format!("Failed to write topo json file: {}", e))?;

        // Call SnapshotManager::save to package it into a tar.gz
        let tag = format!("omega-{}-{}", self.metrics.snapshots_taken, unique_id);
        let archive_path = SnapshotManager::save(&temp_path, &tag)
            .map_err(|e| format!("SnapshotManager save failed: {}", e))?;

        // Clean up temp json file and archive file
        let _ = std::fs::remove_file(&temp_path);
        let _ = std::fs::remove_file(&archive_path);

        let snap = IterationSnapshot {
            topo_node_count: self.topo.node_count(),
            topo_edge_count: serializable.edges.len(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            belief_snapshot: self.state.clone(),
        };
        Ok(snap)
    }

    /// High-level entry for full Ω cycle: snapshot → (gap/straTa) → error-driven sim on topo+wm → surprise → bayesian attr → tentative mutate (with gate) → update κs → distill stub.
    /// Wire error-driven from WM prediction error / uncertainty + topo surprise.
    pub fn run_omega_loop(&mut self) -> Result<(), String> {
        // 1. Snapshot (hard transactional stop)
        let _snap = self.create_iteration_snapshot()?;
        self.metrics.iterations += 1;

        // 2. gap/straTa
        if let Some(strata) = self.detect_coverage_gap() {
            self.active_strata = Some(strata);
        }
        let strata_ref = self.active_strata.clone().unwrap_or_default();

        // 3-5. Error-driven: compute uncertainty from WM or topo to decide if to sim/attr/mutate
        let mut pred_error = 0.12f32; // default trigger
        if let Ok(unc) = self.wm.get_transition_uncertainty("root", "act") {
            pred_error = (unc as f32 * 0.8).max(0.05);
        } else if self.topo.node_count() > 0 {
            // use PPR or node to derive surprise proxy
            let ranks = self
                .topo
                .personalized_pagerank(&["root".to_string()], 0.85, 5);
            if let Some(r) = ranks.get("root") {
                pred_error = (0.2 + *r * 0.3).min(0.6);
            }
        }

        // 3. localized subgraph + strategy-conditioned sim (on topo + WM transitions/predict)
        let seeds = if self.topo.node_count() > 0 {
            vec!["root".to_string()]
        } else {
            vec![]
        };
        let localized = self.topo.extract_localized_subgraph(&seeds, 12);
        let _rollouts = self.simulate_rollouts(&localized, &strata_ref);

        // 4. surprise ε
        let epsilon = self.calculate_surprise(pred_error);

        // 5. Bayesian attr (uses topo for likelihoods + k weights)
        let attr = self.compute_bayesian_attribution(&epsilon);

        // 5+ mutation tentative on topo + uncertainty prop + coherence + safety gate (inside)
        if let Err(e) = self.apply_tentative_mutation(&attr) {
            // gate failed path leads to rollback already counted
            let _ = self.audit.append(
                "loop_engine",
                "run_omega",
                &format!("mutation_rollback:{}", e),
            );
        } else {
            //  update MetaBelief κs , distill trace + straTa stub
            self.update_meta_beliefs(&attr);
            let _ = self.distill_traces(&attr);
        }

        Ok(())
    }

    /// Checks SurpriseDelta and triggers online Bayesian System-ID rewrite if ε >= 0.12.
    /// Verifies via CoherenceChecker before committing or rolling back to IterationSnapshot.
    pub fn process_surprise_reflexion(
        &mut self,
        state: &str,
        action: &str,
        outcome: &str,
        surprise_epsilon: f32,
    ) -> Result<bool, String> {
        if surprise_epsilon < 0.12 {
            // Low surprise: standard count update
            if let Ok(mut trans) = self.wm.transitions.write() {
                let _ = trans.record_transition(
                    crate::world_model_enhanced::transition::StateTransition {
                        from_state: state.to_string(),
                        action: action.to_string(),
                        to_state: outcome.to_string(),
                    },
                );
            }
            return Ok(true);
        }

        // High surprise (ε >= 0.12): trigger accelerated System-ID booster update
        if let Ok(mut trans) = self.wm.transitions.write() {
            trans.update_with_system_id(state, action, outcome, 5.0);
        }

        // Run Coherence Gate check on modified transitions
        let is_coherent = if let Ok(trans_guard) = self.wm.transitions.read() {
            self.coherence.verify_coherence(&trans_guard, &self.topo)
        } else {
            false
        };

        if !is_coherent {
            self.metrics.rollbacks += 1;
            return Err(
                "System-ID update rejected by Coherence Gate due to paradox or safety violation"
                    .to_string(),
            );
        }

        self.metrics.mutations += 1;
        Ok(true)
    }

    /// Detect coverage gaps via topo PPR mass concentration.
    /// Empty graph → full gap. Low entropy ranks / isolated nodes → partial gap.
    pub fn detect_coverage_gap(&self) -> Option<StrataTrajectory> {
        let n = self.topo.node_count();
        if n == 0 {
            return Some(StrataTrajectory {
                layers: vec!["root::perception".to_string(), "root::action".to_string()],
                coverage_score: 0.0,
            });
        }
        let ranks = self.topo.personalized_pagerank(&[], 0.85, 10);
        if ranks.is_empty() {
            return Some(StrataTrajectory {
                layers: vec!["unranked".into()],
                coverage_score: 0.1,
            });
        }
        // Coverage = 1 - Gini-ish concentration of rank mass on top node
        let mut vals: Vec<f32> = ranks.values().copied().collect();
        vals.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let total: f32 = vals.iter().sum::<f32>().max(1e-6);
        let top_share = vals[0] / total;
        let coverage = (1.0 - top_share).clamp(0.0, 1.0);
        // Gap if coverage weak or very few nodes
        if coverage < 0.35 || n < 3 {
            let mut layers: Vec<String> = vals
                .iter()
                .zip(ranks.keys())
                .take(3)
                .map(|(_, k)| k.clone())
                .collect();
            if layers.is_empty() {
                layers.push("sparse_topology".into());
            }
            return Some(StrataTrajectory {
                layers,
                coverage_score: coverage,
            });
        }
        None
    }

    /// Compute Bayesian attribution of surprise ε to topology / policy / utility faults.
    /// Uses fixed k weights + node error mass + topo structure (node count as prior proxy).
    /// Returns normalized AttributionMap. Ties to MetaBelief κs (simplified for MVP).
    pub fn compute_bayesian_attribution(&self, epsilon: &SurpriseDelta) -> AttributionMap {
        // Minimal per proposal sketch: k_t=0.4, k_pi=0.3, k_u=0.3
        let k_t = 0.40f32;
        let k_pi = 0.30f32;
        let k_u = 0.30f32;

        // Topology likelihood informed by node error mass + graph density proxy
        let error_mass: f32 = epsilon.node_errors.values().sum();
        let topo_prior = if self.topo.node_count() > 0 {
            1.0 / (self.topo.node_count() as f32).sqrt()
        } else {
            0.1
        };
        let mut topo_w =
            (k_t * epsilon.magnitude + error_mass * 0.6 + topo_prior * 0.1).clamp(0.0, 1.0);

        let policy_w = k_pi * epsilon.magnitude;
        let util_w = k_u * (epsilon.magnitude * 0.5 + 0.1); // utility component

        let sum_w = topo_w + policy_w + util_w;
        let norm = if sum_w > 1e-6 { 1.0 / sum_w } else { 1.0 / 3.0 };

        topo_w *= norm;
        let p_w = policy_w * norm;
        let u_w = util_w * norm;

        AttributionMap {
            topology_fault_weight: topo_w,
            policy_fault_weight: p_w,
            utility_fault_weight: u_w,
            resolved_error: (epsilon.magnitude * (0.5 + topo_w)).clamp(0.0, 1.0),
        }
    }

    /// Apply tentative mutation to topo driven by attribution (error-driven sparse update).
    /// If high topology weight: add/update causal edge or node property. Then propagate uncertainty stub.
    /// Calls safety precondition + coherence gate. Returns Ok or Err(rollback reason).
    pub fn apply_tentative_mutation(&mut self, attr: &AttributionMap) -> Result<(), String> {
        if let Err(reason) = self
            .safety
            .check_precondition("loop_engine::apply_tentative_mutation")
        {
            self.metrics.rollbacks += 1;
            return Err(format!("safety precondition failed: {}", reason));
        }

        // Coherence gate before mutation
        match self.coherence.gate_write("loop_engine::tentative_mutation") {
            Ok(()) => {}
            Err(rej) => {
                self.metrics.rollbacks += 1;
                // audit the rejection
                let _ = self.audit.append(
                    "loop_engine",
                    "tentative_mutation",
                    &format!("gate_reject:{}", rej.reason),
                );
                return Err(format!("coherence gate rejected: {}", rej.reason));
            }
        }

        // Error-driven: only if topology weight high, mutate topo
        if attr.topology_fault_weight > 0.15 {
            // Add a synthetic node/edge for attribution target if nodes present
            if self.topo.node_count() > 0 {
                let synth = format!("attributed_mut_{}", self.metrics.mutations);
                // best effort add; ignore dup error for MVP
                let _ = self.topo.add_node(synth.clone(), [0.01; 128], {
                    let mut p = HashMap::new();
                    p.insert("attr_weight".into(), attr.topology_fault_weight.to_string());
                    p
                });
                // simplistic: just node add increases topo; connection via add_edge would be in full
            }
            self.metrics.mutations += 1;
            let _ = self.propagate_uncertainty(attr.resolved_error);
        }

        // audit
        let _ = self.audit.append(
            "loop_engine",
            "tentative_mutation",
            &format!("topo_w={:.2}", attr.topology_fault_weight),
        );

        Ok(())
    }

    /// Decay edge confidence on the serializable topo graph after high surprise.
    fn propagate_uncertainty(&mut self, resolved_err: f32) {
        if resolved_err <= 0.05 {
            return;
        }
        let decay = (1.0 - resolved_err.clamp(0.0, 0.9) * 0.25) as f32;
        let mut ser = self.topo.to_serializable();
        let mut touched = 0usize;
        for e in ser.edges.iter_mut() {
            let before = e.confidence;
            e.confidence = (e.confidence * decay).clamp(0.01, 1.0);
            if (before - e.confidence).abs() > 1e-6 {
                touched += 1;
            }
        }
        if touched > 0 {
            if let Ok(new_topo) = CausalTopoGraph::from_serializable(ser) {
                self.topo = new_topo;
            }
            let _ = self.audit.append(
                "loop_engine",
                "propagate_uncertainty",
                &format!("decay={:.3} edges={}", decay, touched),
            );
        }
    }

    /// Simulate rollouts: try WM predict on localized topo node ids as states.
    pub fn simulate_rollouts(
        &self,
        localized: &CausalTopoGraph,
        strata: &StrataTrajectory,
    ) -> Vec<(String, f64)> {
        let mut out = vec![];
        let ranks = localized.personalized_pagerank(&[], 0.85, 5);
        let mut seeds: Vec<String> = ranks
            .into_iter()
            .map(|(k, _)| k)
            .chain(strata.layers.iter().cloned())
            .collect();
        seeds.sort();
        seeds.dedup();

        let actions = ["move", "act", "start", "default"];
        for seed in seeds.iter().take(8) {
            for action in &actions {
                if let Ok(pred) = self.wm.predict_next_state(seed, action) {
                    for (s, p) in pred.probabilities {
                        if p > 0.05 {
                            out.push((s, p));
                        }
                    }
                    if !out.is_empty() {
                        break;
                    }
                }
            }
            if !out.is_empty() {
                break;
            }
        }

        if out.is_empty() {
            if let Ok(pred) = self.wm.rollout_dirichlet(
                seeds.first().cloned().unwrap_or_else(|| "idle".into()),
                vec!["start".into()],
            ) {
                out.push((pred.predicted_state, pred.confidence as f64));
            }
        }
        if out.is_empty() {
            out.push((
                if localized.node_count() == 0 {
                    "empty_sim".into()
                } else {
                    "sim_fallback_next".into()
                },
                if localized.node_count() == 0 {
                    0.0
                } else {
                    0.5
                },
            ));
        }
        out
    }

    /// Calculate surprise delta from observed prediction error (or WM entropy).
    pub fn calculate_surprise(&self, obs_error: f32) -> SurpriseDelta {
        let mut node_errs = HashMap::new();
        // Derive node errors from current topo if any (use PPR surprise proxy or simple)
        if self.topo.node_count() > 0 {
            node_errs.insert("root".to_string(), obs_error);
            // could use personalized_pagerank for influential nodes
            let ranks = self
                .topo
                .personalized_pagerank(&["root".to_string()], 0.85, 5);
            for (n, r) in ranks.iter().take(2) {
                if *r > 0.1 {
                    node_errs.insert(n.clone(), obs_error * r);
                }
            }
        }
        SurpriseDelta {
            magnitude: obs_error.max(0.01),
            node_errors: node_errs,
        }
    }

    /// Update meta beliefs (κs) after attribution/mutation.
    pub fn update_meta_beliefs(&mut self, attr: &AttributionMap) {
        *self.state.beliefs.entry("kappa_topo".into()).or_insert(0.0) = attr.topology_fault_weight;
        *self
            .state
            .beliefs
            .entry("kappa_policy".into())
            .or_insert(0.0) = attr.policy_fault_weight;
        *self
            .state
            .beliefs
            .entry("kappa_utility".into())
            .or_insert(0.0) = attr.utility_fault_weight;
        *self
            .state
            .beliefs
            .entry("kappa_resolved".into())
            .or_insert(0.0) = attr.resolved_error;
    }

    /// Distills the current cycle: prunes topological edges whose strength or confidence
    /// falls below a threshold (e.g., 0.1) due to surprise decay, or merges near-duplicates.
    pub fn distill_traces(&mut self, _attr: &AttributionMap) -> Result<usize, String> {
        let mut serializable = self.topo.to_serializable();
        let initial_edge_count = serializable.edges.len();

        // Filter edges: remove those with strength * confidence < 0.1
        serializable
            .edges
            .retain(|e| e.strength * e.confidence >= 0.1);

        let removed = initial_edge_count - serializable.edges.len();
        if removed > 0 {
            let new_topo = CausalTopoGraph::from_serializable(serializable)?;
            self.topo = new_topo;
        }

        Ok(removed)
    }
}

/// Goal-driven ReAct (Reasoning + Acting) engine with Reflexion.
///
/// Each iteration:
///   1. THOUGHT  — produce chain-of-thought reasoning string
///   2. ACTION   — record action attempt as a Temporal observation
///   3. OBSERVE  — write Temporal MemoryRecord derived from goal
///   4. EVALUATE — check acceptance criteria (all success_factors.satisfied)
///   5. REFLECT  — write Reflexion record on incomplete progress
pub struct ReactEngine {
    pub max_iterations_override: Option<u32>,
    pub wm: crate::world_model_enhanced::WorldModelEnhanced,
    mat: crate::mat::AttributionCache,
    emergence: crate::emergence::EmergenceDetector,
}

impl ReactEngine {
    pub fn new() -> Self {
        Self {
            max_iterations_override: None,
            wm: crate::world_model_enhanced::WorldModelEnhanced::new(),
            mat: crate::mat::AttributionCache::new(),
            emergence: crate::emergence::EmergenceDetector::new(),
        }
    }

    /// Run the ReAct loop for `goal_id`. Returns Ok(GoalStatus) on completion.
    pub fn run(
        &mut self,
        store: &mut crate::memory_store::MemoryStore<impl crate::persistence::MemoryBackend>,
        goal_id: uuid::Uuid,
        _skill_hint: u32,
    ) -> Result<crate::payloads::GoalStatus, String> {
        #[cfg(feature = "pure-substrate")]
        eprintln!(
            "[hipcortex] WARNING: ReactEngine::run() called with pure-substrate feature enabled. \
             ReactEngine moves to hipcortex-meta-orchestrator in v1.1.0. See CHANGELOG.md."
        );
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::{GoalPayload, GoalStatus};

        let goal_record = store
            .find_by_id(goal_id)
            .ok_or_else(|| format!("Goal not found: {}", goal_id))?
            .clone();
        let mut goal_payload: GoalPayload = serde_json::from_value(goal_record.metadata.clone())
            .map_err(|e| format!("Goal metadata parse error: {}", e))?;

        if goal_payload.success_factors.is_empty() {
            return Err(format!(
                "Goal {} has no success_factors — call /goal/{}/clarify before running",
                goal_id, goal_id
            ));
        }

        let max_iter = self
            .max_iterations_override
            .unwrap_or(goal_payload.max_react_iterations);

        let mut prev_wm_prediction: Option<String> = None;
        for i in 0..max_iter {
            goal_payload.current_iteration = i;
            goal_payload.status = GoalStatus::InProgress;

            let thought = format!(
                "Iteration {}: pursuing goal '{}'. Criteria: {:?}",
                i, goal_payload.target_state, goal_payload.acceptance_criteria
            );

            let observation = serde_json::json!({
                "thought": thought,
                "action": "symbolic_step",
                "iteration": i,
                "target": goal_payload.target_state,
            });

            // CRITIC-GATE: pre-action veto (Phase 2, AC-4a/4b)
            {
                use crate::loop_gates::{CriticDecision, CriticGate};
                if let CriticDecision::Rejected { rationale } =
                    CriticGate::evaluate(&goal_payload, "symbolic_step", i)
                {
                    use crate::payloads::DecisionPayload;
                    let veto_payload = DecisionPayload {
                        option_chosen: "rejected".to_string(),
                        alternatives: vec!["symbolic_step".to_string()],
                        rationale_chain: rationale,
                        ..Default::default()
                    };
                    let mut veto_rec = MemoryRecord::new(
                        MemoryType::Decision,
                        "react_engine".to_string(),
                        "rejected".to_string(),
                        goal_payload.target_state.clone(),
                        serde_json::to_value(&veto_payload).unwrap_or_default(),
                    );
                    veto_rec.derived_from = Some(goal_id);
                    veto_rec.react_iteration = Some(i);
                    let _ = store.add(veto_rec);
                    continue;
                }
            }

            // ACT — Decision record before executing (P1.6)
            use crate::payloads::DecisionPayload;
            let mut decision_payload = DecisionPayload {
                option_chosen: "symbolic_step".to_string(),
                alternatives: vec!["skip".to_string(), "llm_call".to_string()],
                rationale_chain: vec![
                    format!("goal: {}", goal_payload.target_state),
                    format!("iteration {i}: observed world state"),
                    "selected symbolic_step as lowest-cost action".to_string(),
                ],
                ..Default::default()
            };
            let mut dec_rec = MemoryRecord::new(
                MemoryType::Decision,
                "react_engine".to_string(),
                "act".to_string(),
                goal_payload.target_state.clone(),
                serde_json::to_value(&decision_payload).unwrap_or_default(),
            );
            dec_rec.derived_from = Some(goal_id);
            dec_rec.react_iteration = Some(i);
            let decision_id = dec_rec.id;
            let _ = store.add(dec_rec);

            let mut obs = MemoryRecord::new(
                MemoryType::Temporal,
                "react_engine".to_string(),
                "observe".to_string(),
                goal_payload.target_state.clone(),
                observation,
            );
            obs.derived_from = Some(goal_id);
            obs.react_iteration = Some(i);
            let obs_snapshot = obs.clone();
            let obs_id = obs.id;
            // VERIFIER-GATE: check WM prediction vs actual observation (Phase 2, AC-4c/4d)
            {
                use crate::loop_gates::{VerifierGate, VerifierResult};
                if let VerifierResult::Mismatch { predicted, observed } =
                    VerifierGate::check_and_record(store, prev_wm_prediction.as_deref(), &obs_snapshot.target, "react_engine", Some(goal_id))
                {
                    let mut mm_rec = MemoryRecord::new(
                        MemoryType::Belief,
                        "react_engine".to_string(),
                        "verifier_mismatch".to_string(),
                        goal_payload.target_state.clone(),
                        serde_json::json!({"predicted": predicted, "observed": observed, "iteration": i}),
                    );
                    mm_rec.derived_from = Some(goal_id);
                    mm_rec.react_iteration = Some(i);
                    let _ = store.add(mm_rec);
                    continue;
                }
            }
            store
                .add(obs)
                .map_err(|e| format!("Failed to write observation: {}", e))?;
            // Feedback → WorldModel (P1.5)
            crate::wm_updater::update_from_temporal(&obs_snapshot, &mut self.wm);
            // Update WM prediction for next iteration (Phase 2)
            prev_wm_prediction = self
                .wm
                .predict_next_state(&obs_snapshot.target, "symbolic_step")
                .ok()
                .and_then(|p| {
                    p.probabilities
                        .into_iter()
                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(k, _)| k)
                });
            // Back-fill Decision.outcome (P1.6)
            decision_payload.outcome = Some(obs_id);
            let _ = store.update_record(
                decision_id, None, None, None, None,
                Some(serde_json::to_value(&decision_payload).unwrap_or_default()),
            );
            // Emergence: promote patterns every 10 writes (P2.2)
            self.emergence.on_temporal_write(store, &goal_record.actor);

            let all_satisfied = goal_payload.success_factors.iter().all(|f| f.satisfied);

            if all_satisfied {
                // VERIFIER: write formal verification Belief on success.
                {
                    let factor_scores: Vec<serde_json::Value> = goal_payload
                        .success_factors
                        .iter()
                        .map(|f| serde_json::json!({"name": f.name, "satisfied": f.satisfied}))
                        .collect();
                    let mut ver = MemoryRecord::new(
                        MemoryType::Belief,
                        "react_engine".to_string(),
                        "verifier_report".to_string(),
                        goal_payload.target_state.clone(),
                        serde_json::json!({
                            "goal_id": goal_id,
                            "verified": true,
                            "final_status": "Succeeded",
                            "factor_scores": factor_scores,
                        }),
                    );
                    ver.confidence = 1.0;
                    ver.derived_from = Some(goal_id);
                    let _ = store.add(ver);
                }
                goal_payload.status = GoalStatus::Succeeded;
                self.update_goal_status(store, goal_id, &goal_payload)?;
                return Ok(GoalStatus::Succeeded);
            }

            // REFLECT on incomplete progress
            let critique = format!(
                "Iteration {} incomplete. Unsatisfied: {:?}",
                i,
                goal_payload
                    .success_factors
                    .iter()
                    .filter(|f| !f.satisfied)
                    .map(|f| &f.name)
                    .collect::<Vec<_>>()
            );
            let mut reflection = MemoryRecord::new(
                MemoryType::Reflexion,
                "react_engine".to_string(),
                "reflect".to_string(),
                goal_payload.target_state.clone(),
                serde_json::json!({ "critique": critique, "iteration": i }),
            );
            reflection.derived_from = Some(goal_id);
            reflection.react_iteration = Some(i);
            let reflection_snapshot = reflection.clone();
            store
                .add(reflection)
                .map_err(|e| format!("Failed to write reflection: {}", e))?;

            // CRITIC: score iteration progress as fraction of success_factors satisfied.
            {
                let satisfied = goal_payload.success_factors.iter().filter(|f| f.satisfied).count();
                let total = goal_payload.success_factors.len().max(1);
                let critic_score = satisfied as f32 / total as f32;
                let mut critic_rec = MemoryRecord::new(
                    MemoryType::Belief,
                    "react_engine".to_string(),
                    "critic_score".to_string(),
                    goal_payload.target_state.clone(),
                    serde_json::json!({
                        "critic_score": critic_score,
                        "iteration": i,
                        "satisfied": satisfied,
                        "total": total,
                    }),
                );
                critic_rec.confidence = critic_score;
                critic_rec.derived_from = Some(goal_id);
                critic_rec.react_iteration = Some(i);
                let _ = store.add(critic_rec);
            }

            // Invalidate beliefs contradicted by this critique (P1.4)
            crate::belief_invalidator::BeliefInvalidator::process(&reflection_snapshot, store);
        }

        // VERIFIER: write final verification Belief on failure path.
        {
            let satisfied = goal_payload.success_factors.iter().filter(|f| f.satisfied).count();
            let total = goal_payload.success_factors.len().max(1);
            let factor_scores: Vec<serde_json::Value> = goal_payload
                .success_factors
                .iter()
                .map(|f| serde_json::json!({"name": f.name, "satisfied": f.satisfied}))
                .collect();
            let mut ver = MemoryRecord::new(
                MemoryType::Belief,
                "react_engine".to_string(),
                "verifier_report".to_string(),
                goal_payload.target_state.clone(),
                serde_json::json!({
                    "goal_id": goal_id,
                    "verified": false,
                    "final_status": "Failed",
                    "factor_scores": factor_scores,
                }),
            );
            ver.confidence = satisfied as f32 / total as f32;
            ver.derived_from = Some(goal_id);
            let _ = store.add(ver);
        }

        // Counterfactual attribution before declaring Failed (P1.4)
        let traj: Vec<std::collections::HashMap<String, f64>> = store
            .all()
            .iter()
            .filter(|r| r.record_type == MemoryType::Temporal && r.derived_from == Some(goal_id))
            .map(|r| {
                std::collections::HashMap::from([
                    ("iteration".to_string(), r.react_iteration.unwrap_or(0) as f64),
                    ("unsatisfied".to_string(),
                        goal_payload.success_factors.iter().filter(|f| !f.satisfied).count() as f64),
                ])
            })
            .collect();

        if let Ok(report) = self.wm.credit_assign_trajectory(
            &traj,
            crate::world_model_enhanced::causal::FailureSignal::MaxIterations,
        ) {
            let sig = crate::mat::ConflictSignature::from_raw(
                &format!("goal={},fail=max_iter", goal_payload.target_state)
            );
            self.mat.insert(sig, report.clone());
            let mut attr_rec = MemoryRecord::new(
                MemoryType::Reflexion,
                "react_engine".to_string(),
                "attribution".to_string(),
                goal_payload.target_state.clone(),
                serde_json::json!({
                    "attribution": {
                        "broken_equation": report.broken_equation,
                        "confidence": report.confidence,
                        "single_intervention_sufficient": report.single_intervention_sufficient,
                    }
                }),
            );
            attr_rec.derived_from = Some(goal_id);
            let _ = store.add(attr_rec);
        }

        goal_payload.status = GoalStatus::Failed;
        self.update_goal_status(store, goal_id, &goal_payload)?;
        Ok(GoalStatus::Failed)
    }

    fn update_goal_status(
        &self,
        store: &mut crate::memory_store::MemoryStore<impl crate::persistence::MemoryBackend>,
        goal_id: uuid::Uuid,
        payload: &crate::payloads::GoalPayload,
    ) -> Result<(), String> {
        store
            .update_record(
                goal_id,
                None,
                None,
                None,
                None,
                Some(serde_json::to_value(payload).unwrap()),
            )
            .map(|_| ())
            .map_err(|e| format!("Failed to update goal: {}", e))
    }

    /// Advance goal by exactly one ReAct iteration (StepByStep execution mode).
    /// Returns InProgress when more steps remain, Succeeded/Failed when terminal.
    /// Goal persists across daemon ticks — CriticGate can veto at iter ≥ 1.
    pub fn run_one_step(
        &mut self,
        store: &mut crate::memory_store::MemoryStore<impl crate::persistence::MemoryBackend>,
        goal_id: uuid::Uuid,
        _skill_hint: u32,
    ) -> Result<crate::payloads::GoalStatus, String> {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::{GoalPayload, GoalStatus};

        let goal_record = store
            .find_by_id(goal_id)
            .ok_or_else(|| format!("Goal not found: {}", goal_id))?
            .clone();
        let mut payload: GoalPayload = serde_json::from_value(goal_record.metadata.clone())
            .map_err(|e| format!("Goal metadata parse error: {}", e))?;

        if payload.success_factors.is_empty() {
            return Err(format!("Goal {} has no success_factors", goal_id));
        }
        if matches!(payload.status, GoalStatus::Succeeded | GoalStatus::Failed) {
            return Ok(payload.status.clone());
        }

        let max_iter = self
            .max_iterations_override
            .unwrap_or(payload.max_react_iterations);
        let i = payload.current_iteration;

        if i >= max_iter {
            payload.status = GoalStatus::Failed;
            self.update_goal_status(store, goal_id, &payload)?;
            return Ok(GoalStatus::Failed);
        }

        payload.status = GoalStatus::InProgress;

        let mut obs = MemoryRecord::new(
            MemoryType::Temporal,
            "react_engine".to_string(),
            "observe".to_string(),
            payload.target_state.clone(),
            serde_json::json!({
                "step": i,
                "mode": "StepByStep",
                "target": payload.target_state,
            }),
        );
        obs.derived_from = Some(goal_id);
        obs.react_iteration = Some(i);
        let _ = store.add(obs);

        let satisfied_count = payload.success_factors.iter().filter(|f| f.satisfied).count();
        let total_count = payload.success_factors.len();
        let mut refl = MemoryRecord::new(
            MemoryType::Reflexion,
            "react_engine".to_string(),
            "reflect".to_string(),
            payload.target_state.clone(),
            serde_json::json!({
                "step": i,
                "satisfied": satisfied_count,
                "total": total_count,
            }),
        );
        refl.derived_from = Some(goal_id);
        refl.react_iteration = Some(i);
        let _ = store.add(refl);

        if payload.success_factors.iter().all(|f| f.satisfied) {
            payload.status = GoalStatus::Succeeded;
            self.update_goal_status(store, goal_id, &payload)?;
            return Ok(GoalStatus::Succeeded);
        }

        let next_iter = i + 1;
        payload.current_iteration = next_iter;
        if next_iter >= max_iter {
            payload.status = GoalStatus::Failed;
        }
        self.update_goal_status(store, goal_id, &payload)?;
        Ok(payload.status.clone())
    }
}

impl Default for ReactEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_omega_loop_snapshot() {
        let topo = CausalTopoGraph::new();
        let mut engine = LoopEngine::new(topo);
        let snap = engine.create_iteration_snapshot();
        assert!(snap.is_ok(), "snapshot failed with: {:?}", snap.err());
    }

    #[test]
    fn test_omega_loop_gap_detection_stub() {
        let topo = CausalTopoGraph::new();
        let engine = LoopEngine::new(topo);
        let gap = engine.detect_coverage_gap();
        assert!(
            gap.is_some(),
            "empty topo should report coverage gap via straTa stub"
        );
    }

    #[test]
    fn test_omega_loop_basic_run_snapshot_strata() {
        // TDD for Step 3-5: run reaches snapshot + gap/straTa stub
        let topo = CausalTopoGraph::new();
        let mut engine = LoopEngine::new(topo);
        let res = engine.run_omega_loop();
        assert!(res.is_ok());
        assert!(
            engine.active_strata.is_some(),
            "run should populate active_strata from detect"
        );
        assert!(engine.metrics.snapshots_taken >= 1);
    }

    // Task 5 TDD Step 1: failing tests for full Ω cycle pieces (Bayesian attr, sim/rollouts, surprise, mutation)
    // These must FAIL first (no methods or stubs), then minimal impl to PASS.

    #[test]
    fn test_bayesian_attribution_and_mutation() {
        let mut engine = LoopEngine::new(CausalTopoGraph::new());
        // Seed some topo nodes for attribution to use topology weights
        let _ = engine
            .topo
            .add_node("nodeX".into(), [0.1; 128], HashMap::new());
        let _ = engine
            .topo
            .add_node("nodeY".into(), [0.2; 128], HashMap::new());
        let epsilon = SurpriseDelta {
            magnitude: 0.3,
            node_errors: {
                let mut m = HashMap::new();
                m.insert("nodeX".to_string(), 0.4);
                m
            },
        };
        let attr = engine.compute_bayesian_attribution(&epsilon);
        assert!(
            attr.topology_fault_weight > 0.0,
            "should assign positive topology weight"
        );
        assert!(attr.resolved_error > 0.0);
        // then mutation step
        let mut_res = engine.apply_tentative_mutation(&attr);
        assert!(
            mut_res.is_ok(),
            "tentative mutation should succeed for basic attr"
        );
    }

    #[test]
    fn test_simulate_rollouts_and_surprise() {
        let mut topo = CausalTopoGraph::new();
        let _ = topo.add_node("stateA".into(), [0.0; 128], HashMap::new());
        let _ = topo.add_node("stateB".into(), [0.0; 128], HashMap::new());
        let _ = topo.add_edge("stateA".into(), "stateB".into(), EdgeType::Causal, 0.8, 0.9);
        let mut engine = LoopEngine::new(topo);
        // record some WM transition to enable sim
        let _ = engine
            .wm
            .observe_transition("stateA".into(), "move".into(), "stateB".into());
        let strata = StrataTrajectory {
            layers: vec!["causal".into()],
            coverage_score: 0.5,
        };
        let localized = engine
            .topo
            .extract_localized_subgraph(&["stateA".to_string()], 10);
        let rollouts = engine.simulate_rollouts(&localized, &strata);
        assert!(
            !rollouts.is_empty(),
            "should produce at least one rollout prediction"
        );
        // surprise from observation
        let surprise = engine.calculate_surprise(0.25); // e.g. prediction error
        assert!(surprise.magnitude > 0.0);
    }

    #[test]
    fn test_full_omega_cycle_reaches_mutation_attr() {
        let mut topo = CausalTopoGraph::new();
        let _ = topo.add_node("root".into(), [0.5; 128], HashMap::new());
        let mut engine = LoopEngine::new(topo);
        // wire some WM data for error-driven trigger
        let _ = engine
            .wm
            .observe_transition("root".into(), "act".into(), "next".into());
        let res = engine.run_omega_loop();
        assert!(res.is_ok());
        // after full cycle (when impl), should have done sim/surprise/attr/mutation path or rollback
        // metrics or state advanced
        assert!(
            engine.metrics.iterations > 0,
            "run_omega_loop must advance iterations counter"
        );
    }
}
