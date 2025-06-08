use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
}

/// Simple in-memory graph backend with an LRU cache for label lookups.
pub struct InMemoryGraph {
    nodes: HashMap<Uuid, SymbolicNode>,
    edges: HashSet<SymbolicEdge>,
    label_cache: LruCache<String, Vec<Uuid>>,
}

impl InMemoryGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashSet::new(),
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
        let node_id = node.id;
        self.nodes.insert(node_id, node);
        node_id
    }

    fn add_edge(&mut self, from: Uuid, to: Uuid, relation: &str) {
        let edge = SymbolicEdge {
            from,
            to,
            relation: relation.to_string(),
        };
        self.edges.insert(edge);
    }

    fn get_node(&self, node_id: Uuid) -> Option<SymbolicNode> {
        self.nodes.get(&node_id).cloned()
    }

    fn neighbors(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicNode> {
        self.edges
            .iter()
            .filter(|e| e.from == node_id && relation.map_or(true, |r| r == e.relation))
            .filter_map(|e| self.nodes.get(&e.to).cloned())
            .collect()
    }

    fn edges_from(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicEdge> {
        self.edges
            .iter()
            .filter(|e| e.from == node_id && relation.map_or(true, |r| r == e.relation))
            .cloned()
            .collect()
    }

    fn update_property(&mut self, node_id: Uuid, key: &str, value: &str) -> bool {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.properties.insert(key.to_string(), value.to_string());
            true
        } else {
            false
        }
    }

    fn find_by_label(&mut self, label: &str) -> Vec<SymbolicNode> {
        if let Some(ids) = self.label_cache.get(label).cloned() {
            return ids
                .into_iter()
                .filter_map(|id| self.nodes.get(&id).cloned())
                .collect();
        }
        let ids: Vec<Uuid> = self
            .nodes
            .values()
            .filter(|n| n.label == label)
            .map(|n| n.id)
            .collect();
        self.label_cache.put(label.to_string(), ids.clone());
        ids.into_iter()
            .filter_map(|id| self.nodes.get(&id).cloned())
            .collect()
    }

    fn find_by_property(&self, key: &str, value: &str) -> Vec<SymbolicNode> {
        self.nodes
            .values()
            .filter(|n| n.properties.get(key).map_or(false, |v| v == value))
            .cloned()
            .collect()
    }

    fn remove_node(&mut self, node_id: Uuid) -> bool {
        let existed = self.nodes.remove(&node_id).is_some();
        if existed {
            self.edges.retain(|e| e.from != node_id && e.to != node_id);
        }
        existed
    }
}

/// Persistent graph backend backed by a `sled` key-value store.
pub struct SledGraph {
    db: sled::Db,
    nodes: sled::Tree,
    edges: sled::Tree,
}

impl SledGraph {
    /// Open or create a sled-backed graph at the given path.
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> sled::Result<Self> {
        let db = sled::open(path)?;
        let nodes = db.open_tree("nodes")?;
        let edges = db.open_tree("edges")?;
        Ok(Self { db, nodes, edges })
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
