use crate::symbolic_store::{InMemoryGraph, SymbolicStore};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Simple persistent world model memory built on SymbolicStore.
/// Provides thread safe access and serialization helpers.
#[derive(Clone)]
pub struct WorldModel {
    store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
}

impl WorldModel {
    /// Create a new empty world model.
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(SymbolicStore::new())),
        }
    }

    /// Add a node with label and arbitrary properties.
    pub fn add_state(&self, label: &str, props: Value) {
        let mut map = std::collections::HashMap::new();
        if let Value::Object(obj) = props {
            for (k, v) in obj {
                map.insert(k, v.to_string());
            }
        }
        let mut store = self.store.lock().unwrap();
        store.add_node(label, map);
    }

    /// Export the current symbolic graph as nodes and edges.
    pub fn export(
        &self,
    ) -> (
        Vec<crate::symbolic_store::SymbolicNode>,
        Vec<crate::symbolic_store::SymbolicEdge>,
    ) {
        let store = self.store.lock().unwrap();
        store.export_graph()
    }
}
