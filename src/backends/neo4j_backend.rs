#[cfg(feature = "neo4j_backend")]
use anyhow::Result;
#[cfg(feature = "neo4j_backend")]
use reqwest::blocking::Client;
use std::collections::HashMap;
use uuid::Uuid;

use crate::symbolic_store::{GraphDatabase, GraphResult, SymbolicEdge, SymbolicNode};

/// Minimal Neo4j backend skeleton using Cypher over HTTP.
#[cfg(feature = "neo4j_backend")]
pub struct Neo4jBackend {
    endpoint: String,
    client: Client,
}

#[cfg(feature = "neo4j_backend")]
impl Neo4jBackend {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client: Client::new(),
        }
    }
}

#[cfg(feature = "neo4j_backend")]
impl GraphDatabase for Neo4jBackend {
    fn add_node(&mut self, _label: &str, _props: HashMap<String, String>) -> Uuid {
        unimplemented!("Neo4j add_node not implemented")
    }
    fn add_edge(&mut self, _from: Uuid, _to: Uuid, _relation: &str) {}
    fn get_node(&self, _node_id: Uuid) -> Option<SymbolicNode> {
        None
    }
    fn neighbors(&self, _node_id: Uuid, _relation: Option<&str>) -> Vec<SymbolicNode> {
        Vec::new()
    }
    fn edges_from(&self, _node_id: Uuid, _relation: Option<&str>) -> Vec<SymbolicEdge> {
        Vec::new()
    }
    fn update_property(&mut self, _node_id: Uuid, _key: &str, _value: &str) -> bool {
        false
    }
    fn find_by_label(&mut self, _label: &str) -> Vec<SymbolicNode> {
        Vec::new()
    }
    fn find_by_property(&self, _key: &str, _value: &str) -> Vec<SymbolicNode> {
        Vec::new()
    }
    fn remove_node(&mut self, _node_id: Uuid) -> bool {
        false
    }
    fn all_nodes(&self) -> Vec<SymbolicNode> {
        Vec::new()
    }
    fn all_edges(&self) -> Vec<SymbolicEdge> {
        Vec::new()
    }
    fn run_query(&self, _query: &str) -> Result<GraphResult> {
        Err(anyhow::anyhow!("not implemented"))
    }
}
