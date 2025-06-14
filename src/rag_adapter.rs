use crate::symbolic_store::{GraphDatabase, SymbolicStore};
use anyhow::Result;
use reqwest::blocking::Client;

pub trait RagAdapter {
    fn retrieve(&self, query: &str) -> Result<Vec<String>>;
}

pub struct LocalRagAdapter<'a, B: GraphDatabase> {
    store: &'a SymbolicStore<B>,
}

impl<'a, B: GraphDatabase> LocalRagAdapter<'a, B> {
    pub fn new(store: &'a SymbolicStore<B>) -> Self {
        Self { store }
    }
}

impl<'a, B: GraphDatabase> RagAdapter for LocalRagAdapter<'a, B> {
    fn retrieve(&self, query: &str) -> Result<Vec<String>> {
        let nodes = self.store.backend().all_nodes();
        Ok(nodes
            .into_iter()
            .filter(|n| n.label.contains(query))
            .map(|n| n.label)
            .collect())
    }
}

pub struct HttpRagAdapter {
    endpoint: String,
    client: Client,
}

impl HttpRagAdapter {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client: Client::new(),
        }
    }
}

impl RagAdapter for HttpRagAdapter {
    fn retrieve(&self, query: &str) -> Result<Vec<String>> {
        let url = format!("{}?q={}", self.endpoint, query);
        let resp = self.client.get(&url).send()?;
        let res: Vec<String> = resp.json()?;
        Ok(res)
    }
}
