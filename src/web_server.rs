#[cfg(feature = "web-server")]
use crate::symbolic_store::{InMemoryGraph, SymbolicStore};
#[cfg(feature = "web-server")]
use crate::memory_record::{MemoryRecord, MemoryType};
#[cfg(feature = "web-server")]
use crate::memory_store::MemoryStore;
#[cfg(feature = "web-server")]
use crate::persistence::MemoryBackend;
#[cfg(feature = "web-server")]
use axum::extract::{Path, Query};
#[cfg(feature = "web-server")]
use axum::{routing::{get, post}, Json, Router, http::StatusCode};
#[cfg(feature = "web-server")]
use std::net::SocketAddr;
#[cfg(feature = "web-server")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "web-server")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct AddMemoryRequest {
    actor: String,
    action: String,
    target: String,
    record_type: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct AddMemoryResponse {
    success: bool,
    record_id: Option<String>,
    error: Option<String>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct QueryMemoryParams {
    actor: Option<String>,
    action: Option<String>,
    record_type: Option<String>,
    limit: Option<usize>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct QueryMemoryResponse {
    records: Vec<MemoryRecordResponse>,
    total: usize,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct MemoryRecordResponse {
    id: String,
    record_type: String,
    timestamp: String,
    actor: String,
    action: String,
    target: String,
    metadata: serde_json::Value,
    integrity: Option<String>,
}

#[cfg(feature = "web-server")]
pub async fn run(addr: SocketAddr) {
    let store = Arc::new(Mutex::new(SymbolicStore::new()));
    run_with_store(addr, store).await;
}

#[cfg(feature = "web-server")]
pub async fn run_with_memory<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr, 
    memory_store: Arc<Mutex<MemoryStore<B>>>,
) {
    let symbolic_store = Arc::new(Mutex::new(SymbolicStore::new()));
    run_with_both_stores(addr, symbolic_store, memory_store).await;
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

#[cfg(feature = "web-server")]
pub async fn run_with_both_stores<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr, 
    symbolic_store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
    memory_store: Arc<Mutex<MemoryStore<B>>>,
) {
    // Symbolic store routes
    let graph_route = {
        let store = symbolic_store.clone();
        get(move || async move {
            let store = store.lock().unwrap();
            let graph = store.export_graph();
            Json(graph)
        })
    };
    let node_route = {
        let store = symbolic_store.clone();
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

    // Memory store routes
    let add_memory_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<AddMemoryRequest>| async move {
            handle_add_memory(store, req).await
        })
    };

    let query_memory_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<QueryMemoryParams>| async move {
            handle_query_memory(store, params).await
        })
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/graph", graph_route)
        .route("/node/:id", node_route)
        .route("/memory/add", add_memory_route)
        .route("/memory/query", query_memory_route);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
}

#[cfg(feature = "web-server")]
async fn handle_add_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    req: AddMemoryRequest,
) -> Result<Json<AddMemoryResponse>, (StatusCode, Json<AddMemoryResponse>)> {
    let record_type = match req.record_type.as_deref() {
        Some("Temporal") => MemoryType::Temporal,
        Some("Symbolic") => MemoryType::Symbolic,
        Some("Procedural") => MemoryType::Procedural,
        Some("Reflexion") => MemoryType::Reflexion,
        Some("Perception") => MemoryType::Perception,
        _ => MemoryType::Temporal, // Default
    };

    let record = MemoryRecord::new(
        record_type,
        req.actor,
        req.action,
        req.target,
        req.metadata.unwrap_or_else(|| serde_json::json!({})),
    );

    match store.lock() {
        Ok(mut store) => {
            match store.add(record.clone()) {
                Ok(_) => Ok(Json(AddMemoryResponse {
                    success: true,
                    record_id: Some(record.id.to_string()),
                    error: None,
                })),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AddMemoryResponse {
                        success: false,
                        record_id: None,
                        error: Some(e.to_string()),
                    }),
                )),
            }
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AddMemoryResponse {
                success: false,
                record_id: None,
                error: Some(format!("Lock error: {}", e)),
            }),
        )),
    }
}

#[cfg(feature = "web-server")]
async fn handle_query_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    params: QueryMemoryParams,
) -> Result<Json<QueryMemoryResponse>, (StatusCode, Json<QueryMemoryResponse>)> {
    match store.lock() {
        Ok(store) => {
            let all_records = store.all();
            let mut filtered_records = all_records.iter().collect::<Vec<_>>();

            // Apply filters
            if let Some(actor) = &params.actor {
                filtered_records.retain(|r| r.actor == *actor);
            }
            if let Some(action) = &params.action {
                filtered_records.retain(|r| r.action == *action);
            }
            if let Some(record_type) = &params.record_type {
                let target_type = match record_type.as_str() {
                    "Temporal" => MemoryType::Temporal,
                    "Symbolic" => MemoryType::Symbolic,
                    "Procedural" => MemoryType::Procedural,
                    "Reflexion" => MemoryType::Reflexion,
                    "Perception" => MemoryType::Perception,
                    _ => return Err((
                        StatusCode::BAD_REQUEST,
                        Json(QueryMemoryResponse { records: vec![], total: 0 }),
                    )),
                };
                filtered_records.retain(|r| r.record_type == target_type);
            }

            // Apply limit
            let limit = params.limit.unwrap_or(100);
            filtered_records.truncate(limit);

            let response_records = filtered_records
                .into_iter()
                .map(|r| MemoryRecordResponse {
                    id: r.id.to_string(),
                    record_type: format!("{:?}", r.record_type),
                    timestamp: r.timestamp.to_rfc3339(),
                    actor: r.actor.clone(),
                    action: r.action.clone(),
                    target: r.target.clone(),
                    metadata: r.metadata.clone(),
                    integrity: r.integrity.clone(),
                })
                .collect::<Vec<_>>();

            Ok(Json(QueryMemoryResponse {
                total: response_records.len(),
                records: response_records,
            }))
        }
        Err(_e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(QueryMemoryResponse {
                records: vec![],
                total: 0,
            }),
        )),
    }
}
