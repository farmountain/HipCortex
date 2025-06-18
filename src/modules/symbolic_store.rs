/// Chain-of-Thought: insert node -> link edges -> query predicates
use anyhow::Result;
use lru::LruCache;
use petgraph::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::num::NonZeroUsize;
use uuid::Uuid;

/// Node within the symbolic graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolicNode {
    pub id: Uuid,
    pub label: String,
    pub properties: HashMap<String, String>,
}

/// Directed edge between two nodes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolicEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub relation: String,
}

/// Generic query result returned from `GraphDatabase::run_query`.
pub enum GraphResult {
    Nodes(Vec<SymbolicNode>),
    Edges(Vec<SymbolicEdge>),
    Count(usize),
}

/// Abstraction over a graph database used by `SymbolicStore`.
pub trait GraphDatabase {
    fn add_node(&mut self, label: &str, properties: HashMap<String, String>) -> Uuid;
    fn add_edge(&mut self, from: Uuid, to: Uuid, relation: &str);
    fn get_node(&self, node_id: Uuid) -> Option<SymbolicNode>;
    fn neighbors(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicNode>;
    fn edges_from(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicEdge>;
    fn update_property(&mut self, node_id: Uuid, key: &str, value: &str) -> bool;
    fn find_by_label(&mut self, label: &str) -> Vec<SymbolicNode>;
    fn find_by_property(&self, key: &str, value: &str) -> Vec<SymbolicNode>;
    fn remove_node(&mut self, node_id: Uuid) -> bool;
    /// Return all nodes in the graph.
    fn all_nodes(&self) -> Vec<SymbolicNode>;
    /// Return all edges in the graph.
    fn all_edges(&self) -> Vec<SymbolicEdge>;

    /// Verify basic graph invariants such as edge endpoints existing.
    fn assert_graph_invariants(&self) {
        let nodes: HashSet<Uuid> = self.all_nodes().iter().map(|n| n.id).collect();
        for e in self.all_edges() {
            assert!(nodes.contains(&e.from), "edge from missing node");
            assert!(nodes.contains(&e.to), "edge to missing node");
        }
    }

    /// Compute the shortest path between two nodes using BFS by default.
    fn shortest_path(&self, from: Uuid, to: Uuid) -> Option<Vec<Uuid>> {
        let mut visited = HashSet::new();
        let mut q: VecDeque<(Uuid, Vec<Uuid>)> = VecDeque::new();
        q.push_back((from, vec![from]));
        while let Some((cur, path)) = q.pop_front() {
            if cur == to {
                return Some(path);
            }
            if visited.insert(cur) {
                for n in self.neighbors(cur, None) {
                    let mut next_path = path.clone();
                    next_path.push(n.id);
                    q.push_back((n.id, next_path));
                }
            }
        }
        None
    }

    /// Return all connected components via BFS.
    fn connected_components(&self) -> Vec<Vec<Uuid>> {
        let mut comps = Vec::new();
        let mut visited = HashSet::new();
        for node in self.all_nodes() {
            if visited.contains(&node.id) {
                continue;
            }
            let mut comp = Vec::new();
            let mut q = VecDeque::new();
            q.push_back(node.id);
            while let Some(cur) = q.pop_front() {
                if visited.insert(cur) {
                    comp.push(cur);
                    for n in self.neighbors(cur, None) {
                        q.push_back(n.id);
                    }
                }
            }
            comps.push(comp);
        }
        comps
    }

    /// Collect neighbors up to a given depth using BFS.
    fn neighbors_depth(&self, node: Uuid, depth: usize) -> Vec<Uuid> {
        let mut out = Vec::new();
        let mut q = VecDeque::new();
        q.push_back((node, 0usize));
        let mut visited = HashSet::new();
        while let Some((cur, d)) = q.pop_front() {
            if d == depth {
                continue;
            }
            if visited.insert(cur) {
                for n in self.neighbors(cur, None) {
                    out.push(n.id);
                    q.push_back((n.id, d + 1));
                }
            }
        }
        out
    }

    /// Execute a backend specific query if supported.
    fn run_query(&self, _query: &str) -> Result<GraphResult> {
        Err(anyhow::anyhow!("query not supported"))
    }
}

/// Simple in-memory graph backend using `petgraph` with an LRU cache for label lookups.
pub struct InMemoryGraph {
    graph: petgraph::Graph<SymbolicNode, String>,
    id_map: HashMap<Uuid, petgraph::prelude::NodeIndex>,
    label_cache: LruCache<String, Vec<Uuid>>,
}

impl InMemoryGraph {
    pub fn new() -> Self {
        Self {
            graph: petgraph::Graph::new(),
            id_map: HashMap::new(),
            label_cache: LruCache::new(NonZeroUsize::new(32).unwrap()),
        }
    }
}

impl GraphDatabase for InMemoryGraph {
    fn add_node(&mut self, label: &str, properties: HashMap<String, String>) -> Uuid {
        let node = SymbolicNode {
            id: Uuid::new_v4(),
            label: label.to_string(),
            properties,
        };
        let idx = self.graph.add_node(node.clone());
        self.id_map.insert(node.id, idx);
        node.id
    }

    fn add_edge(&mut self, from: Uuid, to: Uuid, relation: &str) {
        if let (Some(f), Some(t)) = (self.id_map.get(&from), self.id_map.get(&to)) {
            self.graph.add_edge(*f, *t, relation.to_string());
        }
    }

    fn get_node(&self, node_id: Uuid) -> Option<SymbolicNode> {
        self.id_map
            .get(&node_id)
            .and_then(|idx| self.graph.node_weight(*idx).cloned())
    }

    fn neighbors(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicNode> {
        if let Some(idx) = self.id_map.get(&node_id) {
            self.graph
                .edges(*idx)
                .filter(|e| relation.map_or(true, |r| r == e.weight()))
                .filter_map(|e| self.graph.node_weight(e.target()).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    fn edges_from(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicEdge> {
        if let Some(idx) = self.id_map.get(&node_id) {
            self.graph
                .edges(*idx)
                .filter(|e| relation.map_or(true, |r| r == e.weight()))
                .map(|e| SymbolicEdge {
                    from: node_id,
                    to: self.graph[e.target()].id,
                    relation: e.weight().clone(),
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn update_property(&mut self, node_id: Uuid, key: &str, value: &str) -> bool {
        if let Some(idx) = self.id_map.get(&node_id) {
            if let Some(node) = self.graph.node_weight_mut(*idx) {
                node.properties.insert(key.to_string(), value.to_string());
                return true;
            }
        }
        false
    }

    fn find_by_label(&mut self, label: &str) -> Vec<SymbolicNode> {
        if let Some(ids) = self.label_cache.get(label).cloned() {
            return ids.into_iter().filter_map(|id| self.get_node(id)).collect();
        }
        let ids: Vec<Uuid> = self
            .graph
            .node_indices()
            .filter_map(|i| {
                let n = &self.graph[i];
                if n.label == label {
                    Some(n.id)
                } else {
                    None
                }
            })
            .collect();
        self.label_cache.put(label.to_string(), ids.clone());
        ids.into_iter().filter_map(|id| self.get_node(id)).collect()
    }

    fn find_by_property(&self, key: &str, value: &str) -> Vec<SymbolicNode> {
        self.graph
            .node_indices()
            .filter_map(|i| {
                let n = &self.graph[i];
                if n.properties.get(key).map_or(false, |v| v == value) {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn remove_node(&mut self, node_id: Uuid) -> bool {
        if let Some(idx) = self.id_map.remove(&node_id) {
            self.graph.remove_node(idx);
            // petgraph removes incident edges automatically
            true
        } else {
            false
        }
    }

    fn all_nodes(&self) -> Vec<SymbolicNode> {
        self.graph
            .node_indices()
            .map(|i| self.graph[i].clone())
            .collect()
    }

    fn all_edges(&self) -> Vec<SymbolicEdge> {
        self.graph
            .edge_references()
            .map(|e| SymbolicEdge {
                from: self.graph[e.source()].id,
                to: self.graph[e.target()].id,
                relation: e.weight().clone(),
            })
            .collect()
    }
}

/// Persistent graph backend backed by a `sled` key-value store.
pub struct SledGraph {
    _db: sled::Db,
    nodes: sled::Tree,
    edges: sled::Tree,
}

impl SledGraph {
    /// Open or create a sled-backed graph at the given path.
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> sled::Result<Self> {
        let db = sled::open(path)?;
        let nodes = db.open_tree("nodes")?;
        let edges = db.open_tree("edges")?;
        Ok(Self {
            _db: db,
            nodes,
            edges,
        })
    }

    fn edge_key(from: Uuid, relation: &str, to: Uuid) -> Vec<u8> {
        let mut key = from.as_bytes().to_vec();
        key.extend_from_slice(relation.as_bytes());
        key.push(0); // separator
        key.extend_from_slice(to.as_bytes());
        key
    }
}

impl GraphDatabase for SledGraph {
    fn add_node(&mut self, label: &str, properties: HashMap<String, String>) -> Uuid {
        let id = Uuid::new_v4();
        let node = SymbolicNode {
            id,
            label: label.to_string(),
            properties,
        };
        let data = serde_json::to_vec(&node).unwrap();
        self.nodes.insert(id.as_bytes(), data).unwrap();
        id
    }

    fn add_edge(&mut self, from: Uuid, to: Uuid, relation: &str) {
        let edge = SymbolicEdge {
            from,
            to,
            relation: relation.to_string(),
        };
        let key = Self::edge_key(from, relation, to);
        let data = serde_json::to_vec(&edge).unwrap();
        self.edges.insert(key, data).unwrap();
    }

    fn get_node(&self, node_id: Uuid) -> Option<SymbolicNode> {
        self.nodes
            .get(node_id.as_bytes())
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_slice(&v).ok())
    }

    fn neighbors(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicNode> {
        let prefix = node_id.as_bytes();
        self.edges
            .scan_prefix(prefix)
            .filter_map(|res| res.ok())
            .filter_map(|(_, v)| serde_json::from_slice::<SymbolicEdge>(&v).ok())
            .filter(|e| relation.map_or(true, |r| r == e.relation))
            .filter_map(|e| self.get_node(e.to))
            .collect()
    }

    fn edges_from(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicEdge> {
        let prefix = node_id.as_bytes();
        self.edges
            .scan_prefix(prefix)
            .filter_map(|res| res.ok())
            .filter_map(|(_, v)| serde_json::from_slice::<SymbolicEdge>(&v).ok())
            .filter(|e| relation.map_or(true, |r| r == e.relation))
            .collect()
    }

    fn update_property(&mut self, node_id: Uuid, key: &str, value: &str) -> bool {
        if let Some(mut node) = self.get_node(node_id) {
            node.properties.insert(key.to_string(), value.to_string());
            let data = serde_json::to_vec(&node).unwrap();
            self.nodes.insert(node_id.as_bytes(), data).unwrap();
            true
        } else {
            false
        }
    }

    fn find_by_label(&mut self, label: &str) -> Vec<SymbolicNode> {
        self.nodes
            .iter()
            .filter_map(|res| res.ok())
            .filter_map(|(_, v)| serde_json::from_slice::<SymbolicNode>(&v).ok())
            .filter(|n| n.label == label)
            .collect()
    }

    fn find_by_property(&self, key: &str, value: &str) -> Vec<SymbolicNode> {
        self.nodes
            .iter()
            .filter_map(|res| res.ok())
            .filter_map(|(_, v)| serde_json::from_slice::<SymbolicNode>(&v).ok())
            .filter(|n| n.properties.get(key).map_or(false, |v| v == value))
            .collect()
    }

    fn remove_node(&mut self, node_id: Uuid) -> bool {
        let existed = self.nodes.remove(node_id.as_bytes()).unwrap().is_some();
        if existed {
            let prefix = node_id.as_bytes().to_vec();
            let edges: Vec<Vec<u8>> = self
                .edges
                .scan_prefix(&prefix)
                .filter_map(|res| res.ok().map(|(k, _)| k.to_vec()))
                .collect();
            for k in edges {
                self.edges.remove(k).unwrap();
            }
        }
        existed
    }

    fn all_nodes(&self) -> Vec<SymbolicNode> {
        self.nodes
            .iter()
            .filter_map(|res| res.ok())
            .filter_map(|(_, v)| serde_json::from_slice::<SymbolicNode>(&v).ok())
            .collect()
    }

    fn all_edges(&self) -> Vec<SymbolicEdge> {
        self.edges
            .iter()
            .filter_map(|res| res.ok())
            .filter_map(|(_, v)| serde_json::from_slice::<SymbolicEdge>(&v).ok())
            .collect()
    }
}

/// High level store that delegates operations to a chosen backend.
pub struct SymbolicStore<B: GraphDatabase> {
    backend: B,
}

impl SymbolicStore<InMemoryGraph> {
    /// Create a new store using the default in-memory backend.
    pub fn new() -> Self {
        Self {
            backend: InMemoryGraph::new(),
        }
    }
}

impl<B: GraphDatabase> SymbolicStore<B> {
    /// Instantiate a store with a custom graph backend.
    pub fn from_backend(backend: B) -> Self {
        Self { backend }
    }

    pub fn add_node(&mut self, label: &str, properties: HashMap<String, String>) -> Uuid {
        self.backend.add_node(label, properties)
    }

    pub fn add_edge(&mut self, from: Uuid, to: Uuid, relation: &str) {
        self.backend.add_edge(from, to, relation)
    }

    pub fn get_node(&self, node_id: Uuid) -> Option<SymbolicNode> {
        self.backend.get_node(node_id)
    }

    pub fn neighbors(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicNode> {
        self.backend.neighbors(node_id, relation)
    }

    pub fn edges_from(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicEdge> {
        self.backend.edges_from(node_id, relation)
    }

    pub fn update_property(&mut self, node_id: Uuid, key: &str, value: &str) -> bool {
        self.backend.update_property(node_id, key, value)
    }

    pub fn find_by_label(&mut self, label: &str) -> Vec<SymbolicNode> {
        self.backend.find_by_label(label)
    }

    pub fn find_by_property(&self, key: &str, value: &str) -> Vec<SymbolicNode> {
        self.backend.find_by_property(key, value)
    }

    pub fn remove_node(&mut self, node_id: Uuid) -> bool {
        self.backend.remove_node(node_id)
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Export the entire graph as lists of nodes and edges.
    pub fn export_graph(&self) -> (Vec<SymbolicNode>, Vec<SymbolicEdge>) {
        (self.backend.all_nodes(), self.backend.all_edges())
    }

    /// Check basic graph invariants.
    pub fn assert_graph_invariants(&self) {
        self.backend.assert_graph_invariants()
    }

    /// Find the shortest path between two nodes.
    pub fn shortest_path(&self, from: Uuid, to: Uuid) -> Option<Vec<Uuid>> {
        self.backend.shortest_path(from, to)
    }

    /// Compute connected components.
    pub fn connected_components(&self) -> Vec<Vec<Uuid>> {
        self.backend.connected_components()
    }

    /// Retrieve neighbors up to a given depth.
    pub fn neighbors_depth(&self, node: Uuid, depth: usize) -> Vec<Uuid> {
        self.backend.neighbors_depth(node, depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_node_and_edge() {
        let mut store = SymbolicStore::new();
        let id_a = store.add_node("A", HashMap::new());
        let id_b = store.add_node("B", HashMap::new());
        store.add_edge(id_a, id_b, "rel");
        assert_eq!(store.neighbors(id_a, Some("rel")).len(), 1);
    }
}
