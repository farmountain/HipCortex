// World-Model Enhanced Module - Predictive state modeling and causal reasoning
//
// This module extends the basic World-Model with:
// - Probabilistic state transition learning (Dirichlet-Multinomial)
// - Entity tracking with Kalman filtering
// - Causal graph for causal reasoning and do-calculus
// - Multi-step predictive models
// - Calibrated uncertainty quantification

mod transition;
mod entity;
mod causal;
mod predictor;
mod uncertainty;

pub use transition::{TransitionModel, StateTransition, TransitionPrediction};
pub use entity::{EntityTracker, EntityState, EntityObservation, Anomaly};
pub use causal::{CausalGraph, CausalNode, CausalEdge, InterventionQuery};
pub use predictor::{PredictiveModel, LearnedTransitionPredictor, PredictionResult};
pub use uncertainty::{UncertaintyEstimator, ConfidenceInterval, CalibrationMetrics};

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Central World-Model coordinator integrating all predictive capabilities
pub struct WorldModelEnhanced {
    /// State transition model (Dirichlet-Multinomial)
    transitions: Arc<RwLock<TransitionModel>>,
    
    /// Entity trackers (Kalman filters)
    entities: Arc<RwLock<HashMap<String, EntityTracker>>>,
    
    /// Causal graph for causal reasoning
    causal_graph: Arc<RwLock<CausalGraph>>,
    
    /// Learned predictive models
    predictors: Arc<RwLock<Vec<Box<dyn PredictiveModel + Send + Sync>>>>,
    
    /// Uncertainty estimator
    uncertainty: Arc<RwLock<UncertaintyEstimator>>,
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
        let mut transitions = self.transitions.write()
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
        let transitions = self.transitions.read()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;
        
        transitions.predict(current_state, action)
    }

    /// Get transition uncertainty (Shannon entropy)
    pub fn get_transition_uncertainty(
        &self,
        state: &str,
        action: &str,
    ) -> Result<f64, String> {
        let transitions = self.transitions.read()
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
        let mut entities = self.entities.write()
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
        let mut entities = self.entities.write()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;
        
        let tracker = entities.get_mut(entity_id)
            .ok_or_else(|| format!("Entity '{}' not found", entity_id))?;
        
        tracker.update(observation)
    }

    /// Predict entity state N steps into future
    pub fn predict_entity(
        &self,
        entity_id: &str,
        steps: usize,
    ) -> Result<EntityState, String> {
        let entities = self.entities.read()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;
        
        let tracker = entities.get(entity_id)
            .ok_or_else(|| format!("Entity '{}' not found", entity_id))?;
        
        tracker.predict(steps)
    }

    /// Check if entity has anomalies
    pub fn get_entity_anomalies(
        &self,
        entity_id: &str,
    ) -> Result<Vec<Anomaly>, String> {
        let entities = self.entities.read()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;
        
        let tracker = entities.get(entity_id)
            .ok_or_else(|| format!("Entity '{}' not found", entity_id))?;
        
        Ok(tracker.get_anomalies())
    }

    /// List all tracked entities
    pub fn list_entities(&self) -> Result<Vec<String>, String> {
        let entities = self.entities.read()
            .map_err(|e| format!("Failed to acquire entities lock: {}", e))?;
        
        Ok(entities.keys().cloned().collect())
    }

    // ========================================================================
    // Causal Reasoning
    // ========================================================================

    /// Add causal edge A → B
    pub fn add_causal_edge(
        &self,
        from: String,
        to: String,
    ) -> Result<(), String> {
        let mut graph = self.causal_graph.write()
            .map_err(|e| format!("Failed to acquire causal graph lock: {}", e))?;
        
        graph.add_edge(from, to)
    }

    /// Check if A causally affects B (path exists)
    pub fn has_causal_path(
        &self,
        from: &str,
        to: &str,
    ) -> Result<bool, String> {
        let graph = self.causal_graph.read()
            .map_err(|e| format!("Failed to acquire causal graph lock: {}", e))?;
        
        graph.has_path(from, to)
    }

    /// Perform causal intervention P(Y|do(X=x))
    pub fn causal_intervention(
        &self,
        query: InterventionQuery,
    ) -> Result<HashMap<String, f64>, String> {
        let graph = self.causal_graph.read()
            .map_err(|e| format!("Failed to acquire causal graph lock: {}", e))?;
        
        graph.compute_intervention(&query)
    }

    /// Compute counterfactual "what if X had been x instead of x'?"
    pub fn counterfactual(
        &self,
        actual_state: HashMap<String, f64>,
        intervention_var: String,
        intervention_value: f64,
    ) -> Result<HashMap<String, f64>, String> {
        let graph = self.causal_graph.read()
            .map_err(|e| format!("Failed to acquire causal graph lock: {}", e))?;
        
        graph.compute_counterfactual(actual_state, intervention_var, intervention_value)
    }

    // ========================================================================
    // Predictive Models
    // ========================================================================

    /// Train predictive model from transition history
    pub fn train_predictor(&self) -> Result<(), String> {
        let transitions = self.transitions.read()
            .map_err(|e| format!("Failed to acquire transitions lock: {}", e))?;
        
        if transitions.observation_count() < 100 {
            return Err("Need at least 100 observations to train predictor".to_string());
        }

        let predictor = LearnedTransitionPredictor::train(&transitions)?;
        
        let mut predictors = self.predictors.write()
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
        let predictors = self.predictors.read()
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

    // ========================================================================
    // Uncertainty Quantification
    // ========================================================================

    /// Get confidence interval for prediction
    pub fn get_prediction_confidence(
        &self,
        state: &str,
        action: &str,
    ) -> Result<ConfidenceInterval, String> {
        let uncertainty = self.uncertainty.read()
            .map_err(|e| format!("Failed to acquire uncertainty lock: {}", e))?;
        
        let prediction = self.predict_next_state(state, action)?;
        
        uncertainty.compute_confidence_interval(&prediction)
    }

    /// Get calibration metrics
    pub fn get_calibration_metrics(&self) -> Result<CalibrationMetrics, String> {
        let uncertainty = self.uncertainty.read()
            .map_err(|e| format!("Failed to acquire uncertainty lock: {}", e))?;
        
        Ok(uncertainty.get_metrics())
    }

    /// Decompose uncertainty into epistemic and aleatoric
    pub fn decompose_uncertainty(
        &self,
        state: &str,
        action: &str,
    ) -> Result<(f64, f64), String> {
        let uncertainty = self.uncertainty.read()
            .map_err(|e| format!("Failed to acquire uncertainty lock: {}", e))?;
        
        uncertainty.decompose(state, action)
    }

    /// Return all causal edges for serialization.
    pub fn get_causal_edges(&self) -> Vec<CausalEdge> {
        self.causal_graph.read()
            .map(|g| g.all_edges())
            .unwrap_or_default()
    }

    /// Total number of state transitions observed across all (state, action) pairs.
    pub fn transition_count(&self) -> usize {
        self.transitions.read()
            .map(|t| t.observation_count())
            .unwrap_or(0)
    }

    /// All unique states observed in transition data.
    pub fn get_states(&self) -> Vec<String> {
        self.transitions.read()
            .map(|t| t.get_states())
            .unwrap_or_default()
    }

    /// All unique actions observed in transition data.
    pub fn get_actions(&self) -> Vec<String> {
        self.transitions.read()
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
        let mut result: Vec<(String, String, f64)> = transitions.totals
            .keys()
            .filter_map(|(state, action)| {
                transitions.compute_entropy(state, action).ok()
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
        let transitions = self.transitions.read()
            .map_err(|e| anyhow::anyhow!("transitions lock error: {}", e))?;
        let causal = self.causal_graph.read()
            .map_err(|e| anyhow::anyhow!("causal lock error: {}", e))?;

        // Encode HashMap<(String,String,String), usize> → HashMap<String, usize>
        // using \x1F (ASCII Unit Separator) as separator — safe in any UTF-8 string
        let counts_encoded: std::collections::HashMap<String, usize> = transitions.counts
            .iter()
            .map(|((s, a, ns), &v)| (format!("{}\x1F{}\x1F{}", s, a, ns), v))
            .collect();
        let totals_encoded: std::collections::HashMap<String, usize> = transitions.totals
            .iter()
            .map(|((s, a), &v)| (format!("{}\x1F{}", s, a), v))
            .collect();

        let causal_edges: Vec<serde_json::Value> = causal.all_edges().iter().map(|e| {
            serde_json::json!({"from": e.from, "to": e.to, "strength": e.strength})
        }).collect();

        let data = serde_json::json!({
            "version": 1,
            "transition_counts": counts_encoded,
            "transition_totals": totals_encoded,
            "smoothing": transitions.smoothing(),
            "causal_edges": causal_edges,
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
            let mut transitions = wm.transitions.write()
                .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            let smoothing = data["smoothing"].as_f64().unwrap_or(1.0);
            *transitions = TransitionModel::with_smoothing(smoothing);

            if let Some(obj) = data["transition_counts"].as_object() {
                for (k, v) in obj {
                    let parts: Vec<&str> = k.splitn(3, '\x1F').collect();
                    if parts.len() == 3 {
                        if let Some(count) = v.as_u64() {
                            transitions.counts.insert(
                                (parts[0].to_string(), parts[1].to_string(), parts[2].to_string()),
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
            let mut causal = wm.causal_graph.write()
                .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            if let Some(arr) = data["causal_edges"].as_array() {
                for e in arr {
                    let from = e["from"].as_str().unwrap_or("").to_string();
                    let to   = e["to"].as_str().unwrap_or("").to_string();
                    if !from.is_empty() && !to.is_empty() {
                        let _ = causal.add_edge(from, to); // ignore cycle prevention errors on load
                    }
                }
            }
        }

        Ok(wm)
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
        wm.observe_transition("S1".to_string(), "A1".to_string(), "S2".to_string()).unwrap();
        wm.observe_transition("S1".to_string(), "A1".to_string(), "S2".to_string()).unwrap();
        wm.observe_transition("S1".to_string(), "A1".to_string(), "S3".to_string()).unwrap();

        // Predict should return distribution
        let pred = wm.predict_next_state("S1", "A1").unwrap();
        assert!(pred.probabilities.len() >= 2);
        
        // S2 should be more likely than S3 (2/3 vs 1/3)
        let p_s2 = pred.probabilities.get("S2").unwrap_or(&0.0);
        let p_s3 = pred.probabilities.get("S3").unwrap_or(&0.0);
        assert!(p_s2 > p_s3);
    }

    #[test]
    fn test_entity_registration() {
        let wm = WorldModelEnhanced::new();
        
        let initial_state = EntityState {
            properties: vec![1.0, 2.0, 3.0],
            covariance: vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0], vec![0.0, 0.0, 1.0]],
        };

        assert!(wm.register_entity("entity1".to_string(), initial_state).is_ok());
        
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
        assert!(!wm.has_causal_path("C", "A").unwrap());  // No reverse path
    }

    #[test]
    fn test_cycle_prevention() {
        let wm = WorldModelEnhanced::new();
        
        wm.add_causal_edge("A".to_string(), "B".to_string()).unwrap();
        wm.add_causal_edge("B".to_string(), "C".to_string()).unwrap();
        
        // Creating cycle A → B → C → A should fail
        assert!(wm.add_causal_edge("C".to_string(), "A".to_string()).is_err());
    }
}
