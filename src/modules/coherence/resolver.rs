// Conflict Resolver - Automatic resolution strategies for detected inconsistencies
//
// Provides three resolution strategies:
// 1. Consensus: Select value appearing in majority of modules
// 2. Recency: Select most recently observed value based on timestamps
// 3. Confidence: Select value with highest confidence score

use super::checker::{InconsistencyReport, InconsistencyType};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Resolution strategies for conflict resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    /// Select value that appears in majority of modules
    Consensus,
    
    /// Select most recently observed value based on timestamps
    Recency,
    
    /// Select value with highest confidence score
    Confidence,
}

/// Result of a resolution attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// Unique identifier for the resolution
    pub id: String,
    
    /// ID of the inconsistency being resolved
    pub inconsistency_id: String,
    
    /// Strategy used for resolution
    pub strategy: ResolutionStrategy,
    
    /// Whether resolution succeeded
    pub success: bool,
    
    /// Chosen value (if successful)
    pub chosen_value: Option<serde_json::Value>,
    
    /// All candidate values considered
    pub candidates: Vec<CandidateValue>,
    
    /// Rationale for the choice
    pub rationale: String,
    
    /// Resolution timestamp (Unix epoch millis)
    pub resolved_at: u64,
}

/// Candidate value for resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateValue {
    /// The value itself
    pub value: serde_json::Value,
    
    /// Source module
    pub source: String,
    
    /// Timestamp when observed (Unix epoch millis)
    pub timestamp: u64,
    
    /// Confidence score [0.0, 1.0]
    pub confidence: f64,
}

/// Historical record of a resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionHistory {
    /// Resolution ID
    pub id: String,
    
    /// Inconsistency ID that was resolved
    pub inconsistency_id: String,
    
    /// Inconsistency type
    pub inconsistency_type: InconsistencyType,
    
    /// Strategy used
    pub strategy: ResolutionStrategy,
    
    /// Success status
    pub success: bool,
    
    /// Chosen value
    pub chosen_value: Option<serde_json::Value>,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// Operator who manually overrode (if any)
    pub manual_override_by: Option<String>,
}

/// Conflict resolver with pluggable strategies
pub struct ConflictResolver {
    /// Resolution history (for audit)
    history: Vec<ResolutionHistory>,
    
    /// Manual overrides (inconsistency_id -> disabled auto-resolution)
    manual_overrides: HashMap<String, String>,
    
    /// Consensus threshold (fraction of modules that must agree)
    consensus_threshold: f64,
}

impl ConflictResolver {
    /// Create new conflict resolver with default settings
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            manual_overrides: HashMap::new(),
            consensus_threshold: 0.5, // Simple majority
        }
    }

    /// Create with custom consensus threshold
    pub fn with_consensus_threshold(threshold: f64) -> Self {
        Self {
            history: Vec::new(),
            manual_overrides: HashMap::new(),
            consensus_threshold: threshold,
        }
    }

    // ========================================================================
    // Resolution APIs
    // ========================================================================

    /// Resolve an inconsistency using specified strategy
    pub fn resolve(
        &mut self,
        inconsistency: &InconsistencyReport,
        strategy: ResolutionStrategy,
    ) -> Result<ResolutionResult, String> {
        // Check if manual override exists
        if self.manual_overrides.contains_key(&inconsistency.id) {
            return Err(format!(
                "Inconsistency {} has manual override - auto-resolution disabled",
                inconsistency.id
            ));
        }

        // Get candidate values (in real implementation, query modules)
        let candidates = self.get_candidate_values(inconsistency)?;
        
        if candidates.is_empty() {
            return Ok(self.create_failed_result(
                inconsistency,
                strategy,
                Vec::new(),
                "No candidate values available".to_string(),
            ));
        }

        // Apply strategy
        let result = match strategy {
            ResolutionStrategy::Consensus => self.resolve_by_consensus(inconsistency, candidates),
            ResolutionStrategy::Recency => self.resolve_by_recency(inconsistency, candidates),
            ResolutionStrategy::Confidence => self.resolve_by_confidence(inconsistency, candidates),
        }?;

        // Record in history
        self.record_resolution(&result, inconsistency, None);

        Ok(result)
    }

    /// Apply manual override for an inconsistency
    pub fn apply_manual_override(
        &mut self,
        inconsistency_id: &str,
        chosen_value: serde_json::Value,
        operator: &str,
    ) -> Result<(), String> {
        // Record manual override
        self.manual_overrides.insert(
            inconsistency_id.to_string(),
            operator.to_string(),
        );

        // Create resolution result for history
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let result = ResolutionResult {
            id: format!("manual_{}_{}", inconsistency_id, now),
            inconsistency_id: inconsistency_id.to_string(),
            strategy: ResolutionStrategy::Consensus, // Placeholder
            success: true,
            chosen_value: Some(chosen_value),
            candidates: Vec::new(),
            rationale: format!("Manual override by operator {}", operator),
            resolved_at: now,
        };

        // Record in history with operator info
        self.history.push(ResolutionHistory {
            id: result.id.clone(),
            inconsistency_id: inconsistency_id.to_string(),
            inconsistency_type: InconsistencyType::TemporalSymbolicMismatch, // Placeholder
            strategy: ResolutionStrategy::Consensus,
            success: true,
            chosen_value: result.chosen_value.clone(),
            timestamp: now,
            manual_override_by: Some(operator.to_string()),
        });

        Ok(())
    }

    /// Get resolution history
    pub fn get_history(&self) -> Result<Vec<ResolutionHistory>, String> {
        Ok(self.history.clone())
    }

    // ========================================================================
    // Resolution Strategies
    // ========================================================================

    /// Resolve by consensus (majority vote)
    fn resolve_by_consensus(
        &self,
        inconsistency: &InconsistencyReport,
        candidates: Vec<CandidateValue>,
    ) -> Result<ResolutionResult, String> {
        // Count occurrences of each value
        let mut value_counts: HashMap<String, (usize, CandidateValue)> = HashMap::new();
        
        for candidate in &candidates {
            let key = serde_json::to_string(&candidate.value)
                .unwrap_or_else(|_| format!("{:?}", candidate.value));
            
            value_counts.entry(key.clone())
                .and_modify(|(count, _)| *count += 1)
                .or_insert((1, candidate.clone()));
        }

        // Find value with most votes
        let total_candidates = candidates.len();
        let mut max_count = 0;
        let mut chosen: Option<&CandidateValue> = None;

        for (_key, (count, candidate)) in &value_counts {
            if *count > max_count {
                max_count = *count;
                chosen = Some(candidate);
            }
        }

        // Check if consensus threshold met
        let consensus_fraction = max_count as f64 / total_candidates as f64;
        
        if consensus_fraction < self.consensus_threshold {
            return Ok(self.create_failed_result(
                inconsistency,
                ResolutionStrategy::Consensus,
                candidates,
                format!(
                    "No consensus reached: {}/{} votes ({:.2}%) < threshold ({:.2}%)",
                    max_count, total_candidates,
                    consensus_fraction * 100.0,
                    self.consensus_threshold * 100.0
                ),
            ));
        }

        let chosen_candidate = chosen.unwrap();
        Ok(self.create_success_result(
            inconsistency,
            ResolutionStrategy::Consensus,
            candidates,
            chosen_candidate.value.clone(),
            format!(
                "Consensus reached: {}/{} modules agree ({:.2}%)",
                max_count, total_candidates, consensus_fraction * 100.0
            ),
        ))
    }

    /// Resolve by recency (most recent timestamp)
    fn resolve_by_recency(
        &self,
        inconsistency: &InconsistencyReport,
        candidates: Vec<CandidateValue>,
    ) -> Result<ResolutionResult, String> {
        let most_recent = candidates.iter()
            .max_by_key(|c| c.timestamp)
            .ok_or_else(|| "No candidates available".to_string())?;
        
        let value = most_recent.value.clone();
        let source = most_recent.source.clone();
        let timestamp = most_recent.timestamp;

        Ok(self.create_success_result(
            inconsistency,
            ResolutionStrategy::Recency,
            candidates,
            value,
            format!(
                "Selected most recent value from {} (timestamp={})",
                source, timestamp
            ),
        ))
    }

    /// Resolve by confidence (highest confidence score)
    fn resolve_by_confidence(
        &self,
        inconsistency: &InconsistencyReport,
        candidates: Vec<CandidateValue>,
    ) -> Result<ResolutionResult, String> {
        let highest_confidence = candidates.iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .ok_or_else(|| "No candidates available".to_string())?;
        
        let value = highest_confidence.value.clone();
        let source = highest_confidence.source.clone();
        let confidence = highest_confidence.confidence;

        Ok(self.create_success_result(
            inconsistency,
            ResolutionStrategy::Confidence,
            candidates,
            value,
            format!(
                "Selected highest confidence value from {} (confidence={:.4})",
                source, confidence
            ),
        ))
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Get candidate values from modules (placeholder - real impl would query modules)
    fn get_candidate_values(
        &self,
        _inconsistency: &InconsistencyReport,
    ) -> Result<Vec<CandidateValue>, String> {
        // In real implementation, this would:
        // 1. Query temporal indexer, symbolic store, procedural cache, world-model
        // 2. Extract conflicting values with timestamps and confidence
        // 3. Return as CandidateValue vec
        
        // For now, return empty (will be populated during integration)
        Ok(Vec::new())
    }

    /// Create successful resolution result
    fn create_success_result(
        &self,
        inconsistency: &InconsistencyReport,
        strategy: ResolutionStrategy,
        candidates: Vec<CandidateValue>,
        chosen_value: serde_json::Value,
        rationale: String,
    ) -> ResolutionResult {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        ResolutionResult {
            id: format!("resolution_{}_{}", inconsistency.id, now),
            inconsistency_id: inconsistency.id.clone(),
            strategy,
            success: true,
            chosen_value: Some(chosen_value),
            candidates,
            rationale,
            resolved_at: now,
        }
    }

    /// Create failed resolution result
    fn create_failed_result(
        &self,
        inconsistency: &InconsistencyReport,
        strategy: ResolutionStrategy,
        candidates: Vec<CandidateValue>,
        rationale: String,
    ) -> ResolutionResult {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        ResolutionResult {
            id: format!("resolution_failed_{}_{}", inconsistency.id, now),
            inconsistency_id: inconsistency.id.clone(),
            strategy,
            success: false,
            chosen_value: None,
            candidates,
            rationale,
            resolved_at: now,
        }
    }

    /// Record resolution in history
    fn record_resolution(
        &mut self,
        result: &ResolutionResult,
        inconsistency: &InconsistencyReport,
        operator: Option<String>,
    ) {
        self.history.push(ResolutionHistory {
            id: result.id.clone(),
            inconsistency_id: result.inconsistency_id.clone(),
            inconsistency_type: inconsistency.inconsistency_type.clone(),
            strategy: result.strategy,
            success: result.success,
            chosen_value: result.chosen_value.clone(),
            timestamp: result.resolved_at,
            manual_override_by: operator,
        });
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coherence::checker::InconsistencyType;

    fn create_test_inconsistency() -> InconsistencyReport {
        InconsistencyReport::new(
            InconsistencyType::TemporalSymbolicMismatch,
            vec!["entity1".to_string()],
            "Test inconsistency".to_string(),
            5,
        )
    }

    fn create_test_candidates() -> Vec<CandidateValue> {
        vec![
            CandidateValue {
                value: serde_json::json!("value_A"),
                source: "temporal".to_string(),
                timestamp: 1000,
                confidence: 0.8,
            },
            CandidateValue {
                value: serde_json::json!("value_A"),
                source: "symbolic".to_string(),
                timestamp: 1100,
                confidence: 0.9,
            },
            CandidateValue {
                value: serde_json::json!("value_B"),
                source: "world_model".to_string(),
                timestamp: 1200,
                confidence: 0.7,
            },
        ]
    }

    #[test]
    fn test_resolver_creation() {
        let resolver = ConflictResolver::new();
        assert_eq!(resolver.consensus_threshold, 0.5);
        assert_eq!(resolver.history.len(), 0);
    }

    #[test]
    fn test_custom_consensus_threshold() {
        let resolver = ConflictResolver::with_consensus_threshold(0.75);
        assert_eq!(resolver.consensus_threshold, 0.75);
    }

    #[test]
    fn test_resolve_by_consensus_success() {
        let resolver = ConflictResolver::new();
        let inconsistency = create_test_inconsistency();
        let candidates = create_test_candidates();
        
        // value_A appears 2/3 times (66%), should reach consensus
        let result = resolver.resolve_by_consensus(&inconsistency, candidates).unwrap();
        
        assert!(result.success);
        assert_eq!(result.chosen_value, Some(serde_json::json!("value_A")));
        assert!(result.rationale.contains("Consensus reached"));
    }

    #[test]
    fn test_resolve_by_consensus_failure() {
        let resolver = ConflictResolver::with_consensus_threshold(0.8);
        let inconsistency = create_test_inconsistency();
        let candidates = create_test_candidates();
        
        // value_A: 2/3 = 66% < 80% threshold
        let result = resolver.resolve_by_consensus(&inconsistency, candidates).unwrap();
        
        assert!(!result.success);
        assert_eq!(result.chosen_value, None);
        assert!(result.rationale.contains("No consensus reached"));
    }

    #[test]
    fn test_resolve_by_recency() {
        let resolver = ConflictResolver::new();
        let inconsistency = create_test_inconsistency();
        let candidates = create_test_candidates();
        
        // Most recent is world_model at timestamp 1200
        let result = resolver.resolve_by_recency(&inconsistency, candidates).unwrap();
        
        assert!(result.success);
        assert_eq!(result.chosen_value, Some(serde_json::json!("value_B")));
        assert!(result.rationale.contains("most recent"));
        assert!(result.rationale.contains("1200"));
    }

    #[test]
    fn test_resolve_by_confidence() {
        let resolver = ConflictResolver::new();
        let inconsistency = create_test_inconsistency();
        let candidates = create_test_candidates();
        
        // Highest confidence is symbolic at 0.9
        let result = resolver.resolve_by_confidence(&inconsistency, candidates).unwrap();
        
        assert!(result.success);
        assert_eq!(result.chosen_value, Some(serde_json::json!("value_A")));
        assert!(result.rationale.contains("highest confidence"));
        assert!(result.rationale.contains("0.9"));
    }

    #[test]
    fn test_manual_override() {
        let mut resolver = ConflictResolver::new();
        let inconsistency_id = "test_inconsistency";
        let override_value = serde_json::json!("manual_value");
        
        resolver.apply_manual_override(
            inconsistency_id,
            override_value.clone(),
            "operator123",
        ).unwrap();
        
        // Check override recorded
        assert!(resolver.manual_overrides.contains_key(inconsistency_id));
        
        // Check history recorded
        assert_eq!(resolver.history.len(), 1);
        let history = &resolver.history[0];
        assert_eq!(history.manual_override_by, Some("operator123".to_string()));
        assert_eq!(history.chosen_value, Some(override_value));
    }

    #[test]
    fn test_resolve_with_manual_override_blocks_auto() {
        let mut resolver = ConflictResolver::new();
        let inconsistency = create_test_inconsistency();
        
        // Apply manual override
        resolver.manual_overrides.insert(
            inconsistency.id.clone(),
            "operator".to_string(),
        );
        
        // Try auto-resolution (should fail)
        let result = resolver.resolve(&inconsistency, ResolutionStrategy::Consensus);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("manual override"));
    }

    #[test]
    fn test_resolution_history_tracking() {
        let mut resolver = ConflictResolver::new();
        
        // Apply manual override
        resolver.apply_manual_override(
            "inconsistency1",
            serde_json::json!("value1"),
            "operator1",
        ).unwrap();
        
        let history = resolver.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].inconsistency_id, "inconsistency1");
    }

    #[test]
    fn test_empty_candidates_recency() {
        let resolver = ConflictResolver::new();
        let inconsistency = create_test_inconsistency();
        
        let result = resolver.resolve_by_recency(&inconsistency, Vec::new());
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No candidates"));
    }

    #[test]
    fn test_empty_candidates_confidence() {
        let resolver = ConflictResolver::new();
        let inconsistency = create_test_inconsistency();
        
        let result = resolver.resolve_by_confidence(&inconsistency, Vec::new());
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No candidates"));
    }

    #[test]
    fn test_resolution_result_serialization() {
        let result = ResolutionResult {
            id: "test_resolution".to_string(),
            inconsistency_id: "test_inconsistency".to_string(),
            strategy: ResolutionStrategy::Consensus,
            success: true,
            chosen_value: Some(serde_json::json!({"key": "value"})),
            candidates: Vec::new(),
            rationale: "Test rationale".to_string(),
            resolved_at: 1000,
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Consensus"));
        assert!(json.contains("Test rationale"));
    }

    #[test]
    fn test_candidate_value_creation() {
        let candidate = CandidateValue {
            value: serde_json::json!(42),
            source: "test_module".to_string(),
            timestamp: 1234567890,
            confidence: 0.95,
        };
        
        assert_eq!(candidate.confidence, 0.95);
        assert_eq!(candidate.source, "test_module");
    }
}
