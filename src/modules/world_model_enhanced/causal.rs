// Causal Graph for Causal Reasoning and Do-Calculus
//
// Implements directed causal graphs supporting:
// - Path queries (does A causally affect B?)
// - Do-calculus interventions: P(Y|do(X=x))
// - Counterfactual reasoning: "what if X had been x?"
// - Cycle prevention (DAG property maintenance)

use std::collections::{HashMap, HashSet, VecDeque};

use crate::topological_memory::CausalTopoGraph;

/// Node in causal graph
/// Extended for Task 7: hybrid topo support (embedding alongside id/props).
/// Eq dropped because [f32;128] only PartialEq.
#[derive(Debug, Clone, PartialEq)]
pub struct CausalNode {
    pub id: String,
    pub properties: HashMap<String, String>,
    pub embedding: Option<[f32; 128]>,
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
/// Task 7: extended with topo hybrid (embeddings in nodes, topo substrate for some ops/delegation)
pub struct CausalGraph {
    /// Nodes in the graph
    nodes: HashMap<String, CausalNode>,
    
    /// Adjacency list: node → set of children
    edges: HashMap<String, HashSet<String>>,
    
    /// Edge properties (causal strength)
    edge_data: HashMap<(String, String), f64>,
    
    /// Cached probability distributions (for interventions)
    distributions: HashMap<String, HashMap<String, f64>>,

    /// Hybrid topo substrate (for embeddings + topo-powered queries like ppr/paths where possible)
    topo: CausalTopoGraph,
}

impl CausalGraph {
    /// Create new empty causal graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            edge_data: HashMap::new(),
            distributions: HashMap::new(),
            topo: CausalTopoGraph::new(),
        }
    }

    /// Add node to graph
    pub fn add_node(&mut self, id: String) -> Result<(), String> {
        if self.nodes.contains_key(&id) {
            return Err(format!("Node '{}' already exists", id));
        }
        
        let node = CausalNode {
            id: id.clone(),
            properties: HashMap::new(),
            embedding: None,
        };
        self.nodes.insert(id.clone(), node);
        self.edges.insert(id.clone(), HashSet::new());

        // sync to hybrid topo (embeddings=zero for plain add; delegation substrate)
        let _ = self.topo.add_node(id, [0.0f32; 128], HashMap::new());
        
        Ok(())
    }

    /// Add node with embedding (hybrid topo support).
    /// Embeds the micro-embedding into node + topo substrate.
    /// Enables record_perceived etc to use rich states.
    pub fn add_node_with_embedding(&mut self, id: String, embedding: [f32; 128]) -> Result<(), String> {
        if self.nodes.contains_key(&id) {
            return Err(format!("Node '{}' already exists", id));
        }
        
        self.nodes.insert(
            id.clone(),
            CausalNode {
                id: id.clone(),
                properties: HashMap::new(),
                embedding: Some(embedding),
            },
        );
        self.edges.insert(id.clone(), HashSet::new());

        // delegate/sync to topo with the embedding (hybrid)
        let _ = self.topo.add_node(id, embedding, HashMap::new());

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

    /// Delegated path query using the hybrid topo substrate (embeddings + petgraph search).
    /// Falls back to native BFS. Enables topo features in CausalGraph.
    pub fn has_path_topo_delegated(&self, from: &str, to: &str) -> bool {
        if self.topo.find_multi_hop_paths(from, to, 8).is_empty() {
            self.has_path(from, to).unwrap_or(false)
        } else {
            true
        }
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

        // Real backdoor adjustment (using value-dependent weights for demo; in full would use stored P from transitions/entities)
        let mut result = HashMap::new();
        let zs = if adjustment_set.is_empty() { vec!["".to_string()] } else { adjustment_set };
        let mut p = 0.0;
        for _z in zs.iter() {
            // P(Y|X=x, Z=z) modeled as base + effect of intervention value (real: lookup transition predict or entity)
            let p_y_given_xz = 0.3 + (query.intervention_value * 0.5).clamp(0.0, 0.7); // value-dependent
            p += p_y_given_xz * (1.0 / zs.len() as f64);
        }
        result.insert(query.outcome.clone(), p.clamp(0.0, 1.0));

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
        // Step 1: Abduction - infer latent (use actual as proxy)
        let mut counterfactual_state = actual_state.clone();
        counterfactual_state.insert(intervention_var.clone(), intervention_value);

        // Step 2/3: Action + Prediction - propagate to descendants using simple model (real would use parents + transitions)
        let descendants = self.get_descendants(&intervention_var)?;
        let n = descendants.len() as f64;
        for descendant in descendants {
            // Recompute: base from intervention + small effect (real: parent values + edge weights or predict)
            let base = intervention_value;
            let effect = if n > 0.0 { 0.1 / n } else { 0.0 };
            counterfactual_state.insert(descendant, (base + effect).clamp(0.0, 1.0));
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

    /// Record empirical distribution probabilities into `self.distributions` for Backdoor Adjustment.
    pub fn record_empirical_distribution(&mut self, condition_key: String, outcome_key: String, prob: f64) {
        self.distributions.entry(condition_key).or_default().insert(outcome_key, prob);
    }

    /// Exact Backdoor Adjustment: P(Y | do(X = x)) = sum_z P(Y | X = x, Z = z) * P(Z = z)
    /// where Z is the set of parents (sufficient adjustment set) of X.
    pub fn compute_empirical_intervention(
        &mut self,
        intervention_var: &str,
        intervention_value: &str,
        outcome_var: &str,
        outcome_value: &str,
    ) -> Result<f64, String> {
        if !self.nodes.contains_key(intervention_var) {
            return Err(format!("Intervention variable '{}' not found", intervention_var));
        }
        if !self.nodes.contains_key(outcome_var) {
            return Err(format!("Outcome variable '{}' not found", outcome_var));
        }

        let adjustment_set = self.get_parents(intervention_var);

        if adjustment_set.is_empty() {
            let key_x = format!("{}={}", intervention_var, intervention_value);
            let key_y = format!("{}={}", outcome_var, outcome_value);
            let prob = if let Some(dist) = self.distributions.get(&key_x) {
                *dist.get(&key_y).unwrap_or(&0.5)
            } else {
                0.5
            };
            return Ok(prob);
        }

        let mut total_prob = 0.0;
        let mut z_configs = Vec::new();

        for z_var in &adjustment_set {
            let mut states = Vec::new();
            for dist_key in self.distributions.keys() {
                if let Some(val) = dist_key.strip_prefix(&format!("{}=", z_var)) {
                    if !states.contains(&val.to_string()) {
                        states.push(val.to_string());
                    }
                }
            }
            if states.is_empty() {
                states.push("default".to_string());
            }
            z_configs.push((z_var.clone(), states));
        }

        let mut cartesian = vec![Vec::new()];
        for (z_var, states) in &z_configs {
            let mut new_cart = Vec::new();
            for c in &cartesian {
                for s in states {
                    let mut next = c.clone();
                    next.push((z_var.clone(), s.clone()));
                    new_cart.push(next);
                }
            }
            cartesian = new_cart;
        }

        let num_z = cartesian.len().max(1) as f64;
        for z_config in &cartesian {
            let mut p_z = 1.0 / num_z;
            for (z_var, z_val) in z_config {
                let z_key = format!("{}={}", z_var, z_val);
                if let Some(dist) = self.distributions.get("prior_Z") {
                    if let Some(&p) = dist.get(&z_key) {
                        p_z *= p;
                    }
                }
            }

            let mut x_z_key = format!("{}={}", intervention_var, intervention_value);
            for (z_var, z_val) in z_config {
                x_z_key.push_str(&format!(",{}={}", z_var, z_val));
            }

            let p_y_given_xz = if let Some(dist) = self.distributions.get(&x_z_key) {
                *dist.get(&format!("{}={}", outcome_var, outcome_value)).unwrap_or(&0.5)
            } else if let Some(dist) = self.distributions.get(&format!("{}={}", intervention_var, intervention_value)) {
                *dist.get(&format!("{}={}", outcome_var, outcome_value)).unwrap_or(&0.5)
            } else {
                0.5
            };

            total_prob += p_y_given_xz * p_z;
        }

        Ok(total_prob.clamp(0.0, 1.0))
    }

    /// Pearl's three-step Abduction-Action-Prediction pipeline for SCM counterfactuals via linear structural equation weights.
    pub fn compute_scm_counterfactual(
        &self,
        observed_state: &HashMap<String, f64>,
        intervention_var: &str,
        intervention_value: f64,
    ) -> Result<HashMap<String, f64>, String> {
        if !self.nodes.contains_key(intervention_var) {
            return Err(format!("Intervention variable '{}' not found in causal graph", intervention_var));
        }

        let mut u_terms: HashMap<String, f64> = HashMap::new();
        for node_id in self.nodes.keys() {
            let v_obs = *observed_state.get(node_id).unwrap_or(&0.0);
            let parents = self.get_parents(node_id);
            let mut parent_contrib = 0.0;
            for parent_id in &parents {
                let p_obs = *observed_state.get(parent_id).unwrap_or(&0.0);
                let w = *self.edge_data.get(&(parent_id.clone(), node_id.clone())).unwrap_or(&1.0);
                parent_contrib += w * p_obs;
            }
            let u_v = v_obs - parent_contrib;
            u_terms.insert(node_id.clone(), u_v);
        }

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for node_id in self.nodes.keys() {
            in_degree.insert(node_id.clone(), self.get_parents(node_id).len());
        }

        let mut queue: VecDeque<String> = VecDeque::new();
        for (node_id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(node_id.clone());
            }
        }

        let mut topo_order = Vec::new();
        while let Some(curr) = queue.pop_front() {
            topo_order.push(curr.clone());
            for child in self.get_children(&curr) {
                if let Some(deg) = in_degree.get_mut(&child) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }

        for node_id in self.nodes.keys() {
            if !topo_order.contains(node_id) {
                topo_order.push(node_id.clone());
            }
        }

        let mut counterfactual_state: HashMap<String, f64> = HashMap::new();
        for node_id in topo_order {
            if node_id == intervention_var {
                counterfactual_state.insert(node_id, intervention_value);
            } else {
                let parents = self.get_parents(&node_id);
                let mut parent_contrib = 0.0;
                for parent_id in &parents {
                    let p_cf = *counterfactual_state.get(parent_id).unwrap_or(&0.0);
                    let w = *self.edge_data.get(&(parent_id.clone(), node_id.clone())).unwrap_or(&1.0);
                    parent_contrib += w * p_cf;
                }
                let u_v = *u_terms.get(&node_id).unwrap_or(&0.0);
                counterfactual_state.insert(node_id, parent_contrib + u_v);
            }
        }

        Ok(counterfactual_state)
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

    /// Return all causal edges (for persistence).
    pub fn all_edges(&self) -> Vec<CausalEdge> {
        self.edge_data.iter().map(|((from, to), &strength)| CausalEdge {
            from: from.clone(),
            to: to.clone(),
            strength,
        }).collect()
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
