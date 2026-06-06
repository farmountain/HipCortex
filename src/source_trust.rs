// Chain-of-Thought: track credibility of memory sources over time using
// Bayesian updates (Beta distribution). Integrates with search to weight
// results by source trustworthiness, preventing adversarial memory injection.
// Unknown sources start at trust=0.5 (neutral), corroboration increases
// trust, contradiction decreases it. Trust decays for inactive sources.

use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// How to categorize a memory source.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SourceCategory {
    User,
    LLM,
    System,
    Sensor,
    External,
    Unknown,
}

impl Default for SourceCategory {
    fn default() -> Self {
        SourceCategory::Unknown
    }
}

/// Per-source credibility profile maintained via Bayesian updates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceTrustProfile {
    pub source_id: String,
    pub trust_score: f64, // 0.0-1.0, posterior mean of Beta(alpha, beta)
    pub alpha: f64,       // Beta prior: corroboration evidence
    pub beta: f64,        // Beta prior: contradiction evidence
    pub observation_count: u64,
    pub corroboration_count: u64,
    pub contradiction_count: u64,
    pub first_seen: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub category: SourceCategory,
}

impl SourceTrustProfile {
    pub fn new(source_id: String, category: SourceCategory) -> Self {
        let now = Utc::now();
        Self {
            source_id,
            trust_score: 0.5, // neutral for unknown sources
            alpha: 1.0,       // Beta(1,1) = uniform prior
            beta: 1.0,
            observation_count: 0,
            corroboration_count: 0,
            contradiction_count: 0,
            first_seen: now,
            last_updated: now,
            category,
        }
    }

    /// Compute current trust score from Beta posterior.
    pub fn recompute_trust(&mut self) {
        self.trust_score = self.alpha / (self.alpha + self.beta);
    }

    /// Record a corroboration (success) event.
    pub fn record_corroboration(&mut self) {
        self.alpha += 1.0;
        self.observation_count += 1;
        self.corroboration_count += 1;
        self.last_updated = Utc::now();
        self.recompute_trust();
    }

    /// Record a contradiction (failure) event.
    pub fn record_contradiction(&mut self) {
        self.beta += 1.0;
        self.observation_count += 1;
        self.contradiction_count += 1;
        self.last_updated = Utc::now();
        self.recompute_trust();
    }

    /// Decay trust if source hasn't been active for `max_days`.
    /// Moves alpha toward 0.5 weight (uniform prior).
    pub fn decay_if_stale(&mut self, max_days: i64) {
        let now = Utc::now();
        let days_since = (now - self.last_updated).num_days();
        if days_since > max_days {
            let decay_factor = 0.5_f64.powf((days_since - max_days) as f64 / 30.0);
            self.alpha = 1.0 + (self.alpha - 1.0) * decay_factor;
            self.beta = 1.0 + (self.beta - 1.0) * decay_factor;
            self.recompute_trust();
        }
    }
}

/// Registry of source trust profiles. Thread-safe behind an Arc<RwLock<>>.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SourceTrustRegistry {
    pub sources: HashMap<String, SourceTrustProfile>,
}

impl SourceTrustRegistry {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Get or create a trust profile for a source.
    pub fn get_or_create(&mut self, source_id: &str) -> &mut SourceTrustProfile {
        self.sources
            .entry(source_id.to_string())
            .or_insert_with(|| SourceTrustProfile::new(source_id.to_string(), SourceCategory::Unknown))
    }

    /// Get the trust score for a source. Returns 0.5 for unknown sources.
    pub fn get_trust(&self, source_id: &str) -> f64 {
        self.sources
            .get(source_id)
            .map(|p| p.trust_score)
            .unwrap_or(0.5)
    }

    /// Get a trust profile if it exists.
    pub fn get_profile(&self, source_id: &str) -> Option<&SourceTrustProfile> {
        self.sources.get(source_id)
    }

    /// Check if a source meets a minimum trust threshold.
    pub fn is_trusted(&self, source_id: &str, threshold: f64) -> bool {
        self.get_trust(source_id) >= threshold
    }

    /// Record a corroboration for a source.
    pub fn record_corroboration(&mut self, source_id: &str) {
        let profile = self.get_or_create(source_id);
        profile.record_corroboration();
    }

    /// Record a contradiction for a source.
    pub fn record_contradiction(&mut self, source_id: &str) {
        let profile = self.get_or_create(source_id);
        profile.record_contradiction();
    }

    /// Combine base confidence with source trust for a weighted score.
    pub fn trust_weighted_score(&self, base_confidence: f32, source_id: &str) -> f32 {
        let trust = self.get_trust(source_id) as f32;
        // Geometric mean of confidence and trust
        (base_confidence * trust).sqrt()
    }

    /// Decay trust for all sources that have been inactive.
    pub fn decay_all_stale(&mut self, max_days: i64) {
        for profile in self.sources.values_mut() {
            profile.decay_if_stale(max_days);
        }
    }

    /// Manually set a source's trust score (admin override).
    pub fn override_trust(&mut self, source_id: &str, score: f64, category: SourceCategory) {
        let profile = self.get_or_create(source_id);
        profile.trust_score = score.clamp(0.0, 1.0);
        profile.category = category;
        profile.last_updated = Utc::now();
    }

    /// List all source profiles.
    pub fn list_sources(&self) -> Vec<&SourceTrustProfile> {
        self.sources.values().collect()
    }

    /// Count registered sources.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_source_neutral_trust() {
        let registry = SourceTrustRegistry::new();
        assert_eq!(registry.get_trust("unknown-source"), 0.5);
    }

    #[test]
    fn test_corroboration_increases_trust() {
        let mut registry = SourceTrustRegistry::new();
        let initial = registry.get_trust("user-input");
        registry.record_corroboration("user-input");
        registry.record_corroboration("user-input");
        registry.record_corroboration("user-input");
        let after = registry.get_trust("user-input");
        assert!(after > initial, "trust should increase after corroboration");
        assert!(after <= 1.0, "trust should not exceed 1.0");
    }

    #[test]
    fn test_contradiction_decreases_trust() {
        let mut registry = SourceTrustRegistry::new();
        // Build up trust first
        registry.record_corroboration("llm-1");
        registry.record_corroboration("llm-1");
        let mid = registry.get_trust("llm-1");
        // Then contradict
        registry.record_contradiction("llm-1");
        registry.record_contradiction("llm-1");
        registry.record_contradiction("llm-1");
        let after = registry.get_trust("llm-1");
        assert!(after < mid, "trust should decrease after contradictions");
        assert!(after >= 0.0, "trust should not go below 0.0");
    }

    #[test]
    fn test_trust_weighted_score() {
        let mut registry = SourceTrustRegistry::new();
        // Neutral source: confidence * sqrt(0.5)
        let neutral = registry.trust_weighted_score(0.8, "unknown");
        assert!(neutral < 0.8, "weighted score should be less than raw confidence for neutral source");

        // Build trust
        for _ in 0..20 {
            registry.record_corroboration("trusted-sensor");
        }
        let high_trust = registry.trust_weighted_score(0.5, "trusted-sensor");
        // High trust source should boost low confidence somewhat
        assert!(high_trust > 0.35);
    }

    #[test]
    fn test_is_trusted() {
        let mut registry = SourceTrustRegistry::new();
        assert!(!registry.is_trusted("new-source", 0.7));
        for _ in 0..50 {
            registry.record_corroboration("new-source");
        }
        assert!(registry.is_trusted("new-source", 0.7));
    }

    #[test]
    fn test_override_trust() {
        let mut registry = SourceTrustRegistry::new();
        registry.override_trust("admin-source", 0.95, SourceCategory::System);
        assert_eq!(registry.get_trust("admin-source"), 0.95);
        let profile = registry.get_profile("admin-source").unwrap();
        assert!(matches!(profile.category, SourceCategory::System));
    }

    #[test]
    fn test_decay_stale_source() {
        // Create a source with high trust but old last_updated
        let mut profile = SourceTrustProfile::new("old-sensor".into(), SourceCategory::Sensor);
        // Artificially boost trust
        profile.alpha = 100.0;
        profile.beta = 1.0;
        profile.recompute_trust();
        let before = profile.trust_score;
        assert!(before > 0.9);

        // Set last_updated to 60 days ago
        profile.last_updated = Utc::now() - chrono::Duration::days(60);
        profile.decay_if_stale(30); // max 30 days before decay starts
        assert!(profile.trust_score < before, "stale trust should decay");
    }

    #[test]
    fn test_trust_bounded() {
        let mut registry = SourceTrustRegistry::new();
        for _ in 0..1000 {
            registry.record_corroboration("perfect-source");
        }
        let trust = registry.get_trust("perfect-source");
        assert!(trust > 0.99); // should approach but not exceed 1.0
    }

    #[test]
    fn test_list_sources() {
        let mut registry = SourceTrustRegistry::new();
        registry.record_corroboration("a");
        registry.record_corroboration("b");
        registry.record_contradiction("c");
        assert_eq!(registry.source_count(), 3);
        assert_eq!(registry.list_sources().len(), 3);
    }
}
