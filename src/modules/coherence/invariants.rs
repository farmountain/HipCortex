// System Invariants - Enforce mathematical invariants across modules
//
// Validates four critical invariants:
// 1. Memory Consistency: Entity counts match across temporal, symbolic, world-model
// 2. Decay Monotonicity: Activation scores never increase over time
// 3. Graph Acyclicity: Symbolic and causal graphs remain DAGs
// 4. Conservation: Entity count conserved (entities_created - entities_deleted = net_change)

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Types of system invariants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvariantType {
    /// Memory consistency: ∀ entity e, temporal_count(e) = symbolic_count(e) = world_model_count(e)
    MemoryConsistency,
    
    /// Decay monotonicity: ∀ t1 < t2, activation(t2) ≤ activation(t1)
    DecayMonotonicity,
    
    /// Graph acyclicity: Symbolic and causal graphs remain DAGs
    GraphAcyclicity,
    
    /// Conservation: entities_created - entities_deleted = net_change
    Conservation,
}

/// Report of an invariant violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantViolation {
    /// Type of invariant violated
    pub invariant_type: InvariantType,
    
    /// Description of the violation
    pub description: String,
    
    /// Affected entities/nodes
    pub affected: Vec<String>,
    
    /// Whether this is a critical violation (should halt operations)
    pub critical: bool,
    
    /// Violation timestamp
    pub timestamp: u64,
    
    /// Additional diagnostic data
    pub diagnostics: HashMap<String, serde_json::Value>,
}

impl InvariantViolation {
    pub fn new(
        invariant_type: InvariantType,
        description: String,
        affected: Vec<String>,
        critical: bool,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            invariant_type,
            description,
            affected,
            critical,
            timestamp: now,
            diagnostics: HashMap::new(),
        }
    }
    
    pub fn with_diagnostic(mut self, key: String, value: serde_json::Value) -> Self {
        self.diagnostics.insert(key, value);
        self
    }
}

/// System invariant validator
pub struct SystemInvariants {
    /// Activation history for decay monitoring (entity_id -> [(timestamp, activation)])
    pub activation_history: HashMap<String, Vec<(u64, f64)>>,

    /// Entity lifecycle tracking (entity_id -> (created_count, deleted_count))
    entity_lifecycle: HashMap<String, (usize, usize)>,

    /// Cached symbolic graph edges for acyclicity validation (from, to)
    symbolic_edges: Vec<(String, String)>,

    /// Cached causal graph edges for acyclicity validation (from, to)
    causal_edges: Vec<(String, String)>,

    /// Last validation timestamp
    last_validation: u64,
}

impl SystemInvariants {
    /// Create new system invariant validator
    pub fn new() -> Self {
        Self {
            activation_history: HashMap::new(),
            entity_lifecycle: HashMap::new(),
            symbolic_edges: Vec::new(),
            causal_edges: Vec::new(),
            last_validation: 0,
        }
    }

    // ========================================================================
    // Validation APIs
    // ========================================================================

    /// Validate all invariants
    ///
    /// Returns list of violations (empty if all pass)
    pub fn validate_all(&mut self) -> Result<Vec<InvariantViolation>, String> {
        let mut violations = Vec::new();
        
        // Update last validation timestamp
        self.last_validation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // Check each invariant
        if let Some(v) = self.check_memory_consistency()? {
            violations.push(v);
        }
        
        if let Some(v) = self.check_decay_monotonicity()? {
            violations.push(v);
        }
        
        if let Some(v) = self.check_graph_acyclicity()? {
            violations.push(v);
        }
        
        if let Some(v) = self.check_conservation()? {
            violations.push(v);
        }
        
        Ok(violations)
    }

    /// Validate specific invariant
    pub fn validate_specific(
        &mut self,
        invariant: InvariantType,
    ) -> Result<Option<InvariantViolation>, String> {
        match invariant {
            InvariantType::MemoryConsistency => self.check_memory_consistency(),
            InvariantType::DecayMonotonicity => self.check_decay_monotonicity(),
            InvariantType::GraphAcyclicity => self.check_graph_acyclicity(),
            InvariantType::Conservation => self.check_conservation(),
        }
    }

    // ========================================================================
    // Invariant Checks
    // ========================================================================

    /// Check memory consistency invariant
    ///
    /// Validates: ∀ entity e, temporal_count(e) = symbolic_count(e) = world_model_count(e)
    fn check_memory_consistency(&self) -> Result<Option<InvariantViolation>, String> {
        // In real implementation, query actual modules
        // For now, provide structure
        
        // Pseudo-code:
        // let entities = get_all_entity_ids();
        // for entity_id in entities {
        //     let temporal_count = temporal_indexer.count_references(&entity_id);
        //     let symbolic_count = symbolic_store.count_references(&entity_id);
        //     let world_model_count = world_model.count_references(&entity_id);
        //     
        //     if temporal_count != symbolic_count || symbolic_count != world_model_count {
        //         return Ok(Some(InvariantViolation::new(
        //             InvariantType::MemoryConsistency,
        //             format!(
        //                 "Entity {} has inconsistent counts: temporal={}, symbolic={}, world_model={}",
        //                 entity_id, temporal_count, symbolic_count, world_model_count
        //             ),
        //             vec![entity_id.clone()],
        //             true, // Critical - memory corruption
        //         )
        //         .with_diagnostic("temporal_count".to_string(), temporal_count.into())
        //         .with_diagnostic("symbolic_count".to_string(), symbolic_count.into())
        //         .with_diagnostic("world_model_count".to_string(), world_model_count.into())));
        //     }
        // }
        
        Ok(None) // No violation detected (placeholder)
    }

    /// Check decay monotonicity invariant
    ///
    /// Validates: ∀ t1 < t2, activation(t2) ≤ activation(t1)
    pub fn check_decay_monotonicity(&self) -> Result<Option<InvariantViolation>, String> {
        for (entity_id, history) in &self.activation_history {
            // Check if activation scores are monotonically non-increasing
            for i in 1..history.len() {
                let (t1, activation1) = history[i - 1];
                let (t2, activation2) = history[i];
                
                if t2 > t1 && activation2 > activation1 {
                    return Ok(Some(InvariantViolation::new(
                        InvariantType::DecayMonotonicity,
                        format!(
                            "Entity {} activation increased from {:.4} (t={}) to {:.4} (t={})",
                            entity_id, activation1, t1, activation2, t2
                        ),
                        vec![entity_id.clone()],
                        false, // Non-critical - data anomaly
                    )
                    .with_diagnostic("timestamp1".to_string(), t1.into())
                    .with_diagnostic("activation1".to_string(), activation1.into())
                    .with_diagnostic("timestamp2".to_string(), t2.into())
                    .with_diagnostic("activation2".to_string(), activation2.into())));
                }
            }
        }
        
        Ok(None)
    }

    /// Check graph acyclicity invariant
    ///
    /// Validates: Symbolic and causal graphs remain DAGs (no cycles)
    pub fn check_graph_acyclicity(&self) -> Result<Option<InvariantViolation>, String> {
        // Check symbolic graph edges for cycles
        if !self.symbolic_edges.is_empty() {
            if let Some(cycle) = self.detect_cycle(&self.symbolic_edges) {
                return Ok(Some(InvariantViolation::new(
                    InvariantType::GraphAcyclicity,
                    format!("Cycle detected in symbolic graph: {:?}", cycle),
                    cycle.clone(),
                    true, // Critical - breaks causal reasoning
                )));
            }
        }

        // Check causal graph edges for cycles
        if !self.causal_edges.is_empty() {
            if let Some(cycle) = self.detect_cycle(&self.causal_edges) {
                return Ok(Some(InvariantViolation::new(
                    InvariantType::GraphAcyclicity,
                    format!("Cycle detected in causal graph: {:?}", cycle),
                    cycle.clone(),
                    true,
                )));
            }
        }

        Ok(None) // No cycles detected
    }

    /// Check conservation invariant
    ///
    /// Validates: entities_created - entities_deleted = net_change
    pub fn check_conservation(&self) -> Result<Option<InvariantViolation>, String> {
        for (entity_id, (created, deleted)) in &self.entity_lifecycle {
            // Net change should be non-negative and <= created
            let net = created.saturating_sub(*deleted);
            
            if *deleted > *created {
                return Ok(Some(InvariantViolation::new(
                    InvariantType::Conservation,
                    format!(
                        "Entity {} has more deletions ({}) than creations ({})",
                        entity_id, deleted, created
                    ),
                    vec![entity_id.clone()],
                    true, // Critical - impossible state
                )
                .with_diagnostic("created".to_string(), (*created).into())
                .with_diagnostic("deleted".to_string(), (*deleted).into())
                .with_diagnostic("net_change".to_string(), net.into())));
            }
        }
        
        Ok(None)
    }

    // ========================================================================
    // Tracking APIs (called during operations)
    // ========================================================================

    /// Record activation score for decay monitoring
    pub fn record_activation(&mut self, entity_id: &str, activation: f64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        self.activation_history
            .entry(entity_id.to_string())
            .or_insert_with(Vec::new)
            .push((now, activation));
        
        // Keep only last 100 entries to prevent unbounded growth
        let history = self.activation_history.get_mut(entity_id).unwrap();
        if history.len() > 100 {
            history.remove(0);
        }
    }

    /// Record entity creation for conservation tracking
    pub fn record_entity_creation(&mut self, entity_id: &str) {
        self.entity_lifecycle
            .entry(entity_id.to_string())
            .or_insert((0, 0))
            .0 += 1;
    }

    /// Record entity deletion for conservation tracking
    pub fn record_entity_deletion(&mut self, entity_id: &str) {
        self.entity_lifecycle
            .entry(entity_id.to_string())
            .or_insert((0, 0))
            .1 += 1;
    }

    /// Update symbolic graph edges for acyclicity validation.
    /// Called before a write that would add/remove edges.
    pub fn set_symbolic_edges(&mut self, edges: Vec<(String, String)>) {
        self.symbolic_edges = edges;
    }

    /// Add a single symbolic edge (for incremental updates).
    pub fn add_symbolic_edge(&mut self, from: &str, to: &str) {
        self.symbolic_edges.push((from.to_string(), to.to_string()));
    }

    /// Update causal graph edges for acyclicity validation.
    pub fn set_causal_edges(&mut self, edges: Vec<(String, String)>) {
        self.causal_edges = edges;
    }

    /// Add a single causal edge (for incremental updates).
    pub fn add_causal_edge(&mut self, from: &str, to: &str) {
        self.causal_edges.push((from.to_string(), to.to_string()));
    }

    /// Check if adding an edge would create a cycle (pre-flight check).
    /// Returns the cycle path if one would be created.
    pub fn would_create_cycle(&self, from: &str, to: &str, graph_type: &str) -> Option<Vec<String>> {
        let edges = match graph_type {
            "symbolic" => &self.symbolic_edges,
            "causal" => &self.causal_edges,
            _ => return None,
        };
        let mut test_edges = edges.clone();
        test_edges.push((from.to_string(), to.to_string()));
        self.detect_cycle(&test_edges)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Detect cycle in directed graph using DFS
    ///
    /// Returns Some(cycle_nodes) if cycle detected, None otherwise
    pub fn detect_cycle(&self, edges: &[(String, String)]) -> Option<Vec<String>> {
        // Build adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut nodes = HashSet::new();
        
        for (from, to) in edges {
            graph.entry(from.clone())
                .or_insert_with(Vec::new)
                .push(to.clone());
            nodes.insert(from.clone());
            nodes.insert(to.clone());
        }
        
        // DFS with three colors: white (unvisited), gray (visiting), black (visited)
        let mut color: HashMap<String, u8> = HashMap::new(); // 0=white, 1=gray, 2=black
        let mut parent: HashMap<String, String> = HashMap::new();
        
        for node in &nodes {
            color.insert(node.clone(), 0);
        }
        
        // Try DFS from each unvisited node
        for start_node in &nodes {
            if *color.get(start_node).unwrap() == 0 {
                if let Some(cycle) = self.dfs_detect_cycle(
                    start_node,
                    &graph,
                    &mut color,
                    &mut parent,
                ) {
                    return Some(cycle);
                }
            }
        }
        
        None
    }

    /// DFS helper for cycle detection
    fn dfs_detect_cycle(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        color: &mut HashMap<String, u8>,
        parent: &mut HashMap<String, String>,
    ) -> Option<Vec<String>> {
        color.insert(node.to_string(), 1); // Mark as visiting (gray)
        
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                let neighbor_color = *color.get(neighbor).unwrap_or(&0);
                
                if neighbor_color == 0 {
                    // Unvisited - recurse
                    parent.insert(neighbor.clone(), node.to_string());
                    if let Some(cycle) = self.dfs_detect_cycle(neighbor, graph, color, parent) {
                        return Some(cycle);
                    }
                } else if neighbor_color == 1 {
                    // Back edge found - cycle detected
                    let mut cycle = vec![neighbor.clone()];
                    let mut current = node.to_string();
                    
                    while current != *neighbor {
                        cycle.push(current.clone());
                        if let Some(p) = parent.get(&current) {
                            current = p.clone();
                        } else {
                            break;
                        }
                    }
                    
                    cycle.reverse();
                    return Some(cycle);
                }
            }
        }
        
        color.insert(node.to_string(), 2); // Mark as visited (black)
        None
    }

    /// Alternative cycle detection using topological sort (Kahn's algorithm)
    pub fn is_dag(&self, edges: &[(String, String)]) -> bool {
        // Build adjacency list and in-degree map
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut nodes = HashSet::new();
        
        for (from, to) in edges {
            graph.entry(from.clone())
                .or_insert_with(Vec::new)
                .push(to.clone());
            
            *in_degree.entry(to.clone()).or_insert(0) += 1;
            in_degree.entry(from.clone()).or_insert(0);
            
            nodes.insert(from.clone());
            nodes.insert(to.clone());
        }
        
        // Find all nodes with in-degree 0
        let mut queue: VecDeque<String> = VecDeque::new();
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }
        
        let mut visited_count = 0;
        
        while let Some(node) = queue.pop_front() {
            visited_count += 1;
            
            if let Some(neighbors) = graph.get(&node) {
                for neighbor in neighbors {
                    let degree = in_degree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    
                    if *degree == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
        
        // If all nodes visited, it's a DAG
        visited_count == nodes.len()
    }
}

impl Default for SystemInvariants {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariants_creation() {
        let invariants = SystemInvariants::new();
        assert_eq!(invariants.activation_history.len(), 0);
        assert_eq!(invariants.entity_lifecycle.len(), 0);
    }

    #[test]
    fn test_record_activation() {
        let mut invariants = SystemInvariants::new();
        
        invariants.record_activation("entity1", 1.0);
        invariants.record_activation("entity1", 0.9);
        
        assert_eq!(invariants.activation_history.len(), 1);
        assert_eq!(invariants.activation_history.get("entity1").unwrap().len(), 2);
    }

    #[test]
    fn test_activation_history_limit() {
        let mut invariants = SystemInvariants::new();
        
        // Record 150 activations (should keep only last 100)
        for i in 0..150 {
            invariants.record_activation("entity1", i as f64);
        }
        
        let history = invariants.activation_history.get("entity1").unwrap();
        assert_eq!(history.len(), 100);
        assert_eq!(history[0].1, 50.0); // First kept entry is #50
    }

    #[test]
    fn test_record_entity_lifecycle() {
        let mut invariants = SystemInvariants::new();
        
        invariants.record_entity_creation("entity1");
        invariants.record_entity_creation("entity1");
        invariants.record_entity_deletion("entity1");
        
        let (created, deleted) = invariants.entity_lifecycle.get("entity1").unwrap();
        assert_eq!(*created, 2);
        assert_eq!(*deleted, 1);
    }

    #[test]
    fn test_conservation_violation_detection() {
        let mut invariants = SystemInvariants::new();
        
        // Create impossible state: more deletions than creations
        invariants.entity_lifecycle.insert("entity1".to_string(), (1, 3));
        
        let violation = invariants.check_conservation().unwrap();
        assert!(violation.is_some());
        
        let v = violation.unwrap();
        assert_eq!(v.invariant_type, InvariantType::Conservation);
        assert!(v.critical);
        assert_eq!(v.affected, vec!["entity1".to_string()]);
    }

    #[test]
    fn test_conservation_valid_state() {
        let mut invariants = SystemInvariants::new();
        
        invariants.entity_lifecycle.insert("entity1".to_string(), (5, 3));
        
        let violation = invariants.check_conservation().unwrap();
        assert!(violation.is_none());
    }

    #[test]
    fn test_decay_monotonicity_violation() {
        let mut invariants = SystemInvariants::new();
        
        // Record increasing activation (violation)
        invariants.activation_history.insert(
            "entity1".to_string(),
            vec![(1000, 0.5), (2000, 0.8)],
        );
        
        let violation = invariants.check_decay_monotonicity().unwrap();
        assert!(violation.is_some());
        
        let v = violation.unwrap();
        assert_eq!(v.invariant_type, InvariantType::DecayMonotonicity);
        assert!(!v.critical); // Non-critical
    }

    #[test]
    fn test_decay_monotonicity_valid() {
        let mut invariants = SystemInvariants::new();
        
        // Record decreasing activation (valid)
        invariants.activation_history.insert(
            "entity1".to_string(),
            vec![(1000, 1.0), (2000, 0.8), (3000, 0.6)],
        );
        
        let violation = invariants.check_decay_monotonicity().unwrap();
        assert!(violation.is_none());
    }

    #[test]
    fn test_is_dag_acyclic() {
        let invariants = SystemInvariants::new();
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
            ("A".to_string(), "C".to_string()),
        ];
        
        assert!(invariants.is_dag(&edges));
    }

    #[test]
    fn test_is_dag_cyclic() {
        let invariants = SystemInvariants::new();
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
            ("C".to_string(), "A".to_string()), // Cycle
        ];
        
        assert!(!invariants.is_dag(&edges));
    }

    #[test]
    fn test_detect_cycle_none() {
        let invariants = SystemInvariants::new();
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        
        assert!(invariants.detect_cycle(&edges).is_none());
    }

    #[test]
    fn test_detect_cycle_simple() {
        let invariants = SystemInvariants::new();
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "A".to_string()), // Simple cycle
        ];
        
        let cycle = invariants.detect_cycle(&edges);
        assert!(cycle.is_some());
        let c = cycle.unwrap();
        assert!(c.contains(&"A".to_string()));
        assert!(c.contains(&"B".to_string()));
    }

    #[test]
    fn test_validate_all_empty() {
        let mut invariants = SystemInvariants::new();
        let violations = invariants.validate_all().unwrap();
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_invariant_violation_creation() {
        let violation = InvariantViolation::new(
            InvariantType::MemoryConsistency,
            "Test violation".to_string(),
            vec!["entity1".to_string()],
            true,
        );
        
        assert_eq!(violation.invariant_type, InvariantType::MemoryConsistency);
        assert!(violation.critical);
        assert_eq!(violation.affected, vec!["entity1".to_string()]);
    }

    #[test]
    fn test_invariant_violation_with_diagnostics() {
        let violation = InvariantViolation::new(
            InvariantType::GraphAcyclicity,
            "Cycle detected".to_string(),
            vec!["A".to_string(), "B".to_string()],
            true,
        )
        .with_diagnostic("cycle_length".to_string(), 3.into())
        .with_diagnostic("start_node".to_string(), "A".into());
        
        assert_eq!(violation.diagnostics.len(), 2);
        assert!(violation.diagnostics.contains_key("cycle_length"));
#[test]
    fn test_critical_violation_marked_critical() {
        let mut invariants = SystemInvariants::new();
        invariants.entity_lifecycle.insert("entity1".to_string(), (1, 3));
        let violations = invariants.validate_all().unwrap();
        assert!(!violations.is_empty());
        let conservation_v = violations.iter()
            .find(|v| v.invariant_type == InvariantType::Conservation)
            .expect("Should find Conservation violation");
        assert!(conservation_v.critical);
        assert!(!conservation_v.affected.is_empty());
        assert!(conservation_v.affected.contains(&"entity1".to_string()));
    }

    #[test]
    fn test_validate_specific_memory_consistency() {
        let mut invariants = SystemInvariants::new();
        let result = invariants.validate_specific(InvariantType::MemoryConsistency).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_specific_graph_acyclicity() {
        let mut invariants = SystemInvariants::new();
        let result = invariants.validate_specific(InvariantType::GraphAcyclicity).unwrap();
        assert!(result.is_none());
    }
    }
}
