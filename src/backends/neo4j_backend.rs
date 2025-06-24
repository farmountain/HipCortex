use crate::symbolic_store::{GraphDatabase, GraphResult, SymbolicEdge, SymbolicNode};
#[cfg(feature = "neo4j_backend")]
use neo4rs::{query, Graph, Node};
use std::collections::HashMap;
#[cfg(feature = "neo4j_backend")]
use tokio::runtime::Runtime;
use uuid::Uuid;

#[cfg(feature = "neo4j_backend")]
pub struct Neo4jBackend {
    graph: Graph,
    rt: Runtime,
}

#[cfg(feature = "neo4j_backend")]
impl Neo4jBackend {
    pub fn connect(uri: &str, user: &str, pass: &str) -> anyhow::Result<Self> {
        let rt = Runtime::new()?;
        let graph = Graph::new(uri, user, pass)?;
        Ok(Self { graph, rt })
    }
}

#[cfg(feature = "neo4j_backend")]
impl GraphDatabase for Neo4jBackend {
    fn add_node(&mut self, label: &str, props: HashMap<String, String>) -> Uuid {
        let id = Uuid::new_v4();
        let mut map = props.clone();
        map.insert("id".into(), id.to_string());
        map.insert("label".into(), label.to_string());
        let q = query("CREATE (n $props)").param("props", map);
        let _ = self.rt.block_on(self.graph.run(q));
        id
    }

    fn add_edge(&mut self, from: Uuid, to: Uuid, relation: &str) {
        let q = query("MATCH (a {id: $from}), (b {id: $to}) CREATE (a)-[r:REL]->(b)")
            .param("from", from.to_string())
            .param("to", to.to_string())
            .param("REL", relation);
        let _ = self.rt.block_on(self.graph.run(q));
    }

    fn get_node(&self, node_id: Uuid) -> Option<SymbolicNode> {
        let q = query("MATCH (n {id: $id}) RETURN n").param("id", node_id.to_string());
        self.rt.block_on(async {
            let mut result = self.graph.execute(q).await.ok()?;
            if let Some(row) = result.next().await.ok().flatten() {
                let node: Node = row.get("n").ok()?;
                let label: String = node.get("label").unwrap_or_default();
                let mut props = HashMap::new();
                for key in node.keys() {
                    if key != "id" && key != "label" {
                        if let Ok(val) = node.get::<String>(key) {
                            props.insert(key.to_string(), val);
                        }
                    }
                }
                return Some(SymbolicNode { id: node_id, label, properties: props });
            }
            None
        })
    }

    fn neighbors(&self, node_id: Uuid, relation: Option<&str>) -> Vec<SymbolicNode> {
        let rel_clause = relation.unwrap_or("");
        let rel_query = if rel_clause.is_empty() {
            "MATCH (a {id: $id})-->(b) RETURN b".to_string()
        } else {
            format!("MATCH (a {{id: $id}})-[:{}]->(b) RETURN b", rel_clause)
        };
        let q = query(&rel_query).param("id", node_id.to_string());
        self.rt.block_on(async {
            let mut result = self.graph.execute(q).await.ok()?;
            let mut out = Vec::new();
            while let Some(row) = result.next().await.ok().flatten() {
                let node: Node = row.get("b").ok()?;
                let mut props = HashMap::new();
                for key in node.keys() {
                    if key != "id" && key != "label" {
                        if let Ok(val) = node.get::<String>(key) {
                            props.insert(key.to_string(), val);
                        }
                    }
                }
                let label: String = node.get("label").unwrap_or_default();
                let id = node
                    .get::<String>("id")
                    .ok()
                    .and_then(|s| Uuid::parse_str(&s).ok())
                    .unwrap_or_else(Uuid::new_v4);
                out.push(SymbolicNode { id, label, properties: props });
            }
            Some(out)
        })
        .unwrap_or_default()
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
    use super::*;
    #[test]
    fn dummy_query_format() {
        let mut props = HashMap::new();
        props.insert("foo".to_string(), "bar".to_string());
        let _ = props;
    }
}
