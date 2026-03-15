// Consistency Checker - Detect inconsistencies across memory subsystems
//
// Detects five types of inconsistencies:
// 1. TemporalSymbolicMismatch: Event references missing entity
// 2. ProceduralWorldConflict: FSM allows transition with P=0
// 3. CausalViolation: Event sequence violates causal constraints
// 4. EntityPermanenceViolation: Entity exists in world-model but deleted from symbolic
// 5. GraphInconsistency: Symbolic DAG contradicts world-model causal graph

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Types of inconsistencies that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InconsistencyType {
    /// Event in temporal indexer references entity missing from symbolic store
    TemporalSymbolicMismatch,
    
    /// Procedural FSM allows transition but world-model predicts P=0
    ProceduralWorldConflict,
    
    /// Observed event sequence violates causal constraints
    CausalViolation,
    
    /// Entity exists in world-model but has been deleted from symbolic store
    EntityPermanenceViolation,
    
    /// Symbolic DAG contradicts world-model causal graph (edit distance > threshold)
    GraphInconsistency,
}

/// Report of detected inconsistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InconsistencyReport {
    /// Unique identifier for this inconsistency
    pub id: String,
    
    /// Type of inconsistency
    pub inconsistency_type: InconsistencyType,
    
    /// Affected entity IDs
    pub affected_entities: Vec<String>,
    
    /// Detection timestamp (Unix epoch millis)
    pub detected_at: u64,
    
    /// Detailed description
    pub description: String,
    
    /// Severity level (0-10, where 10 is critical)
    pub severity: u8,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl InconsistencyReport {
    pub fn new(
        inconsistency_type: InconsistencyType,
        affected_entities: Vec<String>,
        description: String,
        severity: u8,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let id = format!("{:?}_{}_{}",
            inconsistency_type,
            affected_entities.join("_"),
            now
        );
        
        Self {
            id,
            inconsistency_type,
            affected_entities,
            detected_at: now,
            description,
            severity,
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Consistency checker for cross-module validation
pub struct ConsistencyChecker {
    /// Threshold for graph edit distance (inconsistent if distance > threshold)
    graph_edit_distance_threshold: usize,
    
    /// Probability threshold for procedural-world conflicts (P < threshold is conflict)
    probability_threshold: f64,
    
    /// Entity count cache for efficiency
    entity_cache: HashMap<String, EntityCounts>,
}

/// Entity counts across different modules
#[derive(Debug, Clone)]
struct EntityCounts {
    temporal_count: usize,
    symbolic_count: usize,
    world_model_count: usize,
    last_updated: u64,
}

impl ConsistencyChecker {
    /// Create new consistency checker with default thresholds
    pub fn new() -> Self {
        Self {
            graph_edit_distance_threshold: 5,
            probability_threshold: 0.01,
            entity_cache: HashMap::new(),
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(graph_edit_threshold: usize, prob_threshold: f64) -> Self {
        Self {
            graph_edit_distance_threshold: graph_edit_threshold,
            probability_threshold: prob_threshold,
            entity_cache: HashMap::new(),
        }
    }

    // ========================================================================
    // Main Checking APIs
    // ========================================================================

    /// Run all consistency checks across modules
    pub fn check_all(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();
        
        // 1. Check temporal-symbolic consistency
        inconsistencies.extend(self.check_temporal_symbolic()?);
        
        // 2. Check procedural-world consistency
        inconsistencies.extend(self.check_procedural_world()?);
        
        // 3. Check causal violations
        inconsistencies.extend(self.check_causal_violations()?);
        
        // 4. Check entity permanence
        inconsistencies.extend(self.check_entity_permanence()?);
        
        // 5. Check graph consistency
        inconsistencies.extend(self.check_graph_consistency()?);
        
        Ok(inconsistencies)
    }

    /// Check consistency for specific entity (targeted check)
    pub fn check_entity(&mut self, entity_id: &str) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();
        
        // Check if entity counts match across modules
        let counts = self.get_entity_counts(entity_id)?;
        
        if counts.temporal_count != counts.symbolic_count {
            inconsistencies.push(InconsistencyReport::new(
                InconsistencyType::TemporalSymbolicMismatch,
                vec![entity_id.to_string()],
                format!(
                    "Entity {} has {} references in temporal indexer but {} in symbolic store",
                    entity_id, counts.temporal_count, counts.symbolic_count
                ),
                7,
            ));
        }
        
        if counts.world_model_count > 0 && counts.symbolic_count == 0 {
            inconsistencies.push(InconsistencyReport::new(
                InconsistencyType::EntityPermanenceViolation,
                vec![entity_id.to_string()],
                format!(
                    "Entity {} exists in world-model ({} refs) but deleted from symbolic store",
                    entity_id, counts.world_model_count
                ),
                8,
            ));
        }
        
        Ok(inconsistencies)
    }

    // ========================================================================
    // Inconsistency Detection Methods
    // ========================================================================

    /// Check for temporal-symbolic mismatches
    ///
    /// Detects: Event in temporal indexer references entity missing from symbolic store
    fn check_temporal_symbolic(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();
        
        // In a real implementation, this would query temporal indexer and symbolic store
        // For now, we provide the structure
        
        // Pseudo-code:
        // 1. Get all events from temporal indexer
        // 2. For each event, extract referenced entity IDs
        // 3. Check if each entity exists in symbolic store
        // 4. If entity missing, create TemporalSymbolicMismatch report
        
        // Example detection (placeholder):
        // let events = temporal_indexer.get_recent_events()?;
        // for event in events {
        //     for entity_id in event.referenced_entities {
        //         if !symbolic_store.contains(&entity_id) {
        //             inconsistencies.push(InconsistencyReport::new(
        //                 InconsistencyType::TemporalSymbolicMismatch,
        //                 vec![entity_id.clone()],
        //                 format!("Event {} references missing entity {}", event.id, entity_id),
        //                 6,
        //             ));
        //         }
        //     }
        // }
        
        Ok(inconsistencies)
    }

    /// Check for procedural-world conflicts
    ///
    /// Detects: Procedural FSM allows transition but world-model predicts P=0
    fn check_procedural_world(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();
        
        // Pseudo-code:
        // 1. Get all state transitions from procedural cache (FSM)
        // 2. For each transition (s, a) -> s', check world-model prediction
        // 3. If P(s'|s,a) < threshold but FSM allows it, report conflict
        
        // Example detection (placeholder):
        // let fsm_transitions = procedural_cache.get_allowed_transitions()?;
        // for (state, action, next_state) in fsm_transitions {
        //     let prediction = world_model.predict_next_state(&state, &action)?;
        //     let prob = prediction.probabilities.get(&next_state).unwrap_or(&0.0);
        //     
        //     if *prob < self.probability_threshold {
        //         inconsistencies.push(InconsistencyReport::new(
        //             InconsistencyType::ProceduralWorldConflict,
        //             vec![],
        //             format!(
        //                 "FSM allows {}--{}-->{} but world-model predicts P={:.4}",
        //                 state, action, next_state, prob
        //             ),
        //             7,
        //         ).with_metadata("state".to_string(), state.into())
        //          .with_metadata("action".to_string(), action.into())
        //          .with_metadata("next_state".to_string(), next_state.into())
        //          .with_metadata("probability".to_string(), prob.into()));
        //     }
        // }
        
        Ok(inconsistencies)
    }

    /// Check for causal violations
    ///
    /// Detects: Event sequence violates causal constraints in causal graph
    fn check_causal_violations(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();
        
        // Pseudo-code:
        // 1. Get recent event sequence from temporal indexer
        // 2. Extract causal dependencies from causal graph
        // 3. Check if event order violates causal constraints
        //    (e.g., effect observed before cause)
        
        // Example detection (placeholder):
        // let events = temporal_indexer.get_event_sequence()?;
        // let causal_graph = world_model.get_causal_graph()?;
        // 
        // for i in 0..events.len() {
        //     for j in i+1..events.len() {
        //         let e1 = &events[i];
        //         let e2 = &events[j];
        //         
        //         // If e2 causally precedes e1 but e1 happened first
        //         if causal_graph.has_edge(&e2.type, &e1.type) {
        //             inconsistencies.push(InconsistencyReport::new(
        //                 InconsistencyType::CausalViolation,
        //                 vec![e1.id.clone(), e2.id.clone()],
        //                 format!(
        //                     "Event {} (t={}) causally preceded by {} (t={}), violating temporal order",
        //                     e1.type, e1.timestamp, e2.type, e2.timestamp
        //                 ),
        //                 8,
        //             ));
        //         }
        //     }
        // }
        
        Ok(inconsistencies)
    }

    /// Check for entity permanence violations
    ///
    /// Detects: Entity exists in world-model but deleted from symbolic store
    fn check_entity_permanence(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();
        
        // Pseudo-code:
        // 1. Get all tracked entities from world-model
        // 2. For each entity, check if it exists in symbolic store
        // 3. If entity missing from symbolic but still tracked, report violation
        
        // Example detection (placeholder):
        // let tracked_entities = world_model.get_tracked_entities()?;
        // 
        // for entity_id in tracked_entities {
        //     if !symbolic_store.contains(&entity_id) {
        //         inconsistencies.push(InconsistencyReport::new(
        //             InconsistencyType::EntityPermanenceViolation,
        //             vec![entity_id.clone()],
        //             format!(
        //                 "Entity {} tracked in world-model but deleted from symbolic store",
        //                 entity_id
        //             ),
        //             8,
        //         ));
        //     }
        // }
        
        Ok(inconsistencies)
    }

    /// Check for graph inconsistencies
    ///
    /// Detects: Symbolic DAG contradicts world-model causal graph (edit distance > threshold)
    fn check_graph_consistency(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();
        
        // Pseudo-code:
        // 1. Get symbolic DAG from symbolic store
        // 2. Get causal graph from world-model
        // 3. Compute graph edit distance between them
        // 4. If distance > threshold, report inconsistency
        
        // Example detection (placeholder):
        // let symbolic_edges = symbolic_store.get_graph_edges()?;
        // let causal_edges = world_model.get_causal_edges()?;
        // 
        // let edit_distance = self.compute_graph_edit_distance(&symbolic_edges, &causal_edges);
        // 
        // if edit_distance > self.graph_edit_distance_threshold {
        //     inconsistencies.push(InconsistencyReport::new(
        //         InconsistencyType::GraphInconsistency,
        //         vec![],
        //         format!(
        //             "Symbolic DAG and causal graph have edit distance {} (threshold={})",
        //             edit_distance, self.graph_edit_distance_threshold
        //         ),
        //         6,
        //     ).with_metadata("edit_distance".to_string(), edit_distance.into())
        //      .with_metadata("threshold".to_string(), self.graph_edit_distance_threshold.into()));
        // }
        
        Ok(inconsistencies)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Get entity counts across modules (with caching)
    fn get_entity_counts(&mut self, entity_id: &str) -> Result<EntityCounts, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Check cache (5 second TTL)
        if let Some(cached) = self.entity_cache.get(entity_id) {
            if now - cached.last_updated < 5 {
                return Ok(cached.clone());
            }
        }
        
        // Compute counts (in real implementation, query actual modules)
        let counts = EntityCounts {
            temporal_count: 0,  // temporal_indexer.count_references(entity_id)
            symbolic_count: 0,  // symbolic_store.count_references(entity_id)
            world_model_count: 0,  // world_model.count_references(entity_id)
            last_updated: now,
        };
        
        self.entity_cache.insert(entity_id.to_string(), counts.clone());
        Ok(counts)
    }

    /// Compute graph edit distance between two sets of edges
    ///
    /// Edit distance = min number of edge insertions/deletions to transform one graph into another
    fn compute_graph_edit_distance(
        &self,
        edges1: &[(String, String)],
        edges2: &[(String, String)],
    ) -> usize {
        let set1: HashSet<_> = edges1.iter().collect();
        let set2: HashSet<_> = edges2.iter().collect();
        
        // Symmetric difference gives edges that need to be added/removed
        let diff1: Vec<_> = set1.difference(&set2).collect();
        let diff2: Vec<_> = set2.difference(&set1).collect();
        
        diff1.len() + diff2.len()
    }
}

impl Default for ConsistencyChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistency_checker_creation() {
        let checker = ConsistencyChecker::new();
        assert_eq!(checker.graph_edit_distance_threshold, 5);
        assert_eq!(checker.probability_threshold, 0.01);
    }

    #[test]
    fn test_custom_thresholds() {
        let checker = ConsistencyChecker::with_thresholds(10, 0.05);
        assert_eq!(checker.graph_edit_distance_threshold, 10);
        assert_eq!(checker.probability_threshold, 0.05);
    }

    #[test]
    fn test_inconsistency_report_creation() {
        let report = InconsistencyReport::new(
            InconsistencyType::TemporalSymbolicMismatch,
            vec!["entity1".to_string()],
            "Test inconsistency".to_string(),
            5,
        );
        
        assert_eq!(report.inconsistency_type, InconsistencyType::TemporalSymbolicMismatch);
        assert_eq!(report.affected_entities, vec!["entity1".to_string()]);
        assert_eq!(report.severity, 5);
    }

    #[test]
    fn test_inconsistency_report_with_metadata() {
        let report = InconsistencyReport::new(
            InconsistencyType::ProceduralWorldConflict,
            vec![],
            "Test".to_string(),
            7,
        )
        .with_metadata("state".to_string(), "S1".into())
        .with_metadata("probability".to_string(), 0.001.into());
        
        assert_eq!(report.metadata.len(), 2);
        assert!(report.metadata.contains_key("state"));
    }

    #[test]
    fn test_graph_edit_distance_identical() {
        let checker = ConsistencyChecker::new();
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        
        let distance = checker.compute_graph_edit_distance(&edges, &edges);
        assert_eq!(distance, 0);
    }

    #[test]
    fn test_graph_edit_distance_completely_different() {
        let checker = ConsistencyChecker::new();
        let edges1 = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        let edges2 = vec![
            ("X".to_string(), "Y".to_string()),
            ("Y".to_string(), "Z".to_string()),
        ];
        
        let distance = checker.compute_graph_edit_distance(&edges1, &edges2);
        assert_eq!(distance, 4); // Remove 2 edges, add 2 edges
    }

    #[test]
    fn test_graph_edit_distance_partial_overlap() {
        let checker = ConsistencyChecker::new();
        let edges1 = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        let edges2 = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "D".to_string()),
        ];
        
        let distance = checker.compute_graph_edit_distance(&edges1, &edges2);
        assert_eq!(distance, 2); // Remove B->C, add B->D
    }

    #[test]
    fn test_check_all_empty() {
        let mut checker = ConsistencyChecker::new();
        let inconsistencies = checker.check_all().unwrap();
        assert_eq!(inconsistencies.len(), 0);
    }

    #[test]
    fn test_check_entity() {
        let mut checker = ConsistencyChecker::new();
        let inconsistencies = checker.check_entity("test_entity").unwrap();
        // Should detect temporal-symbolic mismatch (both counts are 0, so equal)
        assert_eq!(inconsistencies.len(), 0);
    }

    #[test]
    fn test_entity_counts_caching() {
        let mut checker = ConsistencyChecker::new();
        
        // First call populates cache
        let counts1 = checker.get_entity_counts("test").unwrap();
        
        // Second call should hit cache
        let counts2 = checker.get_entity_counts("test").unwrap();
        
        assert_eq!(counts1.last_updated, counts2.last_updated);
    }

    #[test]
    fn test_inconsistency_type_serialization() {
        let report = InconsistencyReport::new(
            InconsistencyType::CausalViolation,
            vec!["e1".to_string(), "e2".to_string()],
            "Test violation".to_string(),
            9,
        );
        
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("CausalViolation"));
    }

    #[test]
    fn test_temporal_symbolic_check() {
        let mut checker = ConsistencyChecker::new();
        let inconsistencies = checker.check_temporal_symbolic().unwrap();
        assert_eq!(inconsistencies.len(), 0); // No real modules connected yet
    }

    #[test]
    fn test_procedural_world_check() {
        let mut checker = ConsistencyChecker::new();
        let inconsistencies = checker.check_procedural_world().unwrap();
        assert_eq!(inconsistencies.len(), 0);
    }

    #[test]
    fn test_causal_violations_check() {
        let mut checker = ConsistencyChecker::new();
        let inconsistencies = checker.check_causal_violations().unwrap();
        assert_eq!(inconsistencies.len(), 0);
    }

    #[test]
    fn test_entity_permanence_check() {
        let mut checker = ConsistencyChecker::new();
        let inconsistencies = checker.check_entity_permanence().unwrap();
        assert_eq!(inconsistencies.len(), 0);
    }

    #[test]
    fn test_graph_consistency_check() {
        let mut checker = ConsistencyChecker::new();
        let inconsistencies = checker.check_graph_consistency().unwrap();
        assert_eq!(inconsistencies.len(), 0);
    }
}
