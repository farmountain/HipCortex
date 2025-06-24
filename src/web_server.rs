#[cfg(feature = "web-server")]
use crate::symbolic_store::{InMemoryGraph, SymbolicStore};
use axum::extract::Path;
#[cfg(feature = "web-server")]
use axum::{routing::get, Json, Router};
#[cfg(feature = "web-server")]
use std::net::SocketAddr;
#[cfg(feature = "web-server")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "web-server")]
pub async fn run(addr: SocketAddr) {
    let store = Arc::new(Mutex::new(SymbolicStore::new()));
    run_with_store(addr, store).await;
}

#[cfg(feature = "web-server")]
pub async fn run_with_store(addr: SocketAddr, store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>) {
    let graph_route = {
        let store = store.clone();
        get(move || async move {
            let store = store.lock().unwrap();
            let graph = store.export_graph();
            Json(graph)
        })
    };
    let node_route = {
        let store = store.clone();
        get(move |Path(id): Path<String>| async move {
            let node = {
                let store = store.lock().unwrap();
                uuid::Uuid::parse_str(&id)
                    .ok()
                    .and_then(|u| store.get_node(u))
            };
            Json(node)
        })
    };
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/graph", graph_route)
        .route("/node/:id", node_route);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
}
