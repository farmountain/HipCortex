use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Serialize, Deserialize};

use nalgebra::DVector;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopoNode {
    pub symbolic_id: String,
    #[serde(serialize_with = "serialize_embedding", deserialize_with = "deserialize_embedding")]
    pub micro_embedding: [f32; 128],
    pub properties: HashMap<String, String>,
}

fn serialize_embedding<S>(embedding: &[f32; 128], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(128))?;
    for &val in embedding.iter() {
        seq.serialize_element(&val)?;
    }
    seq.end()
}

fn deserialize_embedding<'de, D>(deserializer: D) -> Result<[f32; 128], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Vec<f32> = serde::Deserialize::deserialize(deserializer)?;
    if v.len() != 128 {
        return Err(serde::de::Error::custom(format!(
            "Expected array of 128 elements, got {}",
            v.len()
        )));
    }
    let mut arr = [0.0; 128];
    arr.copy_from_slice(&v[..128]);
    Ok(arr)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EdgeType {
    Causal,
    Temporal,
    Taxonomic,
    Supports,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

    /// Return the `symbolic_id`s of all outgoing (downstream) neighbors.
    /// Returns empty Vec if `symbolic_id` is not in the graph.
    pub fn get_neighbors(&self, symbolic_id: &str) -> Vec<String> {
        match self.id_map.get(symbolic_id) {
            None => vec![],
            Some(&idx) => self
                .graph
                .neighbors(idx)
                .filter_map(|n| self.graph.node_weight(n))
                .map(|n| n.symbolic_id.clone())
                .collect(),
        }
    }

    /// Return the `symbolic_id`s of all incoming (upstream) neighbors.
    /// Returns empty Vec if `symbolic_id` is not in the graph.
    pub fn get_incoming(&self, symbolic_id: &str) -> Vec<String> {
        match self.id_map.get(symbolic_id) {
            None => vec![],
            Some(&idx) => self
                .graph
                .neighbors_directed(idx, petgraph::Direction::Incoming)
                .filter_map(|n| self.graph.node_weight(n))
                .map(|n| n.symbolic_id.clone())
                .collect(),
        }
    }

    /// Personalized PageRank over this directed graph.
    ///
    /// Returns up to `limit` nodes (excluding seed) ranked by their PPR score —
    /// a measure of weighted graph proximity from the seed via power iteration.
    ///
    /// Edge weight = `strength × confidence`. Dangling nodes (no outgoing edges)
    /// contribute only through the teleport term.
    ///
    /// # Parameters
    /// - `seed_id`: symbolic_id of the starting node (e.g. `"mem-{uuid}"`)
    /// - `limit`: maximum number of results to return
    /// - `alpha`: damping factor — 0.85 is standard PageRank (15% restart).
    ///   Fixed at 0.85 at call-sites for v1; expose as a param in a future change
    ///   if empirical evidence calls for a different default.
    /// - `iterations`: power iteration rounds; 20 is sufficient for graphs ≤ 10K nodes.
    ///
    /// Returns empty vec if seed is not in graph, graph is empty, or no
    /// nodes are reachable from the seed.
    pub fn ppr(
        &self,
        seed_id: &str,
        limit: usize,
        alpha: f64,
        iterations: usize,
    ) -> Vec<(String, f64)> {
        let node_count = self.graph.node_count();
        if node_count == 0 || limit == 0 {
            return vec![];
        }

        let seed_idx = match self.id_map.get(seed_id) {
            None     => return vec![],
            Some(&i) => i,
        };

        // Build a contiguous usize position for each NodeIndex so we can use plain Vec
        let all_indices: Vec<petgraph::graph::NodeIndex> = self.graph.node_indices().collect();
        let idx_to_pos: std::collections::HashMap<petgraph::graph::NodeIndex, usize> =
            all_indices.iter().enumerate().map(|(pos, &idx)| (idx, pos)).collect();
        let seed_pos = idx_to_pos[&seed_idx];

        // Build normalized adjacency list: adj[src_pos] = Vec<(dst_pos, weight)>
        // where weights are row-normalized so each source's outgoing weights sum to 1.
        let mut adj: Vec<Vec<(usize, f64)>> = vec![vec![]; node_count];
        for (src_pos, &src_idx) in all_indices.iter().enumerate() {
            // Iterate over raw edge indices from this node using petgraph 0.6 API
            let mut out: Vec<(usize, f64)> = Vec::new();
            for nb_idx in self.graph.neighbors(src_idx) {
                if let Some(edge_idx) = self.graph.find_edge(src_idx, nb_idx) {
                    if let Some(ew) = self.graph.edge_weight(edge_idx) {
                        let w = ew.strength as f64 * ew.confidence as f64;
                        if w > 0.0 {
                            if let Some(&dst) = idx_to_pos.get(&nb_idx) {
                                out.push((dst, w));
                            }
                        }
                    }
                }
            }
            let total: f64 = out.iter().map(|(_, w)| w).sum();
            if total > 0.0 {
                for (_, w) in &mut out {
                    *w /= total;
                }
                adj[src_pos] = out;
            }
            // Dangling nodes (total == 0): contribute only to teleport, handled below
        }

        // Power iteration
        let mut scores     = vec![0.0f64; node_count];
        let mut new_scores = vec![0.0f64; node_count];
        scores[seed_pos] = 1.0;

        for _ in 0..iterations {
            for s in new_scores.iter_mut() { *s = 0.0; }
            // Teleport: (1 - alpha) of all mass returns to seed
            new_scores[seed_pos] += 1.0 - alpha;
            // Diffusion: alpha * normalized adjacency * current scores
            for (src_pos, edges) in adj.iter().enumerate() {
                for &(dst_pos, w) in edges {
                    new_scores[dst_pos] += alpha * w * scores[src_pos];
                }
            }
            std::mem::swap(&mut scores, &mut new_scores);
        }

        // Collect, exclude seed, remove zero-score nodes, sort descending, truncate
        let mut results: Vec<(String, f64)> = all_indices
            .iter()
            .enumerate()
            .filter_map(|(pos, &idx)| {
                if idx == seed_idx { return None; }
                let score = scores[pos];
                if score <= 0.0 { return None; }
                self.graph.node_weight(idx).map(|n| (n.symbolic_id.clone(), score))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
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

    pub fn to_serializable(&self) -> SerializableTopoGraph {
        let nodes = self.graph.node_weights().cloned().collect();
        let mut edges = Vec::new();
        for &src_idx in self.id_map.values() {
            for nb_idx in self.graph.neighbors(src_idx) {
                if let Some(edge_idx) = self.graph.find_edge(src_idx, nb_idx) {
                    if let Some(ew) = self.graph.edge_weight(edge_idx) {
                        let from_node = &self.graph[src_idx];
                        let to_node = &self.graph[nb_idx];
                        edges.push(SerializableGraphEdge {
                            from: from_node.symbolic_id.clone(),
                            to: to_node.symbolic_id.clone(),
                            edge_type: ew.edge_type.clone(),
                            strength: ew.strength,
                            confidence: ew.confidence,
                            last_updated: ew.last_updated,
                        });
                    }
                }
            }
        }
        SerializableTopoGraph { nodes, edges }
    }

    pub fn from_serializable(sg: SerializableTopoGraph) -> Result<Self, String> {
        let mut g = Self::new();
        for n in sg.nodes {
            g.add_node(n.symbolic_id, n.micro_embedding, n.properties)?;
        }
        for e in sg.edges {
            g.add_edge(e.from, e.to, e.edge_type, e.strength, e.confidence)?;
        }
        Ok(g)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableGraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub strength: f32,
    pub confidence: f32,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableTopoGraph {
    pub nodes: Vec<TopoNode>,
    pub edges: Vec<SerializableGraphEdge>,
}
