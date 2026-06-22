use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet, VecDeque};

use nalgebra::DVector;

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

    /// Personalized PageRank using iterative method with nalgebra::DVector.
    /// Seeds receive initial preference mass. Damping default 0.85.
    /// Returns normalized ranks by symbolic id.
    pub fn personalized_pagerank(&self, seeds: &[String], damping: f32, max_iter: usize) -> HashMap<String, f32> {
        let n = self.graph.node_count();
        if n == 0 {
            return HashMap::new();
        }
        let mut node_list: Vec<String> = self.id_map.keys().cloned().collect();
        node_list.sort();
        let idx_of: HashMap<String, usize> = node_list.iter().enumerate().map(|(i, s)| (s.clone(), i)).collect();

        let mut pref = vec![0.0f32; n];
        let num_seeds = seeds.len().max(1) as f32;
        for s in seeds {
            if let Some(&pos) = idx_of.get(s) {
                pref[pos] = 1.0 / num_seeds;
            }
        }
        let mut rank: DVector<f32> = DVector::from_vec(pref.clone());

        // out degrees for normalization
        let mut out_deg: Vec<f32> = vec![0.0; n];
        for (i, nid) in node_list.iter().enumerate() {
            if let Some(&gidx) = self.id_map.get(nid) {
                out_deg[i] = self.graph.neighbors(gidx).count() as f32;
            }
        }

        let d = damping.clamp(0.0, 0.99);
        for _ in 0..max_iter {
            let mut new_r = DVector::<f32>::zeros(n);
            for (i, nid) in node_list.iter().enumerate() {
                if let Some(&gidx) = self.id_map.get(nid) {
                    let mut sum_in = 0.0f32;
                    for in_idx in self.graph.neighbors_directed(gidx, petgraph::Direction::Incoming) {
                        if let Some(in_node) = self.graph.node_weight(in_idx) {
                            if let Some(&src_pos) = idx_of.get(&in_node.symbolic_id) {
                                if out_deg[src_pos] > 0.0 {
                                    sum_in += rank[src_pos] / out_deg[src_pos];
                                }
                            }
                        }
                    }
                    new_r[i] = d * sum_in + (1.0 - d) * pref[i];
                }
            }
            // normalize to keep stable
            let tot: f32 = new_r.iter().sum();
            if tot > 1e-9 {
                new_r /= tot;
            }
            rank = new_r;
        }

        let mut res: HashMap<String, f32> = HashMap::new();
        for (i, nid) in node_list.iter().enumerate() {
            res.insert(nid.clone(), rank[i]);
        }
        res
    }

    /// Find all simple multi-hop paths from -> to with at most max_hops hops (DFS, no cycles in path).
    pub fn find_multi_hop_paths(&self, from: &str, to: &str, max_hops: usize) -> Vec<Vec<String>> {
        let mut res: Vec<Vec<String>> = vec![];
        if !self.id_map.contains_key(from) || !self.id_map.contains_key(to) {
            return res;
        }
        fn dfs(
            g: &CausalTopoGraph,
            curr: &str,
            target: &str,
            rem_hops: usize,
            path: &mut Vec<String>,
            out: &mut Vec<Vec<String>>,
        ) {
            if rem_hops == 0 {
                return;
            }
            if curr == target && path.len() > 1 {
                out.push(path.clone());
                return;
            }
            if let Some(&idx) = g.id_map.get(curr) {
                for neigh in g.graph.neighbors(idx) {
                    if let Some(nw) = g.graph.node_weight(neigh) {
                        let nid = nw.symbolic_id.clone();
                        if !path.contains(&nid) {
                            path.push(nid.clone());
                            dfs(g, &nid, target, rem_hops - 1, path, out);
                            path.pop();
                        }
                    }
                }
            }
        }
        let mut start_path = vec![from.to_string()];
        dfs(self, from, to, max_hops, &mut start_path, &mut res);
        res
    }

    /// Detect if adding (new_from -> new_to, et) would create a contradiction:
    /// - self-loop or would introduce cycle
    /// - or reverse high-strength Causal edge exists
    pub fn detect_contradiction(&self, new_from: String, new_to: String, et: EdgeType) -> bool {
        if new_from == new_to {
            return true;
        }
        // would create cycle if path new_to -> ... -> new_from already
        if self.has_path(&new_to, &new_from) {
            return true;
        }
        // reverse high strength causal conflict (common case)
        if let (Some(&fidx), Some(&tidx)) = (self.id_map.get(&new_from), self.id_map.get(&new_to)) {
            if let Some(eidx) = self.graph.find_edge(tidx, fidx) {
                if let Some(e) = self.graph.edge_weight(eidx) {
                    if matches!(e.edge_type, EdgeType::Causal) && et == EdgeType::Causal && e.strength > 0.5 && e.confidence > 0.5 {
                        return true;
                    }
                }
            }
        }
        // same pair different type could be added but YAGNI for MVP
        false
    }
}
