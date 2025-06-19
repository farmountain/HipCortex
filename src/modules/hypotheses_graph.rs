use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use uuid::Uuid;

use crate::aureus_bridge::ReflexionHypothesis;

/// Node in the reflexion hypotheses graph.
pub struct HypothesisNode {
    pub id: Uuid,
    pub hypothesis: ReflexionHypothesis,
    pub children: Vec<Uuid>,
    pub supports: bool,
}

/// Directed acyclic graph of hypotheses.
pub struct HypothesesGraph {
    graph: DiGraph<HypothesisNode, bool>,
    map: HashMap<Uuid, NodeIndex>,
}

impl HypothesesGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            map: HashMap::new(),
        }
    }

    /// Add a hypothesis node, optionally connecting to a parent.
    pub fn add_hypothesis(
        &mut self,
        hypothesis: ReflexionHypothesis,
        parent: Option<Uuid>,
        supports: bool,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let node = HypothesisNode {
            id,
            hypothesis: hypothesis.clone(),
            children: Vec::new(),
            supports,
        };
        let idx = self.graph.add_node(node);
        self.map.insert(id, idx);
        if let Some(pid) = parent {
            if let Some(pidx) = self.map.get(&pid).cloned() {
                self.graph.add_edge(pidx, idx, supports);
                if let Some(pn) = self.graph.node_weight_mut(pidx) {
                    pn.children.push(id);
                }
            }
        }
        id
    }

    /// Retrieve a hypothesis by id.
    pub fn get_hypothesis(&self, id: Uuid) -> Option<&ReflexionHypothesis> {
        self.map
            .get(&id)
            .and_then(|idx| self.graph.node_weight(*idx))
            .map(|n| &n.hypothesis)
    }

    /// Mutable access to a hypothesis.
    pub fn get_hypothesis_mut(&mut self, id: Uuid) -> Option<&mut ReflexionHypothesis> {
        if let Some(idx) = self.map.get(&id).cloned() {
            self.graph.node_weight_mut(idx).map(|n| &mut n.hypothesis)
        } else {
            None
        }
    }

    /// Remove nodes with confidence below threshold.
    pub fn prune_low_confidence(&mut self, threshold: f32) {
        let remove: Vec<NodeIndex> = self
            .graph
            .node_indices()
            .filter(|idx| self.graph[*idx].hypothesis.confidence < threshold)
            .collect();
        for idx in remove {
            if let Some(node) = self.graph.node_weight(idx) {
                self.map.remove(&node.id);
            }
            self.graph.remove_node(idx);
        }
    }

    /// Check if the graph contains cycles.
    pub fn has_cycles(&self) -> bool {
        is_cyclic_directed(&self.graph)
    }
}
