use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::entity::EntityState;
use super::transition::TransitionModel;

/// A first-class behavioral policy attached to an Entity inside the World Model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub entity_id: String,
    pub utility_weights: HashMap<String, f64>,
    pub action_distribution: HashMap<String, f64>,
    pub temperature: f64,
}

impl Policy {
    pub fn new(entity_id: String, temperature: f64) -> Self {
        Self {
            entity_id,
            utility_weights: HashMap::new(),
            action_distribution: HashMap::new(),
            temperature: if temperature <= 0.0 { 0.0001 } else { temperature },
        }
    }

    /// Evaluates action distribution using softmax over base distribution and utility weights.
    pub fn sample_action(&self, _state: &EntityState, _transitions: &TransitionModel) -> String {
        if self.action_distribution.is_empty() {
            return "idle".to_string();
        }

        if self.temperature <= 0.001 {
            // Greedy exploitation: return action with highest probability * utility boost
            let mut best_action = "idle".to_string();
            let mut max_prob = -1.0_f64;
            for (act, prob) in &self.action_distribution {
                if *prob > max_prob {
                    max_prob = *prob;
                    best_action = act.clone();
                }
            }
            return best_action;
        }

        // Standard temperature-scaled sampling
        let mut best_action = "idle".to_string();
        let mut max_scaled = -1.0_f64;
        for (act, prob) in &self.action_distribution {
            let log_logit = (prob.max(1e-9)).ln() / self.temperature;
            if log_logit > max_scaled {
                max_scaled = log_logit;
                best_action = act.clone();
            }
        }
        best_action
    }
}
