#[cfg(feature = "web-server")]
use hipcortex::web_server::run_with_store;
#[cfg(feature = "web-server")]
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore, SymbolicNode, SymbolicEdge};
#[cfg(feature = "web-server")]
use std::collections::HashMap;
#[cfg(feature = "web-server")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "web-server")]
use tokio::time::{sleep, Duration};

#[cfg(feature = "web-server")]
#[tokio::test]
async fn web_server_graph_endpoint() {
    let store = Arc::new(Mutex::new(SymbolicStore::<InMemoryGraph>::new()));
    {
        let mut s = store.lock().unwrap();
        let a = s.add_node("A", HashMap::new());
        let b = s.add_node("B", HashMap::new());
        s.add_edge(a, b, "rel");
    }
    let addr: std::net::SocketAddr = "127.0.0.1:3010".parse().unwrap();
    let srv_store = store.clone();
    let server = tokio::spawn(async move {
        run_with_store(addr, srv_store).await;
    });
    sleep(Duration::from_millis(100)).await;
    let resp = reqwest::get("http://127.0.0.1:3010/graph").await.unwrap();
    let text = resp.text().await.unwrap();
    let parsed: (Vec<SymbolicNode>, Vec<SymbolicEdge>) = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed.0.len(), 2);
    assert_eq!(parsed.1.len(), 1);
    server.abort();
}
