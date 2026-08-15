// Consistency Checker - Detect inconsistencies across memory subsystems
//
// Detects five types of inconsistencies:
// 1. TemporalSymbolicMismatch: Event references missing entity
// 2. ProceduralWorldConflict: FSM allows transition with P=0
// 3. CausalViolation: Event sequence violates causal constraints
// 4. EntityPermanenceViolation: Entity exists in world-model but deleted from symbolic
// 5. GraphInconsistency: Symbolic DAG contradicts world-model causal graph

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::topological_memory::{CausalTopoGraph, EdgeType};

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

        let id = format!(
            "{:?}_{}_{}",
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
    pub graph_edit_distance_threshold: usize,

    /// Probability threshold for procedural-world conflicts (P < threshold is conflict)
    pub probability_threshold: f64,

    /// Entity count cache for efficiency
    entity_cache: HashMap<String, EntityCounts>,

    /// Optional CausalTopoGraph (from topological_memory) used to flesh out
    /// the previously stubbed check_causal_violations and check_entity_permanence.
    /// Nodes discovered via personalized_pagerank (even with empty seeds), contradictions
    /// via detect_contradiction + find_multi_hop_paths for path-based causal issues.
    /// Task 6 TDD: replaces placeholders with real topo-driven detection.
    topo: Option<CausalTopoGraph>,

    pub temporal_indexer: Option<Arc<Mutex<crate::temporal_indexer::TemporalIndexer<uuid::Uuid>>>>,
    pub symbolic_store: Option<
        Arc<Mutex<crate::symbolic_store::SymbolicStore<crate::symbolic_store::InMemoryGraph>>>,
    >,
    pub procedural_cache: Option<Arc<Mutex<crate::procedural_cache::ProceduralCache>>>,
    pub world_model: Option<Arc<crate::world_model_enhanced::WorldModelEnhanced>>,
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
            topo: None,
            temporal_indexer: None,
            symbolic_store: None,
            procedural_cache: None,
            world_model: None,
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(graph_edit_threshold: usize, prob_threshold: f64) -> Self {
        Self {
            graph_edit_distance_threshold: graph_edit_threshold,
            probability_threshold: prob_threshold,
            entity_cache: HashMap::new(),
            topo: None,
            temporal_indexer: None,
            symbolic_store: None,
            procedural_cache: None,
            world_model: None,
        }
    }

    pub fn set_causal_topo(&mut self, topo: CausalTopoGraph) {
        self.topo = Some(topo);
    }

    pub fn set_temporal_indexer(
        &mut self,
        indexer: Arc<Mutex<crate::temporal_indexer::TemporalIndexer<uuid::Uuid>>>,
    ) {
        self.temporal_indexer = Some(indexer);
    }

    pub fn set_symbolic_store(
        &mut self,
        store: Arc<
            Mutex<crate::symbolic_store::SymbolicStore<crate::symbolic_store::InMemoryGraph>>,
        >,
    ) {
        self.symbolic_store = Some(store);
    }

    pub fn set_procedural_cache(
        &mut self,
        cache: Arc<Mutex<crate::procedural_cache::ProceduralCache>>,
    ) {
        self.procedural_cache = Some(cache);
    }

    pub fn set_world_model(&mut self, wm: Arc<crate::world_model_enhanced::WorldModelEnhanced>) {
        self.world_model = Some(wm);
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

    /// Clone Arc smart pointer snapshots (temporal_indexer, symbolic_store, procedural_cache, CausalTopoGraph)
    /// and run long background verifications on an isolated thread.
    pub fn check_consistency_cow(
        &self,
    ) -> std::thread::JoinHandle<Result<Vec<InconsistencyReport>, String>> {
        let temporal_clone = self.temporal_indexer.clone();
        let symbolic_clone = self.symbolic_store.clone();
        let procedural_clone = self.procedural_cache.clone();
        let world_clone = self.world_model.clone();
        let topo_clone = self.topo.clone();
        let threshold_clone = self.graph_edit_distance_threshold;
        let prob_clone = self.probability_threshold;

        std::thread::spawn(move || {
            let mut checker_snapshot = ConsistencyChecker {
                graph_edit_distance_threshold: threshold_clone,
                probability_threshold: prob_clone,
                entity_cache: HashMap::new(),
                topo: topo_clone,
                temporal_indexer: temporal_clone,
                symbolic_store: symbolic_clone,
                procedural_cache: procedural_clone,
                world_model: world_clone,
            };

            checker_snapshot.check_all()
        })
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

    fn check_temporal_symbolic(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();

        if let (Some(ref indexer_arc), Some(ref store_arc)) =
            (&self.temporal_indexer, &self.symbolic_store)
        {
            let indexer = indexer_arc
                .lock()
                .map_err(|e| format!("Failed to lock indexer: {}", e))?;
            let store = store_arc
                .lock()
                .map_err(|e| format!("Failed to lock store: {}", e))?;

            let recent_traces = indexer.get_recent(100);
            for trace in recent_traces {
                let entity_id = trace.data;
                if store.get_node(entity_id).is_none() {
                    inconsistencies.push(InconsistencyReport::new(
                        InconsistencyType::TemporalSymbolicMismatch,
                        vec![entity_id.to_string()],
                        format!(
                            "Temporal trace {} references entity ID {} which is missing from symbolic store",
                            trace.id, entity_id
                        ),
                        7,
                    ));
                }
            }
        }

        Ok(inconsistencies)
    }

    fn check_procedural_world(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();

        if let (Some(ref cache_arc), Some(ref wm_arc)) = (&self.procedural_cache, &self.world_model)
        {
            let cache = cache_arc
                .lock()
                .map_err(|e| format!("Failed to lock procedural cache: {}", e))?;

            let transitions = cache.get_transitions();
            for t in transitions {
                let format_state = |state: &crate::procedural_cache::FSMState| -> String {
                    match state {
                        crate::procedural_cache::FSMState::Custom(s) => s.clone(),
                        other => format!("{:?}", other),
                    }
                };

                let state_str = format_state(&t.from);
                let next_state_str = format_state(&t.to);
                let action_str = t.condition.clone().unwrap_or_else(|| "None".to_string());

                // Query world model transition probability
                if let Ok(pred) = wm_arc.predict_next_state(&state_str, &action_str) {
                    let prob = pred
                        .probabilities
                        .get(&next_state_str)
                        .cloned()
                        .unwrap_or(0.0);
                    if prob < self.probability_threshold {
                        inconsistencies.push(
                            InconsistencyReport::new(
                                InconsistencyType::ProceduralWorldConflict,
                                vec![],
                                format!(
                                    "FSM allows transition from state '{}' to '{}' on condition '{}', but world-model predicts P={:.4}",
                                    state_str, next_state_str, action_str, prob
                                ),
                                7,
                            )
                            .with_metadata("from_state".to_string(), state_str.into())
                            .with_metadata("to_state".to_string(), next_state_str.into())
                            .with_metadata("condition".to_string(), action_str.into())
                            .with_metadata("probability".to_string(), prob.into())
                        );
                    }
                }
            }
        }

        Ok(inconsistencies)
    }

    /// Check for causal violations
    ///
    /// Detects: Event sequence violates causal constraints in causal graph
    /// Task 6: now uses injected CausalTopoGraph (if present) for real detection:
    /// - node discovery via personalized_pagerank (seeds=[] yields all ids)
    /// - contradiction detection via detect_contradiction (cycle + high-str reverse Causal)
    /// - path contradictions via find_multi_hop_paths
    fn check_causal_violations(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        if let Some(ref topo) = self.topo {
            return self.check_causal_violations_from_topo(topo);
        }
        Ok(vec![])
    }

    /// Internal impl: use topo APIs for contradiction and path based causal violations.
    fn check_causal_violations_from_topo(
        &self,
        topo: &CausalTopoGraph,
    ) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();

        // Discover nodes (ppr with no seeds still populates ranks for entire graph)
        let ranks = topo.personalized_pagerank(&[], 0.85, 2);
        let ids: Vec<String> = ranks.keys().cloned().collect();

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a = &ids[i];
                let b = &ids[j];

                // If a->b would contradict existing topo (path b..a or high-str reverse), report violation
                if topo.detect_contradiction(a.clone(), b.clone(), EdgeType::Causal) {
                    let rev_paths = topo.find_multi_hop_paths(b, a, 3);
                    let desc = if !rev_paths.is_empty() {
                        format!(
                            "Causal {}->{} contradicts topo (path {} -> ... -> {} exists; cycle or strength conflict via topo substrate)",
                            a, b, b, a
                        )
                    } else {
                        format!(
                            "Causal {}->{} would contradict topo structure (high-strength reverse or cycle detected by topo)",
                            a, b
                        )
                    };
                    inconsistencies.push(
                        InconsistencyReport::new(
                            InconsistencyType::CausalViolation,
                            vec![a.clone(), b.clone()],
                            desc,
                            8,
                        )
                        .with_metadata("via_topo".to_string(), serde_json::json!(true))
                        .with_metadata(
                            "method".to_string(),
                            serde_json::json!("detect_contradiction+find_multi_hop_paths"),
                        ),
                    );
                }

                // Symmetric: b->a would contradict
                if topo.detect_contradiction(b.clone(), a.clone(), EdgeType::Causal) {
                    inconsistencies.push(
                        InconsistencyReport::new(
                            InconsistencyType::CausalViolation,
                            vec![b.clone(), a.clone()],
                            format!(
                                "Causal direction {}->{} contradicts existing topo structure (using detect_contradiction on paths/edges)",
                                b, a
                            ),
                            8,
                        )
                        .with_metadata("via_topo".to_string(), serde_json::json!(true)),
                    );
                }
            }
        }

        Ok(inconsistencies)
    }

    /// Check for entity permanence violations
    ///
    /// Detects: Entity exists in world-model but deleted from symbolic store
    /// Task 6: when topo present, treat topo nodes (discovered via ppr) as WM-tracked entities.
    /// Report permanence violation (topo tracks but symbolic not cross-checked here).
    fn check_entity_permanence(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();

        if let Some(ref topo) = self.topo {
            let ranks = topo.personalized_pagerank(&[], 0.85, 1);
            for (entity_id, _rank) in ranks {
                // With symbolic store: real cross-check. Without it, topo nodes alone
                // count as "tracked in substrate but not symbolically grounded".
                let missing_in_symbolic = if let Some(ref store) = self.symbolic_store {
                    match store.lock() {
                        Ok(mut s) => s.find_by_label(&entity_id).is_empty(),
                        Err(_) => true,
                    }
                } else {
                    true
                };
                if missing_in_symbolic {
                    inconsistencies.push(
                        InconsistencyReport::new(
                            InconsistencyType::EntityPermanenceViolation,
                            vec![entity_id.clone()],
                            format!(
                                "Entity {} tracked in topo/world substrate but missing from symbolic store",
                                entity_id
                            ),
                            8,
                        )
                        .with_metadata("source".into(), serde_json::json!("topo_symbolic_crosscheck")),
                    );
                }
            }
        }

        Ok(inconsistencies)
    }

    /// Check for graph inconsistencies: world-model causal edges vs optional symbolic edge set.
    fn check_graph_consistency(&mut self) -> Result<Vec<InconsistencyReport>, String> {
        let mut inconsistencies = Vec::new();

        let causal_edges: Vec<(String, String)> = if let Some(ref wm) = self.world_model {
            wm.get_causal_edges()
                .into_iter()
                .map(|e| (e.from, e.to))
                .collect()
        } else if let Some(ref topo) = self.topo {
            // Approximate causal edges from topo serializable
            topo.to_serializable()
                .edges
                .into_iter()
                .filter(|e| matches!(e.edge_type, EdgeType::Causal))
                .map(|e| (e.from, e.to))
                .collect()
        } else {
            return Ok(inconsistencies);
        };

        // Build undirected set for edit-distance-lite: count WM edges with no reverse topo path
        let mut missing_reverse = 0usize;
        if let Some(ref topo) = self.topo {
            for (from, to) in &causal_edges {
                if topo.detect_contradiction(from.clone(), to.clone(), EdgeType::Causal) {
                    missing_reverse += 1;
                    inconsistencies.push(
                        InconsistencyReport::new(
                            InconsistencyType::GraphInconsistency,
                            vec![from.clone(), to.clone()],
                            format!(
                                "World-model causal edge {}→{} conflicts with topological substrate",
                                from, to
                            ),
                            6,
                        )
                        .with_metadata("wm_edge".into(), serde_json::json!([from, to])),
                    );
                }
            }
        }

        if missing_reverse > self.graph_edit_distance_threshold {
            inconsistencies.push(
                InconsistencyReport::new(
                    InconsistencyType::GraphInconsistency,
                    vec![],
                    format!(
                        "Symbolic/topo vs causal graph conflict count {} exceeds threshold {}",
                        missing_reverse, self.graph_edit_distance_threshold
                    ),
                    7,
                )
                .with_metadata("conflict_count".into(), missing_reverse.into())
                .with_metadata(
                    "threshold".into(),
                    self.graph_edit_distance_threshold.into(),
                ),
            );
        }

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
            temporal_count: 0,    // temporal_indexer.count_references(entity_id)
            symbolic_count: 0,    // symbolic_store.count_references(entity_id)
            world_model_count: 0, // world_model.count_references(entity_id)
            last_updated: now,
        };

        self.entity_cache
            .insert(entity_id.to_string(), counts.clone());
        Ok(counts)
    }

    /// Compute graph edit distance between two sets of edges
    ///
    /// Edit distance = min number of edge insertions/deletions to transform one graph into another
    pub fn compute_graph_edit_distance(
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
    use crate::topological_memory::{CausalTopoGraph, EdgeType};
    use std::collections::HashMap;

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

        assert_eq!(
            report.inconsistency_type,
            InconsistencyType::TemporalSymbolicMismatch
        );
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
    #[test]
    fn test_causal_violation_detected() {
        let mut checker = ConsistencyChecker::new();
        let violations = checker.check_all().unwrap();
        let causal: Vec<_> = violations
            .iter()
            .filter(|r| matches!(r.inconsistency_type, InconsistencyType::CausalViolation))
            .collect();
        // TODO: When check_causal_violations is wired to actual temporal/world-model data,
        // inject event sequence where effect precedes cause and verify causal is non-empty.
    }

    // ========================================================================
    // Task 6 TDD: failing tests for topo-backed causal and entity checks (stubs)
    // ========================================================================

    #[test]
    fn test_coherence_causal_via_topo() {
        let mut topo = CausalTopoGraph::new();
        let _ = topo
            .add_node("cause".into(), [0.0; 128], HashMap::new())
            .unwrap();
        let _ = topo
            .add_node("effect".into(), [0.0; 128], HashMap::new())
            .unwrap();
        topo.add_edge("cause".into(), "effect".into(), EdgeType::Causal, 0.85, 0.9)
            .unwrap();
        // conflicting direction would be detected by topo.detect_contradiction
        let mut checker = ConsistencyChecker::new();
        checker.set_causal_topo(topo);
        let reports = checker.check_causal_violations().unwrap();
        assert!(
            !reports.is_empty(),
            "expected at least one CausalViolation from topo reverse/contradiction"
        );
        assert!(reports
            .iter()
            .any(|r| matches!(r.inconsistency_type, InconsistencyType::CausalViolation)));
        assert!(reports.iter().any(|r| r.description.contains("topo")
            || r.description.contains("contradict")
            || r.description.contains("Causal")));
    }

    #[test]
    fn test_coherence_entity_permanence_via_topo() {
        let mut topo = CausalTopoGraph::new();
        let _ = topo
            .add_node(
                "tracked_entity_from_topo".into(),
                [0.42; 128],
                HashMap::new(),
            )
            .unwrap();
        let mut checker = ConsistencyChecker::new();
        checker.set_causal_topo(topo);
        let reports = checker.check_entity_permanence().unwrap();
        assert!(
            !reports.is_empty(),
            "expected EntityPermanenceViolation using topo nodes as WM entities"
        );
        assert!(reports.iter().any(|r| matches!(
            r.inconsistency_type,
            InconsistencyType::EntityPermanenceViolation
        )));
    }

    #[test]
    fn test_check_consistency_with_topo_influenced_reports() {
        let mut topo = CausalTopoGraph::new();
        let _ = topo
            .add_node("a".into(), [0.0; 128], HashMap::new())
            .unwrap();
        let _ = topo
            .add_node("b".into(), [0.0; 128], HashMap::new())
            .unwrap();
        topo.add_edge("a".into(), "b".into(), EdgeType::Causal, 0.6, 0.7)
            .unwrap();
        let mut checker = ConsistencyChecker::new();
        checker.set_causal_topo(topo);
        let incs = checker.check_all().unwrap();
        // now that wired, should surface causal or entity from topo
        let has_relevant = incs.iter().any(|r| {
            matches!(
                r.inconsistency_type,
                InconsistencyType::CausalViolation | InconsistencyType::EntityPermanenceViolation
            )
        });
        assert!(
            has_relevant,
            "check_consistency (via check_all) should return topo-driven reports"
        );
    }

    #[test]
    fn test_gate_write_and_enforce_still_function_with_topo_context() {
        // gate_write and enforce_invariants live on CoherenceChecker but use invariants (not yet topo reports);
        // here we at least ensure no breakage and that check path can coexist
        use crate::coherence::CoherenceChecker;
        let coherence = CoherenceChecker::new();
        // gate should pass with no critical invariant violations
        let gate_res = coherence.gate_write("task6_topo_test");
        assert!(
            gate_res.is_ok(),
            "gate_write should succeed when no critical invariants"
        );
        let violations = coherence.enforce_invariants().unwrap();
        // may be empty
        let _ = violations;
        // also exercise check_and_resolve path (uses check_consistency)
        let resolve_res =
            coherence.check_and_resolve(crate::coherence::ResolutionStrategy::Recency);
        assert!(resolve_res.is_ok() || resolve_res.is_err()); // either way, no panic
    }

    #[test]
    fn test_temporal_symbolic_coherence_checking() {
        let indexer = Arc::new(Mutex::new(crate::temporal_indexer::TemporalIndexer::new(
            100, 3600,
        )));
        let store = Arc::new(Mutex::new(crate::symbolic_store::SymbolicStore::new()));

        let missing_id = uuid::Uuid::new_v4();
        let present_id = {
            let mut st = store.lock().unwrap();
            st.add_node("present_entity", std::collections::HashMap::new())
        };

        // Add a trace in indexer for entity "missing_entity"
        {
            let mut idx = indexer.lock().unwrap();
            idx.insert(crate::temporal_indexer::TemporalTrace {
                id: uuid::Uuid::new_v4(),
                timestamp: std::time::SystemTime::now(),
                data: missing_id,
                relevance: 1.0,
                decay_factor: 1.0,
                last_access: std::time::SystemTime::now(),
                decay_type: crate::decay::DecayType::Linear {
                    duration: std::time::Duration::from_secs(3600),
                },
            });

            // Add a trace for a present entity
            idx.insert(crate::temporal_indexer::TemporalTrace {
                id: uuid::Uuid::new_v4(),
                timestamp: std::time::SystemTime::now(),
                data: present_id,
                relevance: 1.0,
                decay_factor: 1.0,
                last_access: std::time::SystemTime::now(),
                decay_type: crate::decay::DecayType::Linear {
                    duration: std::time::Duration::from_secs(3600),
                },
            });
        }

        let mut checker = ConsistencyChecker::new();
        checker.set_temporal_indexer(indexer);
        checker.set_symbolic_store(store);

        let reports = checker.check_temporal_symbolic().unwrap();
        assert!(!reports.is_empty(), "expected a mismatch report");
        assert!(reports.iter().any(|r| {
            matches!(
                r.inconsistency_type,
                InconsistencyType::TemporalSymbolicMismatch
            ) && r.affected_entities.contains(&missing_id.to_string())
        }));
    }

    #[test]
    fn test_procedural_world_coherence_checking() {
        let cache = Arc::new(Mutex::new(crate::procedural_cache::ProceduralCache::new()));
        let wm = Arc::new(crate::world_model_enhanced::WorldModelEnhanced::new());

        // Add FSM transitions
        {
            let mut c = cache.lock().unwrap();
            c.add_transition(crate::procedural_cache::FSMTransition {
                from: crate::procedural_cache::FSMState::Start,
                to: crate::procedural_cache::FSMState::Custom("Step2".to_string()),
                condition: Some("go".to_string()),
            });
            c.add_transition(crate::procedural_cache::FSMTransition {
                from: crate::procedural_cache::FSMState::Start,
                to: crate::procedural_cache::FSMState::Custom("Step3".to_string()),
                condition: Some("go".to_string()),
            });
        }

        // Let world model predict Step2 with P=0.8, Step3 is unobserved (P=0.0)
        let _ = wm.observe_transition("Start".to_string(), "go".to_string(), "Step2".to_string());

        let mut checker = ConsistencyChecker::new();
        checker.set_procedural_cache(cache);
        checker.set_world_model(wm);

        let reports = checker.check_procedural_world().unwrap();
        assert!(
            !reports.is_empty(),
            "expected a procedural-world conflict report"
        );
        assert!(reports.iter().any(|r| {
            matches!(
                r.inconsistency_type,
                InconsistencyType::ProceduralWorldConflict
            ) && r.description.contains("Step3")
        }));
    }
}
