use crate::memory_record::MemoryType;
use crate::memory_store::MemoryStore;
use crate::payloads::BeliefPayload;
use crate::persistence::MemoryBackend;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationState {
    pub prediction_error_ewma: f32,
    pub calibration_score: f32,
    pub consolidation_pressure: f32,
    pub epistemic_entropy: f32,
    pub current_tx: u64,
    pub last_updated_ms: u64,
    pub healthy: bool,
}

impl Default for CalibrationState {
    fn default() -> Self {
        Self {
            prediction_error_ewma: 0.0,
            calibration_score: 1.0,
            consolidation_pressure: 0.0,
            epistemic_entropy: 0.0,
            current_tx: 0,
            last_updated_ms: 0,
            healthy: true,
        }
    }
}

pub struct CalibrationTracker {
    state: Arc<RwLock<CalibrationState>>,
    alpha: f32,
}

impl CalibrationTracker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(CalibrationState::default())),
            alpha: 0.1,
        }
    }

    /// Call after WorldModel::predict() vs observed outcome.
    /// error = 0.0 if prediction matched, 1.0 if missed.
    pub fn record_prediction_error(&self, error: f32) {
        if let Ok(mut s) = self.state.write() {
            s.prediction_error_ewma =
                self.alpha * error + (1.0 - self.alpha) * s.prediction_error_ewma;
            s.calibration_score = (1.0 - s.prediction_error_ewma).clamp(0.0, 1.0);
            s.healthy = s.calibration_score >= 0.70 && s.consolidation_pressure <= 0.90;
            s.last_updated_ms = now_ms();
        }
    }

    /// Call on every MemoryStore::add() — recomputes pressure and H(B).
    pub fn update_from_store<B: MemoryBackend>(
        &self,
        store: &MemoryStore<B>,
        pressure: f32,
        current_tx: u64,
    ) {
        let confidences: Vec<f32> = store
            .all()
            .iter()
            .filter(|r| r.record_type == MemoryType::Belief)
            .filter_map(|r| {
                serde_json::from_value::<BeliefPayload>(r.metadata.clone()).ok()
            })
            .map(|b| b.confidence)
            .collect();
        let entropy = epistemic_entropy(&confidences);
        if let Ok(mut s) = self.state.write() {
            s.consolidation_pressure = pressure;
            s.epistemic_entropy = entropy;
            s.current_tx = current_tx;
            s.healthy = s.calibration_score >= 0.70 && s.consolidation_pressure <= 0.90;
            s.last_updated_ms = now_ms();
        }
    }

    pub fn snapshot(&self) -> CalibrationState {
        self.state.read().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Default for CalibrationTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// H(B) = -Σ p_i * log₂(p_i) over beliefs where p_i ∈ (0, 1].
fn epistemic_entropy(confidences: &[f32]) -> f32 {
    if confidences.is_empty() {
        return 0.0;
    }
    confidences
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.log2())
        .sum()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
