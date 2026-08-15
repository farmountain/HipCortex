#[cfg(feature = "neo4j_backend")]
use hipcortex::backends::neo4j_backend::Neo4jBackend;
#[cfg(feature = "postgres_backend")]
use hipcortex::backends::postgres_backend::PostgresGraphBackend;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use std::collections::HashMap;

#[cfg(feature = "postgres_backend")]
#[test]
fn postgres_store_round_trip() {
    if let Ok(url) = std::env::var("POSTGRES_TEST_URL") {
        let backend = PostgresGraphBackend::connect(&url).unwrap();
        let mut store = SymbolicStore::from_backend(backend);
        let id = store.add_node("A", HashMap::new());
        assert!(store.get_node(id).is_some());
    }
}

#[cfg(feature = "neo4j_backend")]
#[test]
fn neo4j_store_round_trip() {
    if let (Ok(uri), Ok(user), Ok(pass)) = (
        std::env::var("NEO4J_TEST_URI"),
        std::env::var("NEO4J_TEST_USER"),
        std::env::var("NEO4J_TEST_PASS"),
    ) {
        let backend = Neo4jBackend::connect(&uri, &user, &pass).unwrap();
        let mut store = SymbolicStore::from_backend(backend);
        let id = store.add_node("A", HashMap::new());
        assert!(store.get_node(id).is_some());
    }
}

// fallback SIT using in-memory to keep pipeline happy
#[test]
fn in_memory_round_trip() {
    let backend = InMemoryGraph::new();
    let mut store = SymbolicStore::from_backend(backend);
    let id = store.add_node("A", HashMap::new());
    assert!(store.get_node(id).is_some());
}
