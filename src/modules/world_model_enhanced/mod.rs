//! # World-Model Enhanced — Predictive State Modeling and Causal Reasoning
//!
//! The world-model enables HipCortex to learn transition dynamics, track
//! entities through time, reason about causality, and quantify uncertainty.
//! It combines probabilistic learning with structured causal graphs.
//!
//! ## Components
//!
//! | Component | Role | Algorithm |
//! |-----------|------|-----------|
//! | [`TransitionModel`] | Learn P(next_state | state, action) from observations | Dirichlet-Multinomial |
//! | [`EntityTracker`] | Estimate entity state with uncertainty via Kalman filter | Linear Kalman |
//! | [`CausalGraph`] | Build and query directed acyclic causal graphs, with do-calculus interventions | DAG + cycle prevention |
//! | [`UncertaintyEstimator`] | Decompose uncertainty into epistemic vs aleatoric, calibration tracking | ECE ≤ 0.1 target |
//!
//! ## Quick Start
//!
//! ```rust
//! use hipcortex::world_model_enhanced::*;
//!
//! // Learn transition dynamics
//! let mut model = TransitionModel::new();
//! model.record_transition(StateTransition {
//!     from_state: "idle".into(), action: "process".into(), to_state: "busy".into(),
//! }).unwrap();
//! let pred = model.predict("idle", "process").unwrap();
//! // pred.probabilities → {"busy": 1.0}
//!
//! // Track an entity with Kalman filtering
//! let mut wm = WorldModelEnhanced::new();
//! wm.register_entity("robot_1".into(), EntityState {
//!     properties: vec![0.0, 0.0],
//!     covariance: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
//! }).unwrap();
//! wm.update_entity("robot_1", EntityObservation {
//!     measured_properties: vec![0.5, 0.3],
//!     measurement_noise: vec![vec![0.1, 0.0], vec![0.0, 0.1]],
//!     timestamp: std::time::Instant::now(),
//! }).unwrap();
//!
//! // Predict entity state 3 steps ahead
//! let future = wm.predict_entity("robot_1", 3).unwrap();
//! // future.properties → predicted [x, y]
//! // future.covariance → increased uncertainty
//!
//! // Build causal graph
//! let mut graph = CausalGraph::new();
//! graph.add_node("treatment".into());
//! graph.add_node("outcome".into());
//! graph.add_edge("treatment".into(), "outcome".into()).unwrap();
//! assert!(graph.is_acyclic());
//! ```
//!
//! ## Mathematical Guarantees
//!
//! - **Probability conservation**: Sum of P(next_state | state, action) = 1.0 for all pairs
//! - **Acyclicity**: Causal graph remains a DAG — adding a cycle-forming edge returns `Err`
//! - **Covariance positive semi-definiteness**: Kalman covariance matrices remain PSD
//! - **Prediction uncertainty growth**: Covariance trace grows monotonically with prediction steps
//! - **Transitivity**: If A→B and B→C, then path A→C exists

pub mod causal;
pub mod constraint;
pub mod entity;
pub mod metalaw;
pub mod policy;
mod predictor;
pub mod simulator;
pub mod transition;
mod uncertainty;

pub use causal::{CausalEdge, CausalGraph, CausalNode, InterventionQuery};
pub use constraint::{Constraint, ConstraintEngine, ConstraintSeverity};
pub use entity::{Anomaly, EntityObservation, EntityState, EntityTracker};
pub use metalaw::{InvariantType, MetaLaw, MetaLawEngine};
pub use policy::Policy;
pub use predictor::{LearnedTransitionPredictor, PredictionResult, PredictiveModel};
pub use simulator::{
    MctsSimulator, SimulationHarness, SimulationStep, SimulationTrajectory, SimulatorNode,
};
pub use transition::{StateTransition, TransitionModel, TransitionPrediction};
pub use uncertainty::{CalibrationMetrics, ConfidenceInterval, UncertaintyEstimator};

use crate::topological_memory::{CausalTopoGraph, EdgeType};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Stable string for empirical dist keys (1.0 → "1.0", not "1").
fn format_empirical_value(v: f64) -> String {
    if (v.fract()).abs() < 1e-12 {
        format!("{:.1}", v)
    } else {
        format!("{}", v)
    }
}

/// Central World-Model coordinator integrating all predictive capabilities
pub struct WorldModelEnhanced {
    /// State transition model (Dirichlet-Multinomial)
    pub transitions: Arc<RwLock<TransitionModel>>,

    /// Entity trackers (Kalman filters)
    entities: Arc<RwLock<HashMap<String, EntityTracker>>>,

    /// Causal graph for causal reasoning
    causal_graph: Arc<RwLock<CausalGraph>>,

    /// Learned predictive models
    predictors: Arc<RwLock<Vec<Box<dyn PredictiveModel + Send + Sync>>>>,

    /// Uncertainty estimator
    uncertainty: Arc<RwLock<UncertaintyEstimator>>,

    /// Topological substrate (hybrid nodes: symbolic_id + [f32;128] embedding + props) to replace dummies in record_perceived_action
    /// (Task 7 integration; coexists with custom CausalGraph for semantics)
    topo_graph: Arc<RwLock<CausalTopoGraph>>,
}

impl WorldModelEnhanced {
    /// Create a new World-Model with default configuration
    pub fn new() -> Self {
        Self {
            transitions: Arc::new(RwLock::new(TransitionModel::new())),
            entities: Arc::new(RwLock::new(HashMap::new())),
            causal_graph: Arc::new(RwLock::new(CausalGraph::new())),
            predictors: Arc::new(RwLock::new(Vec::new())),
            uncertainty: Arc::new(RwLock::new(UncertaintyEstimator::new())),
            topo_graph: Arc::new(RwLock::new(CausalTopoGraph::new())),
        }
    }

    /// Record a perceived action from the agent stream as a world model transition.
    /// This is called automatically from the auto AgentMessage path (no explicit user/agent trigger required).
    /// Feeds the Dirichlet transition model so the substrate maintains latest state predictions
    /// and can answer "what happens if we do X" without the LLM having to remember to tell it.
    pub fn record_perceived_action(&self, action: String) {
        // Task 7: use/create topo states (real symbolic + embeddings) instead of hardcoded dummies
        // "agent_perceived" / "updated"
        if let Ok(mut topo) = self.topo_graph.write() {
            let from = "perceived".to_string();
            let to = format!(
                "after_{}",
                action
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
            );
            let mut props: HashMap<String, String> = HashMap::new();
            props.insert("source".to_string(), "record_perceived_action".to_string());
            props.insert("action".to_string(), action.clone());

            // simple non-zero embedding derived from action bytes (hybrid topo node)
            let mut emb = [0.0f32; 128];
            for (i, b) in action.as_bytes().iter().take(128).enumerate() {
                emb[i] = (*b as f32) / 255.0 + 0.01 * (i as f32);
            }

            let _ = topo.add_node(from.clone(), [0.01f32; 128], HashMap::new());
            let _ = topo.add_node(to.clone(), emb, props);
            let _ = topo.add_edge(from, to, EdgeType::Causal, 0.65, 0.75);
        }

        if let Ok(mut t) = self.transitions.write() {
            let _ = t.record_transition(StateTransition {
                from_state: "perceived".to_string(),
                action,
                to_state: "post_action_state".to_string(),
            });
        }
    }

    // ========================================================================
    // State Transition Learning
    // ========================================================================

    /// Record a state transition observation
    ///
    /// Updates Dirichlet-Multinomial posterior for P(s'|s,a)
    pub fn observe_transition(
        &self,
        from_state: String,
        action: String,
        to_state: String,
    ) -> Result<(), String> {
        let mut transitions = self
            .transitions
            .write()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;

        transitions.record_transition(StateTransition {
            from_state,
            action,
            to_state,
        })
    }

    /// Predict next state distribution given current state and action
    ///
    /// Returns probability distribution over possible next states
    pub fn predict_next_state(
        &self,
        current_state: &str,
        action: &str,
    ) -> Result<TransitionPrediction, String> {
        let transitions = self
            .transitions
            .read()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;

        transitions.predict(current_state, action)
    }

    /// Get transition uncertainty (Shannon entropy)
    pub fn get_transition_uncertainty(&self, state: &str, action: &str) -> Result<f64, String> {
        let transitions = self
            .transitions
            .read()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;

        transitions.compute_entropy(state, action)
    }

    // ========================================================================
    // Entity Tracking
    // ========================================================================

    /// Register a new entity with initial properties
    pub fn register_entity(
        &self,
        entity_id: String,
        initial_state: EntityState,
    ) -> Result<(), String> {
        let mut entities = self
            .entities
            .write()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;

        if entities.contains_key(&entity_id) {
            return Err(format!("Entity '{}' already exists", entity_id));
        }

        let tracker = EntityTracker::new(initial_state);
        entities.insert(entity_id, tracker);

        Ok(())
    }

    /// Update entity with new observation (Kalman update)
    pub fn update_entity(
        &self,
        entity_id: &str,
        observation: EntityObservation,
    ) -> Result<(), String> {
        let mut entities = self
            .entities
            .write()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;

        let tracker = entities
            .get_mut(entity_id)
            .ok_or_else(|| format!("Entity '{}' not found", entity_id))?;

        tracker.update(observation)
    }

    /// Predict entity state N steps into future
    pub fn predict_entity(&self, entity_id: &str, steps: usize) -> Result<EntityState, String> {
        let entities = self
            .entities
            .read()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;

        let tracker = entities
            .get(entity_id)
            .ok_or_else(|| format!("Entity '{}' not found", entity_id))?;

        tracker.predict(steps)
    }

    /// Check if entity has anomalies
    pub fn get_entity_anomalies(&self, entity_id: &str) -> Result<Vec<Anomaly>, String> {
        let entities = self
            .entities
            .read()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;

        let tracker = entities
            .get(entity_id)
            .ok_or_else(|| format!("Entity '{}' not found", entity_id))?;

        Ok(tracker.get_anomalies())
    }

    /// Return per-entity covariance diagonals (entity_id → per-dim variances).
    /// Used by SimulationFork to seed rollout uncertainty from parent state.
    pub fn entity_covariance_diagonals(&self) -> std::collections::HashMap<String, Vec<f32>> {
        let Ok(entities) = self.entities.read() else {
            return std::collections::HashMap::new();
        };
        entities
            .iter()
            .map(|(id, t)| (id.clone(), t.covariance_diagonal()))
            .collect()
    }

    /// Return (entity_id_string, mean_properties_vec) for all tracked entities.
    /// Used by DigitalTwin to seed continuous state from entity positions.
    pub fn entity_mean_vectors(&self) -> Vec<(String, Vec<f64>)> {
        if let Ok(guard) = self.entities.read() {
            guard
                .iter()
                .map(|(k, v)| (k.clone(), v.get_state().properties.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// List all tracked entities
    pub fn list_entities(&self) -> Result<Vec<String>, String> {
        let entities = self
            .entities
            .read()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;

        Ok(entities.keys().cloned().collect())
    }

    // ========================================================================
    // Causal Reasoning
    // ========================================================================

    /// Add causal edge A → B
    pub fn add_causal_edge(&self, from: String, to: String) -> Result<(), String> {
        let mut graph = self
            .causal_graph
            .write()
            .map_err(|e| format!("Failed to acquire causal graph lock: {}", e))?;

        graph.add_edge(from, to)
    }

    /// Check if A causally affects B (path exists)
    pub fn has_causal_path(&self, from: &str, to: &str) -> Result<bool, String> {
        let graph = self
            .causal_graph
            .read()
            .map_err(|e| format!("Failed to acquire causal graph lock: {}", e))?;

        graph.has_path(from, to)
    }

    /// Perform causal intervention P(Y|do(X=x)).
    /// Prefers exact empirical backdoor adjustment when distributions exist; else heuristic.
    pub fn causal_intervention(
        &self,
        query: InterventionQuery,
    ) -> Result<HashMap<String, f64>, String> {
        let mut graph = self
            .causal_graph
            .write()
            .map_err(|e| format!("Failed to acquire causal graph lock: {}", e))?;

        // Normalize float keys: 1.0 → "1.0" (fixed) so record/query match.
        let x_val = format_empirical_value(query.intervention_value);
        let y_candidates = [
            "1.0".to_string(),
            "0.0".to_string(),
            "1".to_string(),
            "0".to_string(),
            x_val.clone(),
        ];
        let mut result = HashMap::new();
        if graph.has_empirical_key(&query.intervention_var, &x_val) {
            for y_val in &y_candidates {
                let y_key = format!("{}={}", query.outcome, y_val);
                // Only insert if a real conditional entry exists (not the 0.5 default hole).
                if graph.has_empirical_outcome(
                    &query.intervention_var,
                    &x_val,
                    &query.outcome,
                    y_val,
                ) {
                    if let Ok(p) = graph.compute_empirical_intervention(
                        &query.intervention_var,
                        &x_val,
                        &query.outcome,
                        y_val,
                    ) {
                        result.insert(y_key, p);
                    }
                }
            }
            if !result.is_empty() {
                if let Some((_, &p)) = result
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                {
                    result.insert(query.outcome.clone(), p);
                }
                return Ok(result);
            }
        }

        graph.compute_intervention(&query)
    }

    /// Counterfactual via Pearl SCM (abduction-action-prediction); falls back to heuristic.
    pub fn counterfactual(
        &self,
        actual_state: HashMap<String, f64>,
        intervention_var: String,
        intervention_value: f64,
    ) -> Result<HashMap<String, f64>, String> {
        let graph = self
            .causal_graph
            .read()
            .map_err(|e| format!("Failed to acquire causal graph lock: {}", e))?;

        match graph.compute_scm_counterfactual(&actual_state, &intervention_var, intervention_value)
        {
            Ok(cf) => Ok(cf),
            Err(_) => {
                graph.compute_counterfactual(actual_state, intervention_var, intervention_value)
            }
        }
    }

    // ========================================================================
    // Predictive Models
    // ========================================================================

    /// Train predictive model from transition history
    pub fn train_predictor(&self) -> Result<(), String> {
        let transitions = self
            .transitions
            .read()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;

        if transitions.observation_count() < 100 {
            return Err("Need at least 100 observations to train predictor".to_string());
        }

        let predictor = LearnedTransitionPredictor::train(&transitions)?;

        let mut predictors = self
            .predictors
            .write()
            .map_err(|e| format!("Failed to acquire predictors lock: {}", e))?;

        predictors.push(Box::new(predictor));

        Ok(())
    }

    /// Predict state after action sequence
    pub fn predict_multi_step(
        &self,
        initial_state: String,
        actions: Vec<String>,
    ) -> Result<PredictionResult, String> {
        let predictors = self
            .predictors
            .read()
            .map_err(|e| format!("Failed to acquire predictors lock: {}", e))?;

        if predictors.is_empty() {
            return Err("No trained predictors available".to_string());
        }

        // Use ensemble of all predictors
        let mut ensemble_predictions = Vec::new();

        for predictor in predictors.iter() {
            let pred = predictor.predict_sequence(initial_state.clone(), actions.clone())?;
            ensemble_predictions.push(pred);
        }

        // Average predictions
        PredictionResult::ensemble_average(ensemble_predictions)
    }

    /// Multi-step MAP rollout using Dirichlet transition counts (no trained predictor required).
    /// Returns trajectory of MAP next-states + joint confidence (product of max probs).
    pub fn rollout_dirichlet(
        &self,
        initial_state: String,
        actions: Vec<String>,
    ) -> Result<PredictionResult, String> {
        if actions.is_empty() {
            return Err("actions must be non-empty".to_string());
        }
        let transitions = self
            .transitions
            .read()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;

        let mut state = initial_state;
        let mut joint_conf = 1.0_f64;
        let mut last_dist = HashMap::new();
        let steps = actions.len();

        for action in &actions {
            let pred = transitions.predict(&state, action)?;
            let (next, p) = pred
                .probabilities
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(k, v)| (k.clone(), *v))
                .ok_or_else(|| {
                    format!(
                        "empty distribution for state '{}' action '{}'",
                        state, action
                    )
                })?;
            joint_conf *= p;
            last_dist = pred.probabilities;
            state = next;
        }

        Ok(PredictionResult {
            predicted_state: state,
            distribution: last_dist,
            confidence: joint_conf.clamp(0.0, 1.0),
            steps,
        })
    }

    /// Goal-shaped state reward for MCTS: high if state matches goal, medium if substring,
    /// else small baseline. Prefer certain (low entropy) transitions when no goal.
    pub fn mcts_state_reward(state: &str, goal_state: Option<&str>) -> f64 {
        if let Some(g) = goal_state {
            if state == g {
                return 10.0;
            }
            if !g.is_empty() && (state.contains(g) || g.contains(state)) {
                return 3.0;
            }
            return 0.05;
        }
        1.0
    }

    /// MCTS (UCB1) best first action over TransitionModel.
    /// `available_actions` empty → actions observed from root_state only.
    /// `goal_state` shapes leaf/rollout reward (prefer reaching goal).
    pub fn mcts_best_action(
        &self,
        root_state: &str,
        available_actions: &[String],
        iterations: usize,
        max_depth: usize,
        exploration_constant: f64,
    ) -> Result<String, String> {
        self.mcts_best_action_goal(
            root_state,
            available_actions,
            iterations,
            max_depth,
            exploration_constant,
            None,
        )
    }

    pub fn mcts_best_action_goal(
        &self,
        root_state: &str,
        available_actions: &[String],
        iterations: usize,
        max_depth: usize,
        exploration_constant: f64,
        goal_state: Option<&str>,
    ) -> Result<String, String> {
        let transitions = self
            .transitions
            .read()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;
        let actions: Vec<String> = if available_actions.is_empty() {
            transitions
                .get_actions()
                .into_iter()
                .filter(|a| {
                    transitions
                        .totals
                        .contains_key(&(root_state.to_string(), a.clone()))
                })
                .collect()
        } else {
            available_actions
                .iter()
                .filter(|a| {
                    transitions
                        .totals
                        .contains_key(&(root_state.to_string(), (*a).clone()))
                })
                .cloned()
                .collect()
        };
        if actions.is_empty() {
            return Err("No actions available for MCTS (observe transitions first)".to_string());
        }
        let goal = goal_state.map(|s| s.to_string());
        let mut sim = MctsSimulator::new(root_state.to_string(), exploration_constant);
        sim.search(
            &transitions,
            &actions,
            iterations.max(1),
            max_depth.max(1),
            move |s| Self::mcts_state_reward(s, goal.as_deref()),
        )
    }

    /// MCTS search for best first action, then one-step Dirichlet MAP along that action.
    pub fn rollout_mcts(
        &self,
        initial_state: String,
        available_actions: Vec<String>,
        iterations: usize,
        max_depth: usize,
    ) -> Result<(String, PredictionResult), String> {
        self.rollout_mcts_goal(
            initial_state,
            available_actions,
            iterations,
            max_depth,
            None,
        )
    }

    pub fn rollout_mcts_goal(
        &self,
        initial_state: String,
        available_actions: Vec<String>,
        iterations: usize,
        max_depth: usize,
        goal_state: Option<String>,
    ) -> Result<(String, PredictionResult), String> {
        let best = self.mcts_best_action_goal(
            &initial_state,
            &available_actions,
            iterations,
            max_depth,
            1.414,
            goal_state.as_deref(),
        )?;
        let pred = self.rollout_dirichlet(initial_state, vec![best.clone()])?;
        Ok((best, pred))
    }

    // ========================================================================
    // Uncertainty Quantification
    // ========================================================================

    /// Get confidence interval for prediction
    pub fn get_prediction_confidence(
        &self,
        state: &str,
        action: &str,
    ) -> Result<ConfidenceInterval, String> {
        let uncertainty = self
            .uncertainty
            .read()
            .map_err(|e| format!("Failed to acquire uncertainty lock: {}", e))?;

        let prediction = self.predict_next_state(state, action)?;

        uncertainty.compute_confidence_interval(&prediction)
    }

    /// Get calibration metrics
    pub fn get_calibration_metrics(&self) -> Result<CalibrationMetrics, String> {
        let uncertainty = self
            .uncertainty
            .read()
            .map_err(|e| format!("Failed to acquire uncertainty lock: {}", e))?;

        Ok(uncertainty.get_metrics())
    }

    /// Decompose uncertainty into epistemic and aleatoric
    pub fn decompose_uncertainty(&self, state: &str, action: &str) -> Result<(f64, f64), String> {
        let uncertainty = self
            .uncertainty
            .read()
            .map_err(|e| format!("Failed to acquire uncertainty lock: {}", e))?;

        let transitions = self
            .transitions
            .read()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;

        let sa_key = (state.to_string(), action.to_string());
        let count = transitions.totals.get(&sa_key).cloned().unwrap_or(0);

        let entropy = if count > 0 {
            transitions.compute_entropy(state, action).unwrap_or(0.0)
        } else {
            0.0
        };

        uncertainty.decompose(state, action, count, entropy)
    }

    /// Test helper (Task 7 TDD): expose topo node count so tests can verify record_perceived populates hybrid states/embeddings.
    #[cfg(test)]
    pub(crate) fn topo_node_count(&self) -> usize {
        self.topo_graph.read().map(|g| g.node_count()).unwrap_or(0)
    }

    /// Return all causal edges for serialization.
    pub fn get_causal_edges(&self) -> Vec<CausalEdge> {
        self.causal_graph
            .read()
            .map(|g| g.all_edges())
            .unwrap_or_default()
    }

    /// Sync causal graph distributions from the current transition model.
    /// Calls `auto_populate_from_transitions` so the causal DAG stays in step
    /// with the latest Dirichlet posteriors without requiring a full rebuild.
    pub fn sync_causal_distributions(&self) {
        if let (Ok(t), Ok(mut g)) = (self.transitions.read(), self.causal_graph.write()) {
            g.auto_populate_from_transitions(&t);
        }
    }

    /// Total number of state transitions observed across all (state, action) pairs.
    pub fn transition_count(&self) -> usize {
        self.transitions
            .read()
            .map(|t| t.observation_count())
            .unwrap_or(0)
    }

    /// All unique states observed in transition data.
    pub fn get_states(&self) -> Vec<String> {
        self.transitions
            .read()
            .map(|t| t.get_states())
            .unwrap_or_default()
    }

    /// All unique actions observed in transition data.
    pub fn get_actions(&self) -> Vec<String> {
        self.transitions
            .read()
            .map(|t| t.get_actions())
            .unwrap_or_default()
    }

    /// Entropy for every observed (state, action) pair, sorted descending by entropy.
    /// Returns Vec of (state, action, entropy_bits).
    pub fn get_all_entropy(&self) -> Vec<(String, String, f64)> {
        let transitions = match self.transitions.read() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        let mut result: Vec<(String, String, f64)> = transitions
            .totals
            .keys()
            .filter_map(|(state, action)| {
                transitions
                    .compute_entropy(state, action)
                    .ok()
                    .map(|e| (state.clone(), action.clone(), e))
            })
            .collect();
        result.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// Save world model state to a JSON file.
    /// Persists: transition counts + totals, causal edges.
    /// Entity Kalman states are NOT persisted (recoverable from live memory).
    /// Uses atomic write: writes to .tmp file then renames.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let transitions = self
            .transitions
            .read()
            .map_err(|e| anyhow::anyhow!("transitions lock error: {}", e))?;
        let causal = self
            .causal_graph
            .read()
            .map_err(|e| anyhow::anyhow!("causal lock error: {}", e))?;
        let entities = self
            .entities
            .read()
            .map_err(|e| anyhow::anyhow!("entities lock error: {}", e))?;

        // Encode HashMap<(String,String,String), usize> → HashMap<String, usize>
        // using \x1F (ASCII Unit Separator) as separator — safe in any UTF-8 string
        let counts_encoded: std::collections::HashMap<String, usize> = transitions
            .counts
            .iter()
            .map(|((s, a, ns), &v)| (format!("{}\x1F{}\x1F{}", s, a, ns), v))
            .collect();
        let totals_encoded: std::collections::HashMap<String, usize> = transitions
            .totals
            .iter()
            .map(|((s, a), &v)| (format!("{}\x1F{}", s, a), v))
            .collect();

        let causal_edges: Vec<serde_json::Value> = causal
            .all_edges()
            .iter()
            .map(|e| serde_json::json!({"from": e.from, "to": e.to, "strength": e.strength}))
            .collect();

        let entities_encoded: std::collections::HashMap<String, serde_json::Value> = entities
            .iter()
            .map(|(id, tracker)| {
                let state = tracker.get_state();
                (
                    id.clone(),
                    serde_json::json!({
                        "properties": state.properties,
                        "covariance": state.covariance,
                    }),
                )
            })
            .collect();

        let data = serde_json::json!({
            "version": 1,
            "transition_counts": counts_encoded,
            "transition_totals": totals_encoded,
            "smoothing": transitions.smoothing(),
            "causal_edges": causal_edges,
            "entities": entities_encoded,
        });

        // Atomic write: write to .tmp then rename
        let tmp_path = path.as_ref().with_extension("json.tmp");
        std::fs::write(&tmp_path, serde_json::to_string_pretty(&data)?)?;
        std::fs::rename(&tmp_path, path.as_ref())?;
        Ok(())
    }

    /// Load world model state from a JSON file written by save().
    /// Returns Err if the file doesn't exist or is malformed.
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let data: serde_json::Value = serde_json::from_str(&content)?;

        let wm = Self::new();

        // Restore transition counts
        {
            let mut transitions = wm
                .transitions
                .write()
                .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            let smoothing = data["smoothing"].as_f64().unwrap_or(1.0);
            *transitions = TransitionModel::with_smoothing(smoothing);

            if let Some(obj) = data["transition_counts"].as_object() {
                for (k, v) in obj {
                    let parts: Vec<&str> = k.splitn(3, '\x1F').collect();
                    if parts.len() == 3 {
                        if let Some(count) = v.as_u64() {
                            transitions.counts.insert(
                                (
                                    parts[0].to_string(),
                                    parts[1].to_string(),
                                    parts[2].to_string(),
                                ),
                                count as usize,
                            );
                        }
                    }
                }
            }
            if let Some(obj) = data["transition_totals"].as_object() {
                for (k, v) in obj {
                    let parts: Vec<&str> = k.splitn(2, '\x1F').collect();
                    if parts.len() == 2 {
                        if let Some(total) = v.as_u64() {
                            transitions.totals.insert(
                                (parts[0].to_string(), parts[1].to_string()),
                                total as usize,
                            );
                        }
                    }
                }
            }
        }

        // Restore causal edges
        {
            let mut causal = wm
                .causal_graph
                .write()
                .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            if let Some(arr) = data["causal_edges"].as_array() {
                for e in arr {
                    let from = e["from"].as_str().unwrap_or("").to_string();
                    let to = e["to"].as_str().unwrap_or("").to_string();
                    if !from.is_empty() && !to.is_empty() {
                        let _ = causal.add_edge(from, to); // ignore cycle prevention errors on load
                    }
                }
            }
        }

        // Restore entities
        {
            let mut entities = wm
                .entities
                .write()
                .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            if let Some(obj) = data["entities"].as_object() {
                for (id, val) in obj {
                    if let (Some(props_val), Some(cov_val)) =
                        (val["properties"].as_array(), val["covariance"].as_array())
                    {
                        let properties: Vec<f64> =
                            props_val.iter().filter_map(|v| v.as_f64()).collect();
                        let covariance: Vec<Vec<f64>> = cov_val
                            .iter()
                            .filter_map(|row_val| {
                                row_val.as_array().map(|row| {
                                    row.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>()
                                })
                            })
                            .collect();

                        let initial_state = EntityState {
                            properties,
                            covariance,
                        };
                        let tracker = EntityTracker::new(initial_state);
                        entities.insert(id.clone(), tracker);
                    }
                }
            }
        }

        Ok(wm)
    }

    pub fn causal_node_count(&self) -> usize {
        self.causal_graph.read().map(|g| g.node_count()).unwrap_or(0)
    }

    pub fn causal_edge_count(&self) -> usize {
        self.causal_graph.read().map(|g| g.edge_count()).unwrap_or(0)
    }

    pub fn credit_assign_trajectory(
        &self,
        trajectory: &[std::collections::HashMap<String, f64>],
        signal: crate::world_model_enhanced::causal::FailureSignal,
    ) -> Result<crate::world_model_enhanced::causal::AttributionReport, String> {
        let graph = self.causal_graph.read().map_err(|e| format!("lock: {}", e))?;
        graph.credit_assign(trajectory, &signal)
    }

    pub fn apply_intervention(&self, var: &str, value: f64) -> Result<(), String> {
        self.causal_graph
            .write()
            .map_err(|e| format!("causal lock: {}", e))?
            .apply_intervention(var, value);
        Ok(())
    }

    pub fn rewrite_structural_equation(
        &self,
        node_id: &str,
        new_weights: Vec<f64>,
    ) -> Result<(), String> {
        use crate::world_model_enhanced::causal::LinearSE;
        let mut g = self.causal_graph
            .write()
            .map_err(|e| format!("causal lock: {}", e))?;
        let node = g.node_mut(node_id)
            .ok_or_else(|| format!("node '{}' not found in causal graph", node_id))?;
        node.equation = Some(std::sync::Arc::new(LinearSE { weights: new_weights }));
        Ok(())
    }

    pub fn causal_node_ids(&self) -> Vec<String> {
        self.causal_graph
            .read()
            .map(|g| g.node_ids())
            .unwrap_or_default()
    }
}

impl Default for WorldModelEnhanced {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_world_model() {
        let wm = WorldModelEnhanced::new();
        assert!(wm.list_entities().unwrap().is_empty());
    }

    #[test]
    fn test_record_and_predict_transition() {
        let wm = WorldModelEnhanced::new();

        // Record several transitions
        wm.observe_transition("S1".to_string(), "A1".to_string(), "S2".to_string())
            .unwrap();
        wm.observe_transition("S1".to_string(), "A1".to_string(), "S2".to_string())
            .unwrap();
        wm.observe_transition("S1".to_string(), "A1".to_string(), "S3".to_string())
            .unwrap();

        // Predict should return distribution
        let pred = wm.predict_next_state("S1", "A1").unwrap();
        assert!(pred.probabilities.len() >= 2);

        // S2 should be more likely than S3 (2/3 vs 1/3)
        let p_s2 = pred.probabilities.get("S2").unwrap_or(&0.0);
        let p_s3 = pred.probabilities.get("S3").unwrap_or(&0.0);
        assert!(p_s2 > p_s3);
    }

    #[test]
    fn test_rollout_dirichlet_multi_step() {
        let wm = WorldModelEnhanced::new();
        wm.observe_transition("idle".into(), "start".into(), "run".into())
            .unwrap();
        wm.observe_transition("run".into(), "stop".into(), "idle".into())
            .unwrap();
        let pred = wm
            .rollout_dirichlet("idle".into(), vec!["start".into(), "stop".into()])
            .unwrap();
        assert_eq!(pred.predicted_state, "idle");
        assert_eq!(pred.steps, 2);
        assert!(pred.confidence > 0.0);
    }

    #[test]
    fn test_mcts_best_action_after_observe() {
        let wm = WorldModelEnhanced::new();
        wm.observe_transition("S".into(), "go".into(), "T".into())
            .unwrap();
        wm.observe_transition("S".into(), "wait".into(), "S".into())
            .unwrap();
        let best = wm.mcts_best_action("S", &[], 30, 2, 1.414).unwrap();
        assert!(best == "go" || best == "wait", "got {}", best);
    }

    #[test]
    fn test_mcts_goal_prefers_path_to_goal() {
        let wm = WorldModelEnhanced::new();
        // go → Goal, wait → stay
        for _ in 0..5 {
            wm.observe_transition("S".into(), "go".into(), "Goal".into())
                .unwrap();
            wm.observe_transition("S".into(), "wait".into(), "S".into())
                .unwrap();
        }
        let best = wm
            .mcts_best_action_goal("S", &[], 80, 2, 1.414, Some("Goal"))
            .unwrap();
        assert_eq!(best, "go", "goal-shaped MCTS should prefer go → Goal");
    }

    #[test]
    fn test_empirical_intervention_parentless() {
        let mut g = CausalGraph::new();
        g.add_node("X".into()).unwrap();
        g.add_node("Y".into()).unwrap();
        g.add_edge("X".into(), "Y".into()).unwrap();
        g.record_empirical_distribution("X=1.0".into(), "Y=1".into(), 0.8);
        let p = g
            .compute_empirical_intervention("X", "1.0", "Y", "1")
            .unwrap();
        assert!((p - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_wm_causal_intervention_uses_empirical_float_keys() {
        let wm = WorldModelEnhanced::new();
        {
            let mut g = wm.causal_graph.write().unwrap();
            g.add_node("X".into()).unwrap();
            g.add_node("Y".into()).unwrap();
            g.add_edge("X".into(), "Y".into()).unwrap();
            g.record_empirical_distribution("X=1.0".into(), "Y=1.0".into(), 0.9);
        }
        let q = InterventionQuery {
            outcome: "Y".into(),
            intervention_var: "X".into(),
            intervention_value: 1.0,
            conditioned_on: HashMap::new(),
            intervention_label: None,
        };
        let res = wm.causal_intervention(q).unwrap();
        assert!(
            (res.get("Y").copied().unwrap_or(0.0) - 0.9).abs() < 1e-9,
            "got {:?}",
            res
        );
    }

    #[test]
    fn test_scm_counterfactual_linear() {
        let mut g = CausalGraph::new();
        g.add_node("X".into()).unwrap();
        g.add_node("Y".into()).unwrap();
        g.add_edge("X".into(), "Y".into()).unwrap();
        // edge weight default 1.0: Y = X + U_y; observe X=0.5, Y=0.7 → U_y=0.2
        let mut obs = HashMap::new();
        obs.insert("X".into(), 0.5);
        obs.insert("Y".into(), 0.7);
        let cf = g.compute_scm_counterfactual(&obs, "X", 1.0).unwrap();
        assert!((cf["X"] - 1.0).abs() < 1e-9);
        assert!((cf["Y"] - 1.2).abs() < 1e-9); // 1.0*1.0 + 0.2
    }

    #[test]
    fn test_entity_registration() {
        let wm = WorldModelEnhanced::new();

        let initial_state = EntityState {
            properties: vec![1.0, 2.0, 3.0],
            covariance: vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
        };

        assert!(wm
            .register_entity("entity1".to_string(), initial_state)
            .is_ok());

        let entities = wm.list_entities().unwrap();
        assert_eq!(entities.len(), 1);
        assert!(entities.contains(&"entity1".to_string()));
    }

    #[test]
    fn test_causal_edge_addition() {
        let wm = WorldModelEnhanced::new();

        assert!(wm.add_causal_edge("A".to_string(), "B".to_string()).is_ok());
        assert!(wm.add_causal_edge("B".to_string(), "C".to_string()).is_ok());

        // Should have transitive path A → B → C
        assert!(wm.has_causal_path("A", "C").unwrap());
        assert!(!wm.has_causal_path("C", "A").unwrap()); // No reverse path
    }

    #[test]
    fn test_cycle_prevention() {
        let wm = WorldModelEnhanced::new();

        wm.add_causal_edge("A".to_string(), "B".to_string())
            .unwrap();
        wm.add_causal_edge("B".to_string(), "C".to_string())
            .unwrap();

        // Creating cycle A → B → C → A should fail
        assert!(wm
            .add_causal_edge("C".to_string(), "A".to_string())
            .is_err());
    }

    // TDD Step 1 for Task 7: failing tests using topo + hybrid extend
    // (uses moved to top of mod tests; tests expect no dummies + topo + hybrid Causal)

    #[test]
    fn test_record_perceived_action_populates_topo_not_dummies() {
        let wm = WorldModelEnhanced::new();
        wm.record_perceived_action("rotate".to_string());
        let states = wm.get_states();
        // Should FAIL currently: dummies "agent_perceived"/"updated" still present
        assert!(
            !states
                .iter()
                .any(|s| s == "agent_perceived" || s == "updated"),
            "record_perceived should not use dummy states; must use topo real states/embeddings"
        );
    }

    #[test]
    fn test_record_perceived_action_creates_topo_nodes_with_embeddings() {
        let wm = WorldModelEnhanced::new();
        wm.record_perceived_action("forward".to_string());
        // Will FAIL compile or assert until field + impl + topo_node_count accessor added in mod.rs
        let count = wm.topo_node_count();
        assert!(
            count >= 2,
            "record_perceived_action must populate topo with hybrid nodes (symbolic + embedding)"
        );
    }

    #[test]
    fn test_causal_graph_add_node_with_embedding_hybrid() {
        let mut g = CausalGraph::new();
        let emb: [f32; 128] = [0.42; 128];
        // now supported via causal extension for topo hybrid
        let res = g.add_node_with_embedding("hybrid_state".to_string(), emb);
        assert!(res.is_ok());
        assert!(g.node_count() >= 1);
        // test delegation too
        assert!(g.has_path_topo_delegated("hybrid_state", "hybrid_state")); // self
    }

    // Additional TDD (step 4): integration of record + observe + predict + topo/causal hybrid
    #[test]
    fn test_wm_record_and_observe_with_topo_tie_in() {
        let wm = WorldModelEnhanced::new();
        wm.record_perceived_action("scan".to_string());
        // also direct observe (uses transition, topo is via record path)
        let _ = wm.observe_transition(
            "perceived".to_string(),
            "scan".to_string(),
            "post_action_state".to_string(),
        );
        // predict tie-in
        let pred = wm.predict_next_state("perceived", "scan");
        assert!(pred.is_ok());
        // topo populated from record
        assert!(wm.topo_node_count() >= 2);
        // causal still functional (hybrid)
        assert!(wm
            .add_causal_edge("perceived".to_string(), "post_action_state".to_string())
            .is_ok());
        assert!(wm
            .has_causal_path("perceived", "post_action_state")
            .unwrap());
    }

    #[test]
    fn test_entity_tracker_persistence() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wm_entity_test.json");

        let wm = WorldModelEnhanced::new();
        let initial_state = EntityState {
            properties: vec![1.5, 2.5, 3.5],
            covariance: vec![
                vec![0.5, 0.0, 0.0],
                vec![0.0, 0.5, 0.0],
                vec![0.0, 0.0, 0.5],
            ],
        };
        wm.register_entity("persistence_entity".to_string(), initial_state)
            .unwrap();

        // Save
        wm.save(&file_path).unwrap();

        // Load
        let loaded_wm = WorldModelEnhanced::load(&file_path).unwrap();

        // Clean up
        let _ = std::fs::remove_file(&file_path);

        // Verify entity is restored
        let entities = loaded_wm.list_entities().unwrap();
        assert!(entities.contains(&"persistence_entity".to_string()));

        let entities_guard = loaded_wm.entities.read().unwrap();
        let tracker = entities_guard.get("persistence_entity").unwrap();
        let state = tracker.get_state();
        assert_eq!(state.properties, vec![1.5, 2.5, 3.5]);
        assert_eq!(state.covariance[0][0], 0.5);
    }

    #[test]
    fn test_wm_causal_counts_empty() {
        let wm = WorldModelEnhanced::new();
        assert_eq!(wm.causal_node_count(), 0);
        assert_eq!(wm.causal_edge_count(), 0);
    }

    #[test]
    fn test_wm_causal_counts_after_edge() {
        let wm = WorldModelEnhanced::new();
        wm.add_causal_edge("A".into(), "B".into()).unwrap();
        assert_eq!(wm.causal_node_count(), 2);
        assert_eq!(wm.causal_edge_count(), 1);
    }
}
