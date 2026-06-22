use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet, VecDeque};

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

    pub fn add_node(&mut self, symbolic_id: String, embedding: [f32; 128], props: HashMap<String, String>) -> Result<String, String> {
        if self.id_map.contains_key(&symbolic_id) {
            return Err("exists".into());
        }
        let node = TopoNode { symbolic_id: symbolic_id.clone(), micro_embedding: embedding, properties: props };
        let idx = self.graph.add_node(node);
        self.id_map.insert(symbolic_id.clone(), idx);
        Ok(symbolic_id)
    }

    pub fn add_edge(&mut self, from: String, to: String, et: EdgeType, strength: f32, confidence: f32) -> Result<(), String> {
        if !self.id_map.contains_key(&from) || !self.id_map.contains_key(&to) {
            return Err("missing nodes".into());
        }
        // cycle check adapted from causal.rs has_path / is_acyclic
        if from == to { return Err("self loop".into()); }
        if self.has_path(&to, &from) { return Err("would cycle".into()); }
        let fidx = self.id_map[&from];
        let tidx = self.id_map[&to];
        let edge = TopoEdge { edge_type: et, strength, confidence, last_updated: 0 };
        self.graph.add_edge(fidx, tidx, edge);
        Ok(())
    }

    // helper BFS adapted from causal.rs has_path (but using petgraph NodeIndex + id_map)
    fn has_path(&self, from: &str, to: &str) -> bool {
        if let (Some(&fidx), Some(&tidx)) = (self.id_map.get(from), self.id_map.get(to)) {
            if fidx == tidx {
                return true;
            }

            // BFS traversal using DiGraph neighbors (outgoing)
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(fidx);
            visited.insert(fidx);

            while let Some(node) = queue.pop_front() {
                if node == tidx {
                    return true;
                }
                for neighbor in self.graph.neighbors(node) {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        false
    }

    /// Extract localized subgraph around seeds, implementing basic Markov blanket:
    /// parents + children + co-parents (spouses), limited by max_size.
    /// Adapted from causal.rs get_parents / get_children patterns + petgraph directed neighbors.
    /// Returns a new CausalTopoGraph copy containing the localized nodes/edges.
    pub fn extract_localized_subgraph(&self, seeds: &[String], max_size: usize) -> Self {
        let mut sub = Self::new();
        let mut to_include: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for seed in seeds {
            if to_include.len() >= max_size {
                break;
            }
            if let Some(&idx) = self.id_map.get(seed) {
                if seen.insert(seed.clone()) {
                    to_include.push(seed.clone());
                }

                // parents (incoming)
                for pidx in self.graph.neighbors_directed(idx, petgraph::Direction::Incoming) {
                    if let Some(pnode) = self.graph.node_weight(pidx) {
                        let pid = pnode.symbolic_id.clone();
                        if seen.insert(pid.clone()) && to_include.len() < max_size {
                            to_include.push(pid);
                        }
                    }
                }

                // children (outgoing)
                for cidx in self.graph.neighbors(idx) {
                    if let Some(cnode) = self.graph.node_weight(cidx) {
                        let cid = cnode.symbolic_id.clone();
                        if seen.insert(cid.clone()) && to_include.len() < max_size {
                            to_include.push(cid.clone());
                        }

                        // co-parents (spouses): other parents of this child
                        for spidx in self.graph.neighbors_directed(cidx, petgraph::Direction::Incoming) {
                            if let Some(spnode) = self.graph.node_weight(spidx) {
                                let spid = spnode.symbolic_id.clone();
                                if spid != *seed && seen.insert(spid.clone()) && to_include.len() < max_size {
                                    to_include.push(spid);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Copy selected nodes into sub (reusing add_node for consistency, will succeed as deduped)
        for nid in &to_include {
            if let Some(&orig_idx) = self.id_map.get(nid) {
                if let Some(node) = self.graph.node_weight(orig_idx) {
                    let _ = sub.add_node(
                        node.symbolic_id.clone(),
                        node.micro_embedding,
                        node.properties.clone(),
                    );
                }
            }
        }

        // Copy edges between included nodes
        for from_nid in &to_include {
            if let Some(&fidx) = self.id_map.get(from_nid) {
                for tidx in self.graph.neighbors(fidx) {
                    if let Some(tnode) = self.graph.node_weight(tidx) {
                        let to_nid = &tnode.symbolic_id;
                        if to_include.contains(to_nid) {
                            if let Some(eidx) = self.graph.find_edge(fidx, tidx) {
                                if let Some(edge) = self.graph.edge_weight(eidx) {
                                    let _ = sub.add_edge(
                                        from_nid.clone(),
                                        to_nid.clone(),
                                        edge.edge_type.clone(),
                                        edge.strength,
                                        edge.confidence,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        sub
    }
}
