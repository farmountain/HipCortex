use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TopoNode {
    pub symbolic_id: String,
    pub micro_embedding: [f32; 128],
    pub properties: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeType {
    Causal,
    Temporal,
    Taxonomic,
    Supports,
}

#[derive(Clone, Debug)]
pub struct TopoEdge {
    pub edge_type: EdgeType,
    pub strength: f32,
    pub confidence: f32,
    pub last_updated: u64,
}

pub struct CausalTopoGraph {
    graph: DiGraph<TopoNode, TopoEdge>,
    id_map: HashMap<String, NodeIndex>,
}

impl CausalTopoGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            id_map: HashMap::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }
}
