use lru::LruCache;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolicNode {
    pub id: Uuid,
    pub label: String,
    pub properties: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SymbolicEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub relation: String,
}

pub struct SymbolicStore {
    pub nodes: HashMap<Uuid, SymbolicNode>,
    pub edges: HashSet<SymbolicEdge>,
    label_cache: LruCache<String, Vec<Uuid>>,
}

impl SymbolicStore {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashSet::new(),
            label_cache: LruCache::new(NonZeroUsize::new(32).unwrap()),
        }
    }

    pub fn add_node(&mut self, label: &str, properties: HashMap<String, String>) -> Uuid {
        let node = SymbolicNode {
            id: Uuid::new_v4(),
            label: label.to_string(),
            properties,
        };
        let node_id = node.id;
        self.nodes.insert(node_id, node);
        node_id
    }

    pub fn add_edge(&mut self, from: Uuid, to: Uuid, relation: &str) {
        let edge = SymbolicEdge {
            from,
            to,
            relation: relation.to_string(),
        };
        self.edges.insert(edge);
    }

    pub fn get_node(&self, node_id: Uuid) -> Option<&SymbolicNode> {
        self.nodes.get(&node_id)
    }

    pub fn neighbors(&self, node_id: Uuid, relation: Option<&str>) -> Vec<&SymbolicNode> {
        self.edges
            .iter()
            .filter(|e| e.from == node_id && relation.map_or(true, |r| r == e.relation))
            .filter_map(|e| self.nodes.get(&e.to))
            .collect()
    }

    pub fn edges_from(&self, node_id: Uuid, relation: Option<&str>) -> Vec<&SymbolicEdge> {
        self.edges
            .iter()
            .filter(|e| e.from == node_id && relation.map_or(true, |r| r == e.relation))
            .collect()
    }

    pub fn update_property(&mut self, node_id: Uuid, key: &str, value: &str) -> bool {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.properties.insert(key.to_string(), value.to_string());
            true
        } else {
            false
        }
    }

    pub fn find_by_label(&mut self, label: &str) -> Vec<&SymbolicNode> {
        if let Some(ids) = self.label_cache.get(label).cloned() {
            return ids
                .into_iter()
                .filter_map(|id| self.nodes.get(&id))
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
            .filter_map(|id| self.nodes.get(&id))
            .collect()
    }

    pub fn find_by_property(&self, key: &str, value: &str) -> Vec<&SymbolicNode> {
        self.nodes
            .values()
            .filter(|n| n.properties.get(key).map_or(false, |v| v == value))
            .collect()
    }

    pub fn remove_node(&mut self, node_id: Uuid) -> bool {
        let existed = self.nodes.remove(&node_id).is_some();
        if existed {
            self.edges.retain(|e| e.from != node_id && e.to != node_id);
        }
        existed
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
