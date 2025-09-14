use std::collections::HashMap;
#[cfg(feature = "postgres_backend")]
use tokio::runtime::Runtime;
#[cfg(feature = "postgres_backend")]
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

use crate::symbolic_store::{GraphDatabase, GraphResult, SymbolicEdge, SymbolicNode};

#[cfg(feature = "postgres_backend")]
pub struct PostgresGraphBackend {
    client: Client,
    rt: Runtime,
}

#[cfg(feature = "postgres_backend")]
impl PostgresGraphBackend {
    pub fn connect(conn_str: &str) -> anyhow::Result<Self> {
        let rt = Runtime::new()?;
        let (client, connection) = rt.block_on(tokio_postgres::connect(conn_str, NoTls))?;
        rt.spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("postgres connection error: {e}");
            }
        });
        rt.block_on(async {
            client
                .batch_execute(
                    "CREATE TABLE IF NOT EXISTS nodes(id UUID PRIMARY KEY, label TEXT, properties JSONB);\
                    CREATE TABLE IF NOT EXISTS edges(from_uuid UUID, to_uuid UUID, relation TEXT);",
                )
                .await
                .ok();
        });
        Ok(Self { client, rt })
    }
}

#[cfg(feature = "postgres_backend")]
impl GraphDatabase for PostgresGraphBackend {
    fn add_node(&mut self, label: &str, props: HashMap<String, String>) -> Uuid {
        let id = Uuid::new_v4();
        let props_json = serde_json::to_string(&props).unwrap();
        let client = &self.client;
        self.rt.block_on(async {
            let _ = client
                .execute(
                    "INSERT INTO nodes(id, label, properties) VALUES($1,$2,$3)",
                    &[&id, &label, &props_json],
                )
                .await;
        });
        id
    }

    fn add_edge(&mut self, from: Uuid, to: Uuid, relation: &str) {
        let client = &self.client;
        self.rt.block_on(async {
            let _ = client
                .execute(
                    "INSERT INTO edges(from_uuid, to_uuid, relation) VALUES($1,$2,$3)",
                    &[&from, &to, &relation],
                )
                .await;
        });
    }

    fn get_node(&self, node_id: Uuid) -> Option<SymbolicNode> {
        let client = &self.client;
        self.rt.block_on(async {
            client
                .query_opt(
                    "SELECT label, properties FROM nodes WHERE id = $1",
                    &[&node_id],
                )
                .await
                .ok()
                .flatten()
                .map(|row| {
                    let label: String = row.get(0);
                    let props_str: String = row.get(1);
                    let props: serde_json::Value = serde_json::from_str(&props_str).unwrap_or_default();
                    let map: HashMap<String, String> =
                        serde_json::from_value(props).unwrap_or_default();
                    SymbolicNode {
                        id: node_id,
                        label,
                        properties: map,
                    }
                })
        })
    }

    fn neighbors(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicNode> {
        let client = &self.client;
        let query = match relation {
            Some(_) =>
                "SELECT n.id, n.label, n.properties FROM edges e JOIN nodes n ON e.to_uuid = n.id WHERE e.from_uuid = $1 AND e.relation = $2",
            None =>
                "SELECT n.id, n.label, n.properties FROM edges e JOIN nodes n ON e.to_uuid = n.id WHERE e.from_uuid = $1",
        };
        self.rt.block_on(async {
            let rows = match relation {
                Some(rel) => client.query(query, &[&node_id, &rel]).await,
                None => client.query(query, &[&node_id]).await,
            }
            .unwrap_or_default();
            rows.into_iter()
                .map(|row| {
                    let id: Uuid = row.get(0);
                    let label: String = row.get(1);
                    let props_str: String = row.get(2);
                    let props: serde_json::Value = serde_json::from_str(&props_str).unwrap_or_default();
                    let map: HashMap<String, String> =
                        serde_json::from_value(props).unwrap_or_default();
                    SymbolicNode {
                        id,
                        label,
                        properties: map,
                    }
                })
                .collect()
        })
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    struct DummyPg;
    impl DummyPg {
        fn new() -> Self {
            Self
        }
    }

    #[test]
    fn dummy_pg_ops() {
        let mut map = HashMap::new();
        map.insert("k".to_string(), "v".to_string());
        // ensure helper functions compile
        let _ = map;
    }
}
