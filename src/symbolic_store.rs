use std::collections::{HashMap, HashSet};
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

#[derive(Default)]
pub struct SymbolicStore {
    pub nodes: HashMap<Uuid, SymbolicNode>,
    pub edges: HashSet<SymbolicEdge>,
}

impl SymbolicStore {
    pub fn new() -> Self {
        Self { 
            nodes: HashMap::new(),
            edges: HashSet::new(),
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
        self.edges.iter()
            .filter(|e| e.from == node_id && relation.map_or(true, |r| r == e.relation))
            .filter_map(|e| self.nodes.get(&e.to))
            .collect()
    }
}
