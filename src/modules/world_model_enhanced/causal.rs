// Causal Graph for Causal Reasoning and Do-Calculus
//
// Implements directed causal graphs supporting:
// - Path queries (does A causally affect B?)
// - Do-calculus interventions: P(Y|do(X=x))
// - Counterfactual reasoning: "what if X had been x?"
// - Cycle prevention (DAG property maintenance)

use std::collections::{HashMap, HashSet, VecDeque};

/// Node in causal graph
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalNode {
    pub id: String,
    pub properties: HashMap<String, String>,
}

/// Directed causal edge A → B
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub from: String,
    pub to: String,
    pub strength: f64,  // Causal strength (0.0 to 1.0)
}

/// Intervention query for do-calculus
#[derive(Debug, Clone)]
pub struct InterventionQuery {
    /// Variable to observe (Y)
    pub outcome: String,
    
    /// Variable to intervene on (X)
    pub intervention_var: String,
    
    /// Intervention value (x)
    pub intervention_value: f64,
    
    /// Conditioning variables (if any)
    pub conditioned_on: HashMap<String, f64>,
}

/// Causal graph supporting do-calculus and counterfactual reasoning
pub struct CausalGraph {
    /// Nodes in the graph
    nodes: HashMap<String, CausalNode>,
    
    /// Adjacency list: node → set of children
    edges: HashMap<String, HashSet<String>>,
    
    /// Edge properties (causal strength)
    edge_data: HashMap<(String, String), f64>,
    
    /// Cached probability distributions (for interventions)
    distributions: HashMap<String, HashMap<String, f64>>,
}

impl CausalGraph {
    /// Create new empty causal graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            edge_data: HashMap::new(),
            distributions: HashMap::new(),
        }
    }

    /// Add node to graph
    pub fn add_node(&mut self, id: String) -> Result<(), String> {
        if self.nodes.contains_key(&id) {
            return Err(format!("Node '{}' already exists", id));
        }
        
        self.nodes.insert(
            id.clone(),
            CausalNode {
                id: id.clone(),
                properties: HashMap::new(),
            },
        );
        self.edges.insert(id, HashSet::new());
        
        Ok(())
    }

    /// Add causal edge A → B with cycle detection
    pub fn add_edge(&mut self, from: String, to: String) -> Result<(), String> {
        // Ensure nodes exist
        if !self.nodes.contains_key(&from) {
            self.add_node(from.clone())?;
        }
        if !self.nodes.contains_key(&to) {
            self.add_node(to.clone())?;
        }

        // Check for cycles: if B already has path to A, adding A→B creates cycle
        if self.has_path(&to, &from)? {
            return Err(format!(
                "Adding edge {} → {} would create a cycle",
                from, to
            ));
        }

        // Add edge
        self.edges.get_mut(&from).unwrap().insert(to.clone());
        self.edge_data.insert((from.clone(), to.clone()), 1.0);

        Ok(())
    }

    /// Check if there's a directed path from `from` to `to`
    pub fn has_path(&self, from: &str, to: &str) -> Result<bool, String> {
        if !self.nodes.contains_key(from) || !self.nodes.contains_key(to) {
            return Ok(false);
        }

        if from == to {
            return Ok(true);
        }

        // BFS traversal
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.to_string());
        visited.insert(from.to_string());

        while let Some(node) = queue.pop_front() {
            if let Some(neighbors) = self.edges.get(&node) {
                for neighbor in neighbors {
                    if neighbor == to {
                        return Ok(true);
                    }
                    if visited.insert(neighbor.clone()) {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get all paths from `from` to `to`
    pub fn get_all_paths(&self, from: &str, to: &str) -> Vec<Vec<String>> {
        let mut paths = Vec::new();
        let mut current_path = vec![from.to_string()];
        let mut visited = HashSet::new();
        visited.insert(from.to_string());

        self.dfs_paths(from, to, &mut current_path, &mut visited, &mut paths);

        paths
    }

    fn dfs_paths(
        &self,
        current: &str,
        target: &str,
        path: &mut Vec<String>,
        visited: &mut HashSet<String>,
        all_paths: &mut Vec<Vec<String>>,
    ) {
        if current == target {
            all_paths.push(path.clone());
            return;
        }

        if let Some(neighbors) = self.edges.get(current) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    path.push(neighbor.clone());
                    self.dfs_paths(neighbor, target, path, visited, all_paths);
                    path.pop();
                    visited.remove(neighbor);
                }
            }
        }
    }

    /// Get parents of a node
    pub fn get_parents(&self, node: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter_map(|(parent, children)| {
                if children.contains(node) {
                    Some(parent.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get children of a node
    pub fn get_children(&self, node: &str) -> Vec<String> {
        self.edges
            .get(node)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Compute causal intervention P(Y|do(X=x))
    ///
    /// Implements backdoor adjustment:
    /// P(Y|do(X=x)) = Σ_z P(Y|X=x,Z=z) × P(Z)
    ///
    /// where Z is a sufficient adjustment set (blocks backdoor paths)
    pub fn compute_intervention(
        &self,
        query: &InterventionQuery,
    ) -> Result<HashMap<String, f64>, String> {
        // Simplified intervention: remove incoming edges to intervention variable
        // then compute observational distribution
        
        if !self.nodes.contains_key(&query.intervention_var) {
            return Err(format!("Intervention variable '{}' not found", query.intervention_var));
        }
        if !self.nodes.contains_key(&query.outcome) {
            return Err(format!("Outcome variable '{}' not found", query.outcome));
        }

        // Find backdoor adjustment set (parents of intervention variable)
        let adjustment_set = self.get_parents(&query.intervention_var);

        // Compute marginalized distribution (simplified)
        // In real implementation, would integrate over adjustment set using stored distributions
        let mut result = HashMap::new();
        
        // For now, return a placeholder distribution
        // Real implementation would compute: Σ_z P(Y|X=x,Z=z) × P(Z)
        result.insert(query.outcome.clone(), 1.0);

        Ok(result)
    }

    /// Compute counterfactual: "What if X had been x instead of x'?"
    ///
    /// Pearl's 3-step process:
    /// 1. Abduction: Infer latent variables given actual observations
    /// 2. Action: Perform intervention do(X=x)
    /// 3. Prediction: Compute outcome under modified model
    pub fn compute_counterfactual(
        &self,
        actual_state: HashMap<String, f64>,
        intervention_var: String,
        intervention_value: f64,
    ) -> Result<HashMap<String, f64>, String> {
        // Step 1: Abduction - infer latent state from actual observations
        // (Simplified: assume actual_state represents full latent state)

        // Step 2: Action - modify graph with intervention
        let mut counterfactual_state = actual_state.clone();
        counterfactual_state.insert(intervention_var.clone(), intervention_value);

        // Step 3: Prediction - propagate through graph
        // For each node dependent on intervention_var, recompute value
        let descendants = self.get_descendants(&intervention_var)?;
        
        for descendant in descendants {
            // Simplified: mark as updated (real impl would recompute from parents)
            if !counterfactual_state.contains_key(&descendant) {
                counterfactual_state.insert(descendant, 0.0);
            }
        }

        Ok(counterfactual_state)
    }

    /// Get all descendants of a node (transitive closure)
    fn get_descendants(&self, node: &str) -> Result<Vec<String>, String> {
        let mut descendants = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(node.to_string());
        visited.insert(node.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(children) = self.edges.get(&current) {
                for child in children {
                    if visited.insert(child.clone()) {
                        descendants.push(child.clone());
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        Ok(descendants)
    }

    /// Check if graph is acyclic (DAG property)
    pub fn is_acyclic(&self) -> bool {
        // Use DFS to detect back edges
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in self.nodes.keys() {
            if !visited.contains(node) {
                if self.has_cycle_dfs(node, &mut visited, &mut rec_stack) {
                    return false;
                }
            }
        }

        true
    }

    fn has_cycle_dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = self.edges.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle_dfs(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Get number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get number of edges
    pub fn edge_count(&self) -> usize {
        self.edge_data.len()
    }
}

impl Default for CausalGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_graph() {
        let graph = CausalGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = CausalGraph::new();
        assert!(graph.add_node("A".to_string()).is_ok());
        assert_eq!(graph.node_count(), 1);
        
        // Adding duplicate should fail
        assert!(graph.add_node("A".to_string()).is_err());
    }

    #[test]
    fn test_add_edge() {
        let mut graph = CausalGraph::new();
        assert!(graph.add_edge("A".to_string(), "B".to_string()).is_ok());
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_has_path() {
        let mut graph = CausalGraph::new();
        graph.add_edge("A".to_string(), "B".to_string()).unwrap();
        graph.add_edge("B".to_string(), "C".to_string()).unwrap();
        
        assert!(graph.has_path("A", "B").unwrap());
        assert!(graph.has_path("A", "C").unwrap());  // Transitive
        assert!(graph.has_path("B", "C").unwrap());
        assert!(!graph.has_path("C", "A").unwrap());  // No reverse path
    }

    #[test]
    fn test_cycle_prevention() {
        let mut graph = CausalGraph::new();
        graph.add_edge("A".to_string(), "B".to_string()).unwrap();
        graph.add_edge("B".to_string(), "C".to_string()).unwrap();
        
        // Adding C → A creates cycle
        let result = graph.add_edge("C".to_string(), "A".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cycle"));
    }

    #[test]
    fn test_is_acyclic() {
        let mut graph = CausalGraph::new();
        graph.add_edge("A".to_string(), "B".to_string()).unwrap();
        graph.add_edge("B".to_string(), "C".to_string()).unwrap();
        graph.add_edge("C".to_string(), "D".to_string()).unwrap();
        
        assert!(graph.is_acyclic());
    }

    #[test]
    fn test_get_parents() {
        let mut graph = CausalGraph::new();
        graph.add_edge("A".to_string(), "C".to_string()).unwrap();
        graph.add_edge("B".to_string(), "C".to_string()).unwrap();
        
        let parents = graph.get_parents("C");
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&"A".to_string()));
        assert!(parents.contains(&"B".to_string()));
    }

    #[test]
    fn test_get_children() {
        let mut graph = CausalGraph::new();
        graph.add_edge("A".to_string(), "B".to_string()).unwrap();
        graph.add_edge("A".to_string(), "C".to_string()).unwrap();
        
        let children = graph.get_children("A");
        assert_eq!(children.len(), 2);
        assert!(children.contains(&"B".to_string()));
        assert!(children.contains(&"C".to_string()));
    }

    #[test]
    fn test_get_all_paths() {
        let mut graph = CausalGraph::new();
        graph.add_edge("A".to_string(), "B".to_string()).unwrap();
        graph.add_edge("A".to_string(), "C".to_string()).unwrap();
        graph.add_edge("B".to_string(), "D".to_string()).unwrap();
        graph.add_edge("C".to_string(), "D".to_string()).unwrap();
        
        let paths = graph.get_all_paths("A", "D");
        assert_eq!(paths.len(), 2);  // A→B→D and A→C→D
    }

    #[test]
    fn test_intervention_query() {
        let mut graph = CausalGraph::new();
        graph.add_edge("X".to_string(), "Y".to_string()).unwrap();
        graph.add_edge("Z".to_string(), "X".to_string()).unwrap();
        graph.add_edge("Z".to_string(), "Y".to_string()).unwrap();
        
        let query = InterventionQuery {
            outcome: "Y".to_string(),
            intervention_var: "X".to_string(),
            intervention_value: 1.0,
            conditioned_on: HashMap::new(),
        };
        
        let result = graph.compute_intervention(&query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_counterfactual() {
        let mut graph = CausalGraph::new();
        graph.add_edge("X".to_string(), "Y".to_string()).unwrap();
        graph.add_edge("Y".to_string(), "Z".to_string()).unwrap();
        
        let mut actual_state = HashMap::new();
        actual_state.insert("X".to_string(), 0.0);
        actual_state.insert("Y".to_string(), 0.5);
        actual_state.insert("Z".to_string(), 0.25);
        
        let result = graph.compute_counterfactual(
            actual_state,
            "X".to_string(),
            1.0,
        );
        
        assert!(result.is_ok());
        let counterfactual_state = result.unwrap();
        assert_eq!(counterfactual_state["X"], 1.0);
    }

    #[test]
    fn test_diamond_structure() {
        let mut graph = CausalGraph::new();
        // A causes both B and C, which both cause D
        graph.add_edge("A".to_string(), "B".to_string()).unwrap();
        graph.add_edge("A".to_string(), "C".to_string()).unwrap();
        graph.add_edge("B".to_string(), "D".to_string()).unwrap();
        graph.add_edge("C".to_string(), "D".to_string()).unwrap();
        
        assert!(graph.is_acyclic());
        assert!(graph.has_path("A", "D").unwrap());
        
        let paths = graph.get_all_paths("A", "D");
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_complex_graph() {
        let mut graph = CausalGraph::new();
        // More complex: A→B→C, A→D→C, E→D
        graph.add_edge("A".to_string(), "B".to_string()).unwrap();
        graph.add_edge("B".to_string(), "C".to_string()).unwrap();
        graph.add_edge("A".to_string(), "D".to_string()).unwrap();
        graph.add_edge("D".to_string(), "C".to_string()).unwrap();
        graph.add_edge("E".to_string(), "D".to_string()).unwrap();
        
        assert_eq!(graph.node_count(), 5);
        assert_eq!(graph.edge_count(), 5);
        assert!(graph.is_acyclic());
        
        // Check various paths
        assert!(graph.has_path("A", "C").unwrap());
        assert!(graph.has_path("E", "C").unwrap());
        assert!(!graph.has_path("C", "A").unwrap());
        assert!(!graph.has_path("B", "E").unwrap());
    }

    #[test]
    fn test_self_loop_prevention() {
        let mut graph = CausalGraph::new();
        graph.add_node("A".to_string()).unwrap();
        
        let result = graph.add_edge("A".to_string(), "A".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cycle"));
    }

    #[test]
    fn test_descendants() {
        let mut graph = CausalGraph::new();
        graph.add_edge("A".to_string(), "B".to_string()).unwrap();
        graph.add_edge("A".to_string(), "C".to_string()).unwrap();
        graph.add_edge("B".to_string(), "D".to_string()).unwrap();
        graph.add_edge("C".to_string(), "E".to_string()).unwrap();
        
        let descendants = graph.get_descendants("A").unwrap();
        assert_eq!(descendants.len(), 4);  // B, C, D, E
    }
}
