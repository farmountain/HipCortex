/// Tracks reasoning effort for collapse resistance metrics.
#[derive(Default)]
pub struct EffortEvaluator {
    effort: usize,
}

impl EffortEvaluator {
    pub fn new() -> Self {
        Self { effort: 0 }
    }

    /// Record a reasoning step with an associated cost.
    ///
    /// The cost can represent token usage, elapsed time, or any unit of
    /// computational effort. Larger costs increase the evaluator's effort
    /// counter proportionally.
    pub fn record(&mut self, cost: usize) {
        self.effort += cost;
    }

    /// Convenience method for recording a single unit of effort.
    pub fn record_step(&mut self) {
        self.record(1);
    }

    pub fn effort(&self) -> usize {
        self.effort
    }

    /// Collapse score combines effort and confidence into a bounded metric.
    ///
    /// The formula is:
    /// `collapse_score = normalized_effort * (1 - confidence)`
    /// where `normalized_effort = effort / (effort + 1)` ensures the result
    /// remains in `[0,1]`.
    pub fn collapse_score(&self, confidence: f32) -> f32 {
        let normalized_effort = self.effort as f32 / (self.effort as f32 + 1.0);
        normalized_effort * (1.0 - confidence.clamp(0.0, 1.0))
    }

    /// Returns true if the collapse score is greater than or equal to
    /// `threshold`.
    pub fn is_collapse_imminent(&self, confidence: f32, threshold: f32) -> bool {
        self.collapse_score(confidence) >= threshold
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

    /// Linear decay: `C_new = (C_old - amount).clamp(0, 1)`.
    pub fn decay(&mut self, amount: f32) {
        self.confidence = (self.confidence - amount).clamp(0.0, 1.0);
    }

    /// Exponential decay: `C_new = C_old * (1 - rate)`.
    pub fn decay_exponential(&mut self, decay_rate: f32) {
        self.confidence *= (1.0 - decay_rate).max(0.0);
        self.confidence = self.confidence.clamp(0.0, 1.0);
    }

    /// Logarithmic decay: `C_new = C_old * 1/(1 + factor)`.
    pub fn decay_log(&mut self, decay_factor: f32) {
        self.confidence *= (1.0 / (1.0 + decay_factor)).max(0.0);
        self.confidence = self.confidence.clamp(0.0, 1.0);
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }
}
