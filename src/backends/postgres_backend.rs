#[cfg(feature = "postgres_backend")]
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use uuid::Uuid;

use crate::symbolic_store::{GraphDatabase, GraphResult, SymbolicEdge, SymbolicNode};

#[cfg(feature = "postgres_backend")]
pub struct PostgresGraphBackend {
    pool: Pool<Postgres>,
}

#[cfg(feature = "postgres_backend")]
impl PostgresGraphBackend {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "postgres_backend")]
impl GraphDatabase for PostgresGraphBackend {
    fn add_node(&mut self, _label: &str, _props: HashMap<String, String>) -> Uuid {
        unimplemented!()
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
    fn run_query(&self, _query: &str) -> anyhow::Result<GraphResult> {
        Err(anyhow::anyhow!("not implemented"))
    }
}
