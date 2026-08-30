//! Monitor-Generate-Verify metacognitive operator (Nelson-Narens).
use serde::{Deserialize, Serialize};

pub struct MGVOperator {
    justification_strength: f64,
    calibration_score: f64,
    historical_success_rate: f64,
    jtms_consistency_score: f64,
    empirical_delta_outcome: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MGVResult {
    pub fok: f64,
    pub jol: f64,
    pub divergence: f64,
    pub should_quarantine: bool,
}

impl MGVOperator {
    pub fn new(justification_strength: f64, calibration_score: f64, historical_success_rate: f64) -> Self {
        Self {
            justification_strength,
            calibration_score,
            historical_success_rate,
            jtms_consistency_score: 0.8,
            empirical_delta_outcome: 0.7,
        }
    }

    pub fn fok(&self) -> f64 {
        (self.justification_strength * self.calibration_score * self.historical_success_rate).clamp(0.0, 1.0)
    }

    pub fn jol(&self) -> f64 {
        (self.empirical_delta_outcome * self.jtms_consistency_score).clamp(0.0, 1.0)
    }

    pub fn check(&self) -> MGVResult {
        let fok = self.fok();
        let jol = self.jol();
        let divergence = jol - fok;
        MGVResult { fok, jol, divergence, should_quarantine: divergence.abs() > 0.3 }
    }
}
