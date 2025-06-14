/// Tracks reasoning effort for collapse resistance metrics.
#[derive(Default)]
pub struct EffortEvaluator {
    effort: usize,
}

impl EffortEvaluator {
    pub fn new() -> Self {
        Self { effort: 0 }
    }

    /// Record a reasoning step.
    pub fn record(&mut self) {
        self.effort += 1;
    }

    pub fn effort(&self) -> usize {
        self.effort
    }
}

/// Maintains a confidence score that decays with effort.
#[derive(Default)]
pub struct ConfidenceRegulator {
    confidence: f32,
}

impl ConfidenceRegulator {
    pub fn new() -> Self {
        Self { confidence: 1.0 }
    }

    pub fn decay(&mut self, amount: f32) {
        self.confidence = (self.confidence - amount).max(0.0);
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }
}
