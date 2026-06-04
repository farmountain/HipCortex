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
use axum::http::{HeaderMap, Method, Request};
#[cfg(feature = "web-server")]
use axum::middleware::{self, Next};
#[cfg(feature = "web-server")]
use axum::response::{Html, Response};
#[cfg(feature = "web-server")]
use axum::{routing::{delete, get, patch, post}, Json, Router, http::StatusCode};
#[cfg(feature = "web-server")]
use crate::coherence::CoherenceChecker;
#[cfg(feature = "web-server")]
use crate::world_model_enhanced::WorldModelEnhanced;
#[cfg(feature = "web-server")]
use crate::aureus_bridge::AureusBridge;
#[cfg(feature = "web-server")]
use crate::self_model::SelfModel;
#[cfg(feature = "web-server")]
use std::sync::RwLock;
#[cfg(feature = "web-server")]
use std::collections::HashMap;
#[cfg(feature = "web-server")]
use std::net::SocketAddr;
#[cfg(feature = "web-server")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "web-server")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "web-server")]
use crate::openapi_spec::OPENAPI_SPEC;

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct ForgetActorResponse {
    success: bool,
    actor: String,
    records_deleted: usize,
    symbolic_nodes_deleted: usize,
    error: Option<String>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct CoherenceStatusResponse {
    coherence_score: f64,
    total_checks: u64,
    inconsistencies_found: u64,
    auto_resolutions_succeeded: u64,
    auto_resolutions_failed: u64,
    invariants_validated: u64,
    invariants_violated: u64,
    healthy: bool,
}

/// API tier definitions — limits enforced per billing period
#[cfg(feature = "web-server")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiTier {
    Free,   // 10K records/month, no SLA
    Pro,    // 1M records/month, email support
    Team,   // unlimited, priority support + GDPR endpoints unlocked
}

#[cfg(feature = "web-server")]
impl ApiTier {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pro"  => ApiTier::Pro,
            "team" => ApiTier::Team,
            _      => ApiTier::Free,
        }
    }
    fn as_str(&self) -> &'static str {
        match self { ApiTier::Free => "free", ApiTier::Pro => "pro", ApiTier::Team => "team" }
    }
}

/// Parsed from HIPCORTEX_API_KEYS env var: "key1:free,key2:pro,key3:team"
#[cfg(feature = "web-server")]
fn default_priority_str() -> String { "normal".to_string() }

#[cfg(feature = "web-server")]
fn load_api_keys() -> HashMap<String, ApiTier> {
    std::env::var("HIPCORTEX_API_KEYS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|entry| {
            let mut parts = entry.trim().splitn(2, ':');
            let key  = parts.next()?.to_string();
            let tier = parts.next().unwrap_or("free");
            if key.is_empty() { None } else { Some((key, ApiTier::from_str(tier))) }
        })
        .collect()
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct TierResponse {
    tier: String,
    limits: TierLimits,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct TierLimits {
    records_per_month: Option<u64>,   // None = unlimited
    gdpr_endpoints: bool,
    coherence_endpoints: bool,
    support: String,
}

/// Central server state bundling all stores and intelligence components.
/// Each handler closure Arc-clones only the fields it needs.
#[cfg(feature = "web-server")]
pub struct AppState<B: MemoryBackend + Send + Sync + 'static> {
    pub memory_store:   Arc<Mutex<MemoryStore<B>>>,
    pub symbolic_store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
    /// Dirichlet-Multinomial transitions + Kalman entity tracking + causal DAG
    pub world_model:    Arc<RwLock<WorldModelEnhanced>>,
    /// Bayesian reflexion bridge — &mut self in reflexion_loop, so Mutex
    pub aureus:         Arc<Mutex<AureusBridge>>,
    /// Self-awareness: capability registry, resource monitor, health, decision engine
    pub self_model:     Arc<SelfModel>,
    /// Cross-module consistency checker — persistent, not recreated per request
    pub coherence:      Arc<CoherenceChecker>,
}

/// Manual Clone: all fields are Arc<…> so clone is a ref-count bump regardless of B.
#[cfg(feature = "web-server")]
impl<B: MemoryBackend + Send + Sync + 'static> Clone for AppState<B> {
    fn clone(&self) -> Self {
        Self {
            memory_store:   self.memory_store.clone(),
            symbolic_store: self.symbolic_store.clone(),
            world_model:    self.world_model.clone(),
            aureus:         self.aureus.clone(),
            self_model:     self.self_model.clone(),
            coherence:      self.coherence.clone(),
        }
    }
}

/// POST /memory/search — semantic + keyword search request
#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct SearchMemoryRequest {
    /// Free-text query (used for keyword matching, and as label when no embedding)
    pub query: String,
    /// Optional embedding vector for cosine similarity search
    pub embedding: Option<Vec<f64>>,
    /// Max results (default 10)
    pub limit: Option<usize>,
    /// If provided, auto-generate query embedding before search.
    /// Format: "ollama/<model>" e.g. "ollama/nomic-embed-text"
    /// or "openai/<model>" e.g. "openai/text-embedding-3-small"
    pub embedding_model: Option<String>,
    /// If set, truncate results so total target text fits within max_tokens.
    /// Rough estimate: 1 token ≈ 4 chars. Default: no limit.
    pub max_tokens: Option<usize>,
    /// If true, include quarantined records in results. Default false.
    #[serde(default)]
    pub include_quarantined: Option<bool>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct SearchFlatParams {
    query: Option<String>,
    actor: Option<String>,
    limit: Option<usize>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct SearchFlatResponse {
    memories: Vec<String>,
    total: usize,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct SearchMemoryResponse {
    results: Vec<SearchResult>,
    total: usize,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    score: f64,
    record: MemoryRecordResponse,
}

/// POST /memory/bulk — add multiple records in one HTTP request
#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct BulkAddRequest {
    records: Vec<AddMemoryRequest>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct BulkAddResponse {
    success: bool,
    inserted: usize,
    failed: usize,
    record_ids: Vec<String>,
    errors: Vec<crate::memory_store::BulkAddError>,
}

/// PATCH /memory/update/:id — partial in-place update of a memory record
#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct UpdateMemoryRequest {
    target:     Option<String>,
    action:     Option<String>,
    confidence: Option<f32>,
    source:     Option<String>,
    metadata:   Option<serde_json::Value>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct UpdateMemoryResponse {
    success:    bool,
    record_id:  String,
    version:    u32,
    error:      Option<String>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct LatestMemoryParams {
    actor:  Option<String>,
    action: Option<String>,
    limit:  Option<usize>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct CreateNodeRequest {
    label:      String,
    properties: Option<std::collections::HashMap<String, String>>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct CreateEdgeRequest {
    from_id:  String,
    to_id:    String,
    relation: String,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct GraphWriteResponse {
    success: bool,
    id:      Option<String>,
    error:   Option<String>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct ConsolidateParams {
    actor:     Option<String>,
    threshold: Option<f64>,  // keyword similarity threshold [0.0, 1.0], default 0.8
    dry_run:   Option<bool>, // if true, show what would be merged without writing
}

/// POST /memory/ingest — zero-config smart memory ingest.
/// Automatically classifies record_type, priority, TTL, tags, actor, action
/// from plain text. No memory architecture knowledge required.
#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct IngestRequest {
    /// Plain text to remember
    pub text: String,
    /// Optional actor override (auto-extracted from text if absent)
    pub actor: Option<String>,
    /// Optional session grouping
    pub session_id: Option<String>,
    /// Optional context hint: "meeting", "code", "chat", "sensor", "decision"
    pub context: Option<String>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct IngestResponse {
    pub record_id:      String,
    pub record_type:    String,
    pub priority:       String,
    pub tags:           Vec<String>,
    pub ttl_seconds:    Option<u64>,
    pub confidence:     f32,
    pub actor:          String,
    pub action:         String,
    pub target:         String,
    pub working_memory: bool,
    pub warning:        Option<serde_json::Value>,
}

/// POST /memory/embed — auto-generate embedding then store memory.
/// Calls Ollama (OLLAMA_URL env) or OpenAI (OPENAI_API_KEY env).
#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct EmbedAndAddRequest {
    actor: String,
    action: String,
    target: String,
    record_type: Option<String>,
    metadata: Option<serde_json::Value>,
    /// "ollama/<model>" e.g. "ollama/nomic-embed-text"
    /// or "openai/<model>" e.g. "openai/text-embedding-3-small"
    embedding_model: String,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct AddMemoryRequest {
    actor: String,
    action: String,
    target: String,
    record_type: Option<String>,
    metadata: Option<serde_json::Value>,
    ttl_seconds: Option<u64>,
    /// [0.0, 1.0] reliability signal, default 1.0
    confidence: Option<f32>,
    /// Who or what is writing this memory (e.g. "user-input", "claude-3-7")
    source: Option<String>,
    /// Per-record decay factor [0.0-2.0]. 1.0=normal, <1.0=slower, >1.0=faster. Default 1.0.
    decay_factor: Option<f32>,
    /// Per-record decay half-life in seconds. Overrides server default.
    decay_half_life_secs: Option<u64>,
    /// Tags for categorization and RAG filtering (e.g. ["bug", "decision"])
    #[serde(default)]
    tags: Vec<String>,
    /// Priority: "pinned"|"high"|"normal"|"low". Pinned bypass decay in search. Default "normal".
    #[serde(default = "default_priority_str")]
    priority: String,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct AddMemoryResponse {
    success: bool,
    record_id: Option<String>,
    error: Option<String>,
    warning: Option<serde_json::Value>,  // possible contradiction detected
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct QueryMemoryParams {
    actor: Option<String>,
    /// Comma-separated actor list — returns records for ANY listed actor.
    /// If set, the `actor` single-value param is ignored.
    actors: Option<String>,
    action: Option<String>,
    record_type: Option<String>,
    limit: Option<usize>,
    tags:     Option<String>,    // comma-separated tags filter e.g. "bug,architecture"
    priority: Option<String>,    // filter by priority
    as_of:    Option<String>,    // ISO 8601 timestamp — return records with timestamp <= as_of
    /// If "true", include quarantined records. Default: exclude quarantine.
    include_quarantined: Option<String>,
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
    pub id:          String,
    pub record_type: String,
    pub timestamp:   String,
    pub actor:       String,
    pub action:      String,
    pub target:      String,
    pub metadata:    serde_json::Value,
    pub integrity:   Option<String>,
    /// Reliability signal [0.0, 1.0]. Use to filter low-confidence memories before injection.
    pub confidence:  f32,
    /// Who or what wrote this memory (e.g. "user-input", "claude-3-7").
    pub source:      Option<String>,
    /// "pinned" | "high" | "normal" | "low". Pinned bypasses decay.
    pub priority:    String,
    /// Domain tags for RAG filtering (e.g. ["database", "auth"]).
    pub tags:        Vec<String>,
    /// Update counter. 0 = original. Increments on PATCH /memory/update/:id.
    pub version:     u32,
    /// "active" | "quarantine" | "archived".
    pub status:      String,
    /// Unix timestamp when record expires. None = never expires.
    pub expires_at:  Option<i64>,
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
    let state = AppState {
        memory_store,
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model: Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model: Arc::new(SelfModel::new()),
        coherence: Arc::new(CoherenceChecker::new()),
    };
    run_with_state(addr, state).await;
}

/// Primary server entry point with full intelligence layer.
/// Owns all route registration — intelligence routes are wired here.
/// Placeholder closures for Tasks 5-8 endpoints return {"status":"coming_soon"}.
#[cfg(feature = "web-server")]
pub async fn run_with_state<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr,
    state: AppState<B>,
) {
    // ── Unpack state into locals so closures can capture by value ─────────
    let symbolic_store  = state.symbolic_store.clone();
    let memory_store    = state.memory_store.clone();
    let world_model     = state.world_model.clone();
    let aureus          = state.aureus.clone();
    let self_model_arc  = state.self_model.clone();
    let coherence_arc   = state.coherence.clone();

    // ── Symbolic store routes ─────────────────────────────────────────────
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

    // ── Memory store routes ───────────────────────────────────────────────
    let add_memory_route = {
        let store = memory_store.clone();
        let wm = world_model.clone();
        post(move |Json(req): Json<AddMemoryRequest>| async move {
            handle_add_memory(store, wm, req).await
        })
    };

    let bulk_add_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<BulkAddRequest>| async move {
            handle_bulk_add(store, Json(req)).await
        })
    };

    let query_memory_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<QueryMemoryParams>| async move {
            handle_query_memory(store, params).await
        })
    };

    // Semantic / keyword search: POST /memory/search
    let search_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<SearchMemoryRequest>| async move {
            handle_search_memory(store, Json(req)).await
        })
    };

    // GDPR forget: DELETE /memory/forget/:actor
    let forget_route = {
        let ms = memory_store.clone();
        let ss = symbolic_store.clone();
        delete(move |Path(actor): Path<String>| async move {
            handle_forget_actor(ms, ss, actor).await
        })
    };

    // Embed and add: POST /memory/embed
    let embed_add_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<EmbedAndAddRequest>| async move {
            handle_embed_and_add(store, Json(req)).await
        })
    };

    // Coherence status: GET /coherence/status
    let coherence_route = {
        let c = coherence_arc.clone();
        get(move || { let cc = c.clone(); async move { handle_coherence_status(cc).await } })
    };

    // Live stats: GET /stats
    let stats_route = {
        let store = memory_store.clone();
        get(move || handle_stats(store))
    };

    // Data export: GET /memory/export?actor=optional
    let export_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<QueryMemoryParams>| async move {
            handle_export_memory(store, Query(params)).await
        })
    };

    // Flat search: GET /memory/search-flat?query=&actor=&limit=
    let search_flat_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<SearchFlatParams>| async move {
            handle_search_flat(store, Query(params)).await
        })
    };

    // PATCH /memory/update/:id — versioned in-place update
    let update_route = {
        let store = memory_store.clone();
        patch(move |Path(id): Path<String>, Json(req): Json<UpdateMemoryRequest>| async move {
            handle_update_memory(store, id, Json(req)).await
        })
    };

    // GET /memory/latest — most recent unique fact per (actor, action)
    let latest_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<LatestMemoryParams>| async move {
            handle_latest_memory(store, Query(params)).await
        })
    };

    let audit_verify_route = {
        let store = memory_store.clone();
        get(move || { let s = store.clone(); async move { handle_audit_verify(s).await } })
    };
    let audit_export_route = {
        let store = memory_store.clone();
        get(move || { let s = store.clone(); async move { handle_audit_export(s).await } })
    };

    let create_node_route = {
        let ss = symbolic_store.clone();
        post(move |Json(req): Json<CreateNodeRequest>| async move {
            handle_create_node(ss, Json(req)).await
        })
    };
    let create_edge_route = {
        let ss = symbolic_store.clone();
        post(move |Json(req): Json<CreateEdgeRequest>| async move {
            handle_create_edge(ss, Json(req)).await
        })
    };
    let delete_node_route = {
        let ss = symbolic_store.clone();
        delete(move |Path(id): Path<String>| async move {
            handle_delete_node(ss, id).await
        })
    };
    let consolidate_route = {
        let store = memory_store.clone();
        post(move |Query(params): Query<ConsolidateParams>| async move {
            handle_consolidate(store, Query(params)).await
        })
    };

    let ingest_route = {
        let store = memory_store.clone();
        let wm = world_model.clone();
        post(move |Json(req): Json<IngestRequest>| async move {
            handle_ingest(store, wm, Json(req)).await
        })
    };

    let quarantine_route = {
        let store = memory_store.clone();
        post(move |Path(id): Path<String>| { let s = store.clone(); async move { handle_quarantine_memory(s, id).await } })
    };
    let restore_route = {
        let store = memory_store.clone();
        post(move |Path(id): Path<String>| { let s = store.clone(); async move { handle_restore_memory(s, id).await } })
    };
    let corroborate_route = {
        let store = memory_store.clone();
        post(move |Path(id): Path<String>| { let s = store.clone(); async move { handle_corroborate(s, id).await } })
    };
    let contradict_route = {
        let store = memory_store.clone();
        post(move |Path(id): Path<String>| { let s = store.clone(); async move { handle_contradict(s, id).await } })
    };
    let context_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<ContextRequest>| async move {
            handle_memory_context(store, Json(req)).await
        })
    };

    let metrics_route = {
        let store = memory_store.clone();
        get(move || { let s = store.clone(); async move { handle_prometheus_metrics(s).await } })
    };

    // ── Intelligence routes (filled in Tasks 5-8) ────────────────────────
    let wm_states_route = {
        let wm = world_model.clone();
        get(move || { let w = wm.clone(); async move { handle_wm_states(w).await } })
    };
    let wm_transitions_route = {
        let wm = world_model.clone();
        get(move |Query(p): Query<WmTransitionsParams>| async move {
            handle_wm_transitions(wm, Query(p)).await
        })
    };
    let wm_uncertainty_route = {
        let wm = world_model.clone();
        get(move || { let w = wm.clone(); async move { handle_wm_uncertainty(w).await } })
    };
    let wm_observe_route = {
        let wm = world_model.clone();
        post(move |Json(req): Json<serde_json::Value>| async move {
            handle_wm_observe(wm, Json(req)).await
        })
    };
    let wm_predict_route = {
        let wm = world_model.clone();
        get(move |Query(p): Query<WmPredictParams>| async move {
            handle_wm_predict(wm, Query(p)).await
        })
    };
    let wm_entities_route = {
        let wm = world_model.clone();
        get(move || { let w = wm.clone(); async move { handle_wm_entities(w).await } })
    };
    let wm_entity_route = {
        let wm = world_model.clone();
        post(move |Json(req): Json<serde_json::Value>| async move {
            handle_wm_register_entity(wm, Json(req)).await
        })
    };
    let wm_causal_route = {
        let wm = world_model.clone();
        get(move || { let w = wm.clone(); async move { handle_wm_causal(w).await } })
    };
    let wm_intervention_route = {
        let wm = world_model.clone();
        post(move |Json(req): Json<serde_json::Value>| async move {
            handle_wm_causal_intervention(wm, Json(req)).await
        })
    };
    let wm_counterfactual_route = {
        let wm = world_model.clone();
        post(move |Json(req): Json<serde_json::Value>| async move {
            handle_wm_causal_counterfactual(wm, Json(req)).await
        })
    };
    let memory_reflect_route = {
        let ms = memory_store.clone();
        let au = aureus.clone();
        post(move |Json(req): Json<serde_json::Value>| async move {
            handle_memory_reflect(ms, au, Json(req)).await
        })
    };
    let memory_hypotheses_route = {
        let au = aureus.clone();
        get(move || { let a = au.clone(); async move { handle_memory_hypotheses(a).await } })
    };
    let self_health_route = {
        let sm = self_model_arc.clone();
        get(move || { let s = sm.clone(); async move { handle_self_health(s).await } })
    };
    let app = Router::new()
        .route("/", get(|| async { axum::response::Redirect::permanent("/pricing") }))
        .route("/health", get(|| async { "ok" }))
        .route("/graph", graph_route)
        .route("/node/:id", node_route)
        .route("/memory/add", add_memory_route)
        .route("/memory/bulk", bulk_add_route)
        .route("/memory/embed", embed_add_route)
        .route("/memory/query", query_memory_route)
        .route("/memory/search", search_route)
        .route("/memory/export", export_route)
        .route("/memory/forget/:actor", forget_route)
        .route("/memory/search-flat", search_flat_route)
        .route("/memory/update/:id", update_route)
        .route("/memory/latest", latest_route)
        .route("/audit/verify", audit_verify_route)
        .route("/audit/export", audit_export_route)
        .route("/coherence/status", coherence_route)
        .route("/coherence/inconsistencies", {
            let c = coherence_arc.clone();
            get(move || { let cc = c.clone(); async move { handle_coherence_inconsistencies(cc).await } })
        })
        .route("/worldmodel/status", {
            let wm = world_model.clone();
            get(move || { let w = wm.clone(); async move { handle_worldmodel_status(w).await } })
        })
        .route("/webhooks", get(handle_list_webhooks).post(handle_register_webhook))
        .route("/webhooks/:id", delete(handle_delete_webhook))
        .route("/graph/node", create_node_route)
        .route("/graph/edge", create_edge_route)
        .route("/graph/node/:id", delete_node_route)
        .route("/memory/consolidate", consolidate_route)
        .route("/memory/ingest", ingest_route)
        .route("/memory/quarantine/:id", quarantine_route)
        .route("/memory/restore/:id", restore_route)
        .route("/memory/corroborate/:id", corroborate_route)
        .route("/memory/contradict/:id", contradict_route)
        .route("/memory/context", context_route)
        .route("/metrics", metrics_route)
        .route("/stats", stats_route)
        .route("/tier", get(handle_tier))
        .route("/pricing", get(handle_pricing))
        .route("/openapi.json", get(handle_openapi))
        .route("/ns", get(handle_list_namespaces))
        .route("/regulatory/hold", get(handle_list_regulatory_holds).post(handle_set_regulatory_hold))
        .route("/regulatory/hold/:actor", delete(handle_release_regulatory_hold))
        // ── Intelligence endpoints (Tasks 5-8) ───────────────────────────
        .route("/worldmodel/states",       wm_states_route)
        .route("/worldmodel/transitions",  wm_transitions_route)
        .route("/worldmodel/uncertainty",  wm_uncertainty_route)
        .route("/worldmodel/observe",   wm_observe_route)
        .route("/worldmodel/predict",   wm_predict_route)
        .route("/worldmodel/entities",  wm_entities_route)
        .route("/worldmodel/entity",    wm_entity_route)
        .route("/worldmodel/causal",         wm_causal_route)
        .route("/worldmodel/causal/edge", {
            let wm = world_model.clone();
            post(move |Json(req): Json<serde_json::Value>| async move {
                handle_wm_causal_add_edge(wm, Json(req)).await
            })
        })
        .route("/worldmodel/causal/intervention",   wm_intervention_route)
        .route("/worldmodel/causal/counterfactual", wm_counterfactual_route)
        .route("/memory/reflect",       memory_reflect_route)
        .route("/memory/hypotheses",    memory_hypotheses_route)
        .route("/memory/hypotheses/reset", {
            let au = aureus.clone();
            post(move || { let a = au.clone(); async move {
                match a.lock() {
                    Ok(mut b) => { b.reset_hypotheses(); axum::Json(serde_json::json!({"success": true})) },
                    Err(e)    => axum::Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
                }
            }})
        })
        .route("/self/health",          self_health_route)
        .route("/self/capabilities", {
            let sm_get  = self_model_arc.clone();
            let sm_post = self_model_arc.clone();
            get(move || { let s = sm_get.clone(); async move { handle_self_capabilities(s).await } })
            .post(move |Json(req): Json<serde_json::Value>| async move {
                handle_self_register_capability(sm_post, Json(req)).await
            })
        })
        .route("/self/can-execute", {
            let sm = self_model_arc.clone();
            get(move |Query(p): Query<SelfCanExecuteParams>| async move {
                handle_self_can_execute(sm, Query(p)).await
            })
        })
        .layer(middleware::from_fn(api_key_middleware));

    // G10: Background CoherenceChecker — runs check_consistency every 60s
    {
        let coherence_bg = coherence_arc.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                match coherence_bg.check_consistency() {
                    Ok(reports) => {
                        if !reports.is_empty() {
                            eprintln!("[CoherenceChecker] {} inconsistencies detected", reports.len());
                        }
                    }
                    Err(e) => eprintln!("[CoherenceChecker] check_consistency error: {}", e),
                }
            }
        });
    }

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
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
        .route("/", get(|| async { axum::response::Redirect::permanent("/pricing") }))
        .route("/health", get(|| async { "ok" }))
        .route("/graph", graph_route)
        .route("/node/:id", node_route);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
}

/// Global per-key write counter for tier metering.
/// Resets on server restart (MVP); production: back with Redis or Postgres.
#[cfg(feature = "web-server")]
lazy_static::lazy_static! {
    static ref GLOBAL_METER: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
    static ref WEBHOOKS: Mutex<Vec<WebhookRegistration>> = Mutex::new(Vec::new());
    static ref REGULATORY_HOLDS: Mutex<Vec<RegulatoryHoldRequest>> = Mutex::new(Vec::new());
}

#[cfg(feature = "web-server")]
#[derive(Clone, Serialize, Deserialize)]
pub struct RegulatoryHoldRequest {
    pub actor:  String,
    pub reason: String,
    pub until:  Option<String>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct RegulatoryHoldResponse {
    actor:   String,
    held:    bool,
    reason:  String,
    until:   Option<String>,
    message: String,
}

#[cfg(feature = "web-server")]
async fn handle_set_regulatory_hold(
    Json(req): Json<RegulatoryHoldRequest>,
) -> Json<RegulatoryHoldResponse> {
    let actor  = req.actor.clone();
    let reason = req.reason.clone();
    let until  = req.until.clone();
    if let Ok(mut holds) = REGULATORY_HOLDS.lock() {
        holds.retain(|h| h.actor != actor);
        holds.push(req);
    }
    Json(RegulatoryHoldResponse {
        actor,
        held: true,
        reason,
        until,
        message: "Regulatory hold placed. GDPR forget will be blocked for this actor until hold is lifted.".to_string(),
    })
}

#[cfg(feature = "web-server")]
async fn handle_release_regulatory_hold(
    Path(actor): Path<String>,
) -> Json<serde_json::Value> {
    if let Ok(mut holds) = REGULATORY_HOLDS.lock() {
        let before = holds.len();
        holds.retain(|h| h.actor != actor);
        if holds.len() < before {
            return Json(serde_json::json!({"released": true, "actor": actor}));
        }
    }
    Json(serde_json::json!({"released": false, "actor": actor, "error": "no hold found"}))
}

#[cfg(feature = "web-server")]
async fn handle_list_regulatory_holds() -> Json<serde_json::Value> {
    let holds = REGULATORY_HOLDS.lock().map(|h| h.clone()).unwrap_or_default();
    let total = holds.len();
    Json(serde_json::json!({"holds": holds, "total": total}))
}

#[cfg(feature = "web-server")]
async fn handle_list_namespaces() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "namespaces": ["default"],
        "note": "Full namespace isolation available in Team/Enterprise tier",
        "docs": "https://github.com/farmountain/HipCortex/blob/main/docs/roadmap.md"
    }))
}

#[cfg(feature = "web-server")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRegistration {
    pub id: String,
    pub url: String,
    pub events: Vec<String>,  // ["memory.added", "memory.deleted", "memory.updated"]
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct RegisterWebhookRequest {
    url: String,
    events: Vec<String>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct RegisterWebhookResponse {
    id: String,
    url: String,
    events: Vec<String>,
}

/// Fire webhooks for an event — async, best-effort, does not block the caller.
#[cfg(feature = "web-server")]
fn fire_webhook(event: &str, payload: serde_json::Value) {
    if let Ok(hooks) = WEBHOOKS.lock() {
        let event_str = event.to_string();
        let matching: Vec<String> = hooks.iter()
            .filter(|h| h.events.contains(&event_str) || h.events.contains(&"*".to_string()))
            .map(|h| h.url.clone())
            .collect();
        if matching.is_empty() { return; }
        let body = serde_json::json!({"event": event, "data": payload});
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            for url in matching {
                let _ = client.post(&url)
                    .json(&body)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await;
            }
        });
    }
}

/// Returns `true` if `key` has exceeded its tier write limit (caller should 429).
/// Increments the counter otherwise.
#[cfg(feature = "web-server")]
fn check_meter_limit(key: &str, tier: &ApiTier) -> bool {
    let limit = match tier {
        ApiTier::Free  => Some(10_000u64),
        ApiTier::Pro   => Some(1_000_000u64),
        ApiTier::Team  => None,
    };
    if let Some(max) = limit {
        if let Ok(mut meter) = GLOBAL_METER.lock() {
            let count = meter.entry(key.to_string()).or_insert(0);
            if *count >= max {
                return true; // over limit
            }
            *count += 1;
        }
    }
    false
}

/// Axum middleware: validates X-Api-Key header (when HIPCORTEX_API_KEYS is set),
/// enforces per-tier write quotas on POST /memory/add and POST /memory/search,
/// and stamps X-HipCortex-Tier on every response for observability.
#[cfg(feature = "web-server")]
async fn api_key_middleware<B>(req: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
    // These paths are always public — no auth required
    let path = req.uri().path();
    if path == "/"
        || path == "/health"
        || path == "/pricing"
        || path == "/stats"
        || path == "/openapi.json"
        || path == "/memory/search-flat"
        || path == "/memory/latest"
        || path == "/worldmodel/status"
        || path == "/coherence/inconsistencies"
        || path == "/metrics"
        || path == "/ns"
        || path == "/self/health"
        || path == "/self/capabilities"
        || path == "/self/can-execute"
        || path == "/memory/hypotheses"
        || path == "/worldmodel/predict"
        || path == "/worldmodel/entities"
        || path == "/worldmodel/causal"
        || path == "/worldmodel/states"
        || path == "/worldmodel/transitions"
        || path == "/worldmodel/uncertainty"
    {
        return Ok(next.run(req).await);
    }

    // Namespace prefix: /ns/{namespace}/... routes use same auth
    // This is the foundation for full multi-tenancy (Phase 2 implementation)
    let effective_path = if path.starts_with("/ns/") {
        path.splitn(4, '/').nth(3).map(|p| format!("/{}", p)).unwrap_or_else(|| "/".to_string())
    } else {
        path.to_string()
    };
    let _ = effective_path; // used for future routing

    let keys = load_api_keys();
    if keys.is_empty() {
        return Ok(next.run(req).await); // open / self-hosted mode
    }

    // Check X-Api-Key header first, fall back to ?api_key= query param
    // (needed for platforms like Manus that don't support custom headers)
    let header_key = req
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let provided = if !header_key.is_empty() {
        header_key
    } else {
        // Parse ?api_key= from query string without external deps
        req.uri().query()
            .and_then(|q| {
                q.split('&').find_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next()?;
                    if key == "api_key" { parts.next().map(|v| v.to_string()) } else { None }
                })
            })
            .unwrap_or_default()
    };

    let tier = match keys.get(&provided) {
        Some(t) => t,
        None    => return Err(StatusCode::UNAUTHORIZED),
    };

    // Enforce write quota on memory write endpoint only
    let is_write = req.method() == Method::POST
        && req.uri().path() == "/memory/add";
    if is_write && check_meter_limit(&provided, tier) {
        return Err(StatusCode::from_u16(429).unwrap()); // 429 Too Many Requests
    }

    let tier_str = tier.as_str().to_string();
    let mut resp = next.run(req).await;
    if let Ok(v) = tier_str.parse() {
        resp.headers_mut().insert("X-HipCortex-Tier", v);
    }
    Ok(resp)
}

/// GET /tier — returns the caller's tier + limits (based on X-Api-Key header)
#[cfg(feature = "web-server")]
async fn handle_tier(headers: HeaderMap) -> Json<TierResponse> {
    let keys = load_api_keys();
    let tier = if keys.is_empty() {
        ApiTier::Team // open mode = full access
    } else {
        headers
            .get("X-Api-Key")
            .and_then(|v| v.to_str().ok())
            .and_then(|k| keys.get(k))
            .cloned()
            .unwrap_or(ApiTier::Free)
    };
    let limits = match tier {
        ApiTier::Free => TierLimits {
            records_per_month: Some(10_000),
            gdpr_endpoints: false,
            coherence_endpoints: false,
            support: "community".to_string(),
        },
        ApiTier::Pro => TierLimits {
            records_per_month: Some(1_000_000),
            gdpr_endpoints: true,
            coherence_endpoints: true,
            support: "email (48h SLA)".to_string(),
        },
        ApiTier::Team => TierLimits {
            records_per_month: None,
            gdpr_endpoints: true,
            coherence_endpoints: true,
            support: "priority (4h SLA)".to_string(),
        },
    };
    Json(TierResponse { tier: tier.as_str().to_string(), limits })
}

/// POST /memory/search — cosine similarity or keyword search over all stored records.
///
/// If `embedding` is provided, ranks by cosine similarity against records that carry
/// a `metadata.embedding` float array. Falls back to whitespace-token keyword matching
/// against `actor + action + target` for records without embeddings, or when no
/// query embedding is supplied.
#[cfg(feature = "web-server")]
async fn handle_search_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Json(req): Json<SearchMemoryRequest>,
) -> Result<Json<SearchMemoryResponse>, (StatusCode, Json<SearchMemoryResponse>)> {
    let limit = req.limit.unwrap_or(10).min(100);

    // Resolve query embedding:
    // Priority: explicit embedding > auto-generate from embedding_model > keyword-only
    let resolved_embedding: Option<Vec<f64>> = if req.embedding.is_some() {
        req.embedding.clone()
    } else if let Some(ref model_str) = req.embedding_model {
        match generate_embedding(model_str, &req.query).await {
            Ok(v) if !v.is_empty() => Some(v),
            Ok(_) => None, // empty = fall back to keyword search
            Err(_e) => return Err((
                StatusCode::BAD_GATEWAY,
                Json(SearchMemoryResponse { results: vec![], total: 0 }),
            )),
        }
    } else {
        None
    };

    let now_ts = chrono::Utc::now().timestamp();
    match store.lock() {
        Ok(ms) => {
            let results = ms.search_semantic(
                resolved_embedding.as_deref(),
                &req.query,
                limit,
                req.include_quarantined.unwrap_or(false),
            );
            let response_results = results
                .into_iter()
                .filter(|(r, _)| r.expires_at.map_or(true, |exp| exp > now_ts))
                .map(|(r, score)| SearchResult {
                    score,
                    record: MemoryRecordResponse {
                        id:          r.id.to_string(),
                        record_type: format!("{:?}", r.record_type),
                        timestamp:   r.timestamp.to_rfc3339(),
                        actor:       r.actor.clone(),
                        action:      r.action.clone(),
                        target:      r.target.clone(),
                        metadata:    r.metadata.clone(),
                        integrity:   r.integrity.clone(),
                        confidence:  r.confidence,
                        source:      r.source.clone(),
                        priority:    r.priority.clone(),
                        tags:        r.tags.clone(),
                        version:     r.version,
                        status:      r.status.clone(),
                        expires_at:  r.expires_at,
                    },
                })
                .collect::<Vec<_>>();
            let response_results = if let Some(max_tok) = req.max_tokens {
                let max_chars = max_tok * 4;
                let mut total_chars = 0usize;
                response_results.into_iter().take_while(|r| {
                    total_chars += r.record.target.len();
                    total_chars <= max_chars
                }).collect::<Vec<_>>()
            } else {
                response_results
            };
            let total = response_results.len();
            Ok(Json(SearchMemoryResponse { results: response_results, total }))
        }
        Err(_e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SearchMemoryResponse { results: vec![], total: 0 }),
        )),
    }
}

/// GET /stats — returns live server statistics (record count, breakdown, metering)
#[cfg(feature = "web-server")]
#[derive(Serialize)]
struct StatsResponse {
    total_records: usize,
    by_type: HashMap<String, usize>,
    unique_actors: usize,
    metering_enabled: bool,
    tier_counts: HashMap<String, u64>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize, Deserialize)]
pub struct AuditVerifyResponse {
    intact: bool,
    entry_count: usize,
    message: String,
}

/// GET /memory/export — export all (or actor-filtered) records as JSON array.
/// Data portability: migrate between instances, backup, or import into other systems.
#[cfg(feature = "web-server")]
async fn handle_export_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Query(params): Query<QueryMemoryParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match store.lock() {
        Ok(ms) => {
            let records = ms.all();
            let filtered: Vec<_> = records.iter().filter(|r| {
                params.actor.as_ref().map_or(true, |a| &r.actor == a)
            }).collect();
            let json_records: Vec<serde_json::Value> = filtered.iter().map(|r| {
                serde_json::json!({
                    "id":          r.id.to_string(),
                    "record_type": format!("{:?}", r.record_type),
                    "timestamp":   r.timestamp.to_rfc3339(),
                    "actor":       r.actor,
                    "action":      r.action,
                    "target":      r.target,
                    "metadata":    r.metadata,
                    "integrity":   r.integrity,
                    "confidence":  r.confidence,
                    "source":      r.source.clone(),
                    "priority":    r.priority.clone(),
                    "tags":        r.tags.clone(),
                    "version":     r.version,
                    "status":      r.status.clone(),
                    "expires_at":  r.expires_at,
                })
            }).collect();
            Ok(Json(serde_json::json!({
                "records": json_records,
                "total": json_records.len(),
                "exported_at": chrono::Utc::now().to_rfc3339(),
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
        )),
    }
}

#[cfg(feature = "web-server")]
async fn handle_stats<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
) -> Json<StatsResponse> {
    let (total_records, by_type, unique_actors) = match store.lock() {
        Ok(ms) => {
            let records = ms.all();
            let total = records.len();
            let mut by_type: HashMap<String, usize> = HashMap::new();
            let mut actors: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for r in records {
                *by_type.entry(format!("{:?}", r.record_type)).or_insert(0) += 1;
                actors.insert(&r.actor);
            }
            (total, by_type, actors.len())
        }
        Err(_) => (0, HashMap::new(), 0),
    };

    let metering_enabled = !load_api_keys().is_empty();
    let tier_counts = GLOBAL_METER.lock()
        .map(|m| m.clone())
        .unwrap_or_default();

    Json(StatsResponse { total_records, by_type, unique_actors, metering_enabled, tier_counts })
}

/// GET /pricing — static pricing page (HTML)
#[cfg(feature = "web-server")]
async fn handle_pricing() -> Html<&'static str> {
    Html(PRICING_HTML)
}

#[cfg(feature = "web-server")]
async fn handle_openapi() -> Json<serde_json::Value> {
    Json(serde_json::from_str(OPENAPI_SPEC).expect("openapi spec is valid JSON"))
}

#[cfg(feature = "web-server")]
const PRICING_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>HipCortex — Pricing</title>
<style>
  *{box-sizing:border-box;margin:0;padding:0}
  body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;background:#0d1117;color:#e6edf3;min-height:100vh;display:flex;flex-direction:column;align-items:center;padding:60px 20px}
  h1{font-size:2.4rem;font-weight:700;margin-bottom:.5rem}
  .sub{color:#8b949e;font-size:1.1rem;margin-bottom:3rem;text-align:center}
  .hero-stat{color:#58a6ff;font-weight:600}
  .grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:1.5rem;width:100%;max-width:960px}
  .card{background:#161b22;border:1px solid #30363d;border-radius:12px;padding:2rem;display:flex;flex-direction:column}
  .card.popular{border-color:#58a6ff}
  .badge{background:#58a6ff;color:#0d1117;font-size:.7rem;font-weight:700;padding:.2rem .6rem;border-radius:99px;width:fit-content;margin-bottom:1rem}
  .plan{font-size:1rem;color:#8b949e;font-weight:600;text-transform:uppercase;letter-spacing:.08em;margin-bottom:.5rem}
  .price{font-size:2.4rem;font-weight:700;margin-bottom:.25rem}
  .price span{font-size:1rem;color:#8b949e;font-weight:400}
  .desc{color:#8b949e;font-size:.9rem;margin-bottom:1.5rem}
  ul{list-style:none;flex:1;margin-bottom:1.5rem}
  li{padding:.4rem 0;font-size:.92rem;display:flex;align-items:flex-start;gap:.5rem}
  li::before{content:"✓";color:#3fb950;font-weight:700;flex-shrink:0}
  li.no::before{content:"—";color:#484f58}
  li.no{color:#484f58}
  .btn{display:block;text-align:center;padding:.85rem 1.5rem;border-radius:8px;font-weight:600;font-size:.95rem;text-decoration:none;transition:.15s}
  .btn-primary{background:#238636;color:#fff}
  .btn-primary:hover{background:#2ea043}
  .btn-outline{background:transparent;border:1px solid #30363d;color:#e6edf3}
  .btn-outline:hover{border-color:#58a6ff;color:#58a6ff}
  .footer{margin-top:3rem;color:#484f58;font-size:.85rem;text-align:center}
  .footer a{color:#58a6ff;text-decoration:none}
  .compare{margin-top:.75rem;font-size:.8rem;color:#3fb950;text-align:center}
</style>
</head>
<body>
<h1>HipCortex Memory Engine</h1>
<p class="sub">
  <span class="hero-stat">0.48ms p50 write latency</span> &nbsp;·&nbsp;
  295× faster than Mem0 cloud &nbsp;·&nbsp;
  Temporal decay · Causal world model · Merkle-chained audit log
</p>
<div class="grid">
  <!-- FREE -->
  <div class="card">
    <div class="plan">Free</div>
    <div class="price">$0 <span>/ month</span></div>
    <div class="desc">Self-hosted or managed. No credit card.</div>
    <ul>
      <li>10,000 records / month</li>
      <li>LangChain + LlamaIndex + AutoGen + CrewAI</li>
      <li>REST API + Python SDK</li>
      <li>Temporal decay &amp; keyword search</li>
      <li class="no">GDPR right-to-forget endpoint</li>
      <li class="no">Cosine similarity search</li>
      <li class="no">Coherence checking</li>
      <li class="no">SLA / support</li>
    </ul>
    <a href="https://github.com/farmountain/HipCortex" class="btn btn-outline">Get started free →</a>
    <div class="compare">pip install hipcortex</div>
  </div>
  <!-- PRO -->
  <div class="card popular">
    <div class="badge">Most popular</div>
    <div class="plan">Pro</div>
    <div class="price">$99 <span>/ month</span></div>
    <div class="desc">For teams building production AI agents.</div>
    <ul>
      <li>1,000,000 records / month</li>
      <li>Everything in Free</li>
      <li>GDPR right-to-forget (DELETE /memory/forget/:actor)</li>
      <li>Cosine similarity search (POST /memory/search)</li>
      <li>Coherence status endpoint</li>
      <li>Live stats dashboard (GET /stats)</li>
      <li>Email support (48h SLA)</li>
      <li class="no">Dedicated deployment</li>
    </ul>
    <a href="mailto:hipcortex@farmountain.dev?subject=Pro%20tier%20access&body=Hi%2C%20I%27d%20like%20Pro%20tier%20access%20for%20HipCortex." class="btn btn-primary">Join waitlist →</a>
    <div class="compare">Early access · reply for instant Pro access</div>
  </div>
  <!-- TEAM -->
  <div class="card">
    <div class="plan">Team</div>
    <div class="price">$499 <span>/ month</span></div>
    <div class="desc">Unlimited records. Priority support. EU hosting.</div>
    <ul>
      <li>Unlimited records</li>
      <li>Everything in Pro</li>
      <li>EU data residency (Frankfurt)</li>
      <li>Priority support (4h SLA)</li>
      <li>Dedicated Slack channel</li>
      <li>SOC 2 compliance docs (Q3 2026)</li>
      <li>Custom retention policies</li>
      <li>Invoice / NET-30 billing</li>
    </ul>
    <a href="mailto:hipcortex@farmountain.dev?subject=Team%20tier%20inquiry" class="btn btn-outline">Contact us →</a>
    <div class="compare">Enterprise ($15K+/yr): custom deployment + SLA</div>
  </div>
</div>
<div class="footer">
  <p>Self-hosting forever free · Apache 2.0 · <a href="/stats">Live stats</a> · <a href="https://github.com/farmountain/HipCortex">GitHub</a></p>
  <p style="margin-top:.5rem">Questions? <a href="mailto:hipcortex@farmountain.dev">hipcortex@farmountain.dev</a></p>
</div>
</body>
</html>"#;

#[cfg(feature = "web-server")]
async fn handle_bulk_add<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Json(req): Json<BulkAddRequest>,
) -> Json<BulkAddResponse> {
    let mut record_ids: Vec<String> = Vec::new();
    let mut errors: Vec<crate::memory_store::BulkAddError> = Vec::new();

    match store.lock() {
        Err(e) => Json(BulkAddResponse {
            success: false,
            inserted: 0,
            failed: req.records.len(),
            record_ids: vec![],
            errors: vec![crate::memory_store::BulkAddError {
                index: 0, actor: String::new(), reason: format!("Lock error: {}", e),
            }],
        }),
        Ok(mut ms) => {
            for (idx, r) in req.records.into_iter().enumerate() {
                let actor_name = r.actor.clone();
                let record_type = match r.record_type.as_deref() {
                    Some("Symbolic")   => MemoryType::Symbolic,
                    Some("Procedural") => MemoryType::Procedural,
                    Some("Reflexion")  => MemoryType::Reflexion,
                    Some("Perception") => MemoryType::Perception,
                    _ => MemoryType::Temporal,
                };
                let record = MemoryRecord::new(
                    record_type,
                    r.actor,
                    r.action,
                    r.target,
                    r.metadata.unwrap_or_else(|| serde_json::json!({})),
                );
                let id = record.id.to_string();
                match ms.add(record) {
                    Ok(_)  => record_ids.push(id),
                    Err(e) => errors.push(crate::memory_store::BulkAddError {
                        index: idx, actor: actor_name, reason: e.to_string(),
                    }),
                }
            }
            Json(BulkAddResponse {
                success: errors.is_empty(),
                inserted: record_ids.len(),
                failed: errors.len(),
                record_ids,
                errors,
            })
        }
    }
}

/// Generate an embedding vector by calling Ollama or OpenAI.
/// model_str format: "ollama/<model>" or "openai/<model>"
/// Returns empty Vec on empty response (caller should fall back to keyword search).
#[cfg(feature = "web-server")]
async fn generate_embedding(model_str: &str, text: &str) -> Result<Vec<f64>, String> {
    if model_str.starts_with("ollama/") {
        let model = &model_str["ollama/".len()..];
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        let body = serde_json::json!({ "model": model, "prompt": text });
        let resp = client
            .post(format!("{}/api/embeddings", ollama_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        Ok(data["embedding"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default())
    } else if model_str.starts_with("openai/") {
        let model = &model_str["openai/".len()..];
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        let body = serde_json::json!({ "model": model, "input": text });
        let resp = client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {}", e))?;
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        Ok(data["data"][0]["embedding"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default())
    } else {
        Err(format!(
            "embedding_model must start with 'ollama/' or 'openai/', got: {}",
            model_str
        ))
    }
}

#[cfg(feature = "web-server")]
async fn handle_embed_and_add<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Json(req): Json<EmbedAndAddRequest>,
) -> Result<Json<AddMemoryResponse>, (StatusCode, Json<AddMemoryResponse>)> {
    let embedding = match generate_embedding(&req.embedding_model, &req.target).await {
        Ok(v) => v,
        Err(e) => return Err((
            StatusCode::BAD_GATEWAY,
            Json(AddMemoryResponse { success: false, record_id: None, error: Some(e), warning: None }),
        )),
    };

    let record_type = match req.record_type.as_deref() {
        Some("Symbolic")   => MemoryType::Symbolic,
        Some("Procedural") => MemoryType::Procedural,
        Some("Reflexion")  => MemoryType::Reflexion,
        Some("Perception") => MemoryType::Perception,
        _ => MemoryType::Temporal,
    };
    let mut metadata = req.metadata.unwrap_or_else(|| serde_json::json!({}));
    if let serde_json::Value::Object(ref mut map) = metadata {
        map.insert("embedding".to_string(), serde_json::json!(embedding));
    }
    let record = MemoryRecord::new(record_type, req.actor, req.action, req.target, metadata);

    match store.lock() {
        Ok(mut ms) => match ms.add(record.clone()) {
            Ok(_) => Ok(Json(AddMemoryResponse {
                success: true, record_id: Some(record.id.to_string()), error: None, warning: None,
            })),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR,
                Json(AddMemoryResponse { success: false, record_id: None, error: Some(e.to_string()), warning: None }))),
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json(AddMemoryResponse { success: false, record_id: None,
                error: Some(format!("Lock error: {}", e)), warning: None }))),
    }
}

/// GET /memory/search-flat?query=&actor=&limit=
/// Returns plain string array — for no-code tools (Flowise, Dify, n8n, Make.com)
#[cfg(feature = "web-server")]
async fn handle_search_flat<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Query(params): Query<SearchFlatParams>,
) -> Json<SearchFlatResponse> {
    let query = params.query.unwrap_or_default();
    let limit = params.limit.unwrap_or(10).min(50);
    let now_ts = chrono::Utc::now().timestamp();

    match store.lock() {
        Ok(ms) => {
            let results = ms.search_semantic(None, &query, limit, false);
            let memories: Vec<String> = results
                .into_iter()
                .filter(|(r, _)| {
                    r.expires_at.map_or(true, |exp| exp > now_ts) &&
                    params.actor.as_ref().map_or(true, |a| &r.actor == a)
                })
                .map(|(r, _)| format!("[{}] {}", r.action, r.target))
                .collect();
            let total = memories.len();
            Json(SearchFlatResponse { memories, total })
        }
        Err(_) => Json(SearchFlatResponse { memories: vec![], total: 0 }),
    }
}

#[cfg(feature = "web-server")]
async fn handle_update_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    id_str: String,
    Json(req): Json<UpdateMemoryRequest>,
) -> Result<Json<UpdateMemoryResponse>, (StatusCode, Json<UpdateMemoryResponse>)> {
    let id = match uuid::Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return Err((
            StatusCode::BAD_REQUEST,
            Json(UpdateMemoryResponse {
                success: false, record_id: id_str,
                version: 0, error: Some("invalid UUID".to_string()),
            }),
        )),
    };

    match store.lock() {
        Ok(mut ms) => match ms.update_record(
            id,
            req.target.as_deref(),
            req.action.as_deref(),
            req.confidence,
            req.source.as_deref(),
            req.metadata,
        ) {
            Ok(_) => {
                let version = ms.find_by_id(id).map(|r| r.version).unwrap_or(0);
                Ok(Json(UpdateMemoryResponse {
                    success: true,
                    record_id: id.to_string(),
                    version,
                    error: None,
                }))
            }
            Err(e) => Err((
                StatusCode::NOT_FOUND,
                Json(UpdateMemoryResponse {
                    success: false, record_id: id.to_string(),
                    version: 0, error: Some(e.to_string()),
                }),
            )),
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(UpdateMemoryResponse {
                success: false, record_id: id_str,
                version: 0, error: Some(format!("Lock error: {}", e)),
            }),
        )),
    }
}

/// GET /memory/latest?actor=&action=&limit=
/// Returns the most recent unique fact per (actor, action) pair.
/// Solves: "what is the current value of X?" query pattern.
#[cfg(feature = "web-server")]
async fn handle_latest_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Query(params): Query<LatestMemoryParams>,
) -> Result<Json<QueryMemoryResponse>, (StatusCode, Json<QueryMemoryResponse>)> {
    let limit = params.limit.unwrap_or(20).min(100);
    match store.lock() {
        Ok(ms) => {
            let records = ms.find_latest(
                params.actor.as_deref(),
                params.action.as_deref(),
                limit,
            );
            let response_records = records.into_iter().map(|r| MemoryRecordResponse {
                id:          r.id.to_string(),
                record_type: format!("{:?}", r.record_type),
                timestamp:   r.timestamp.to_rfc3339(),
                actor:       r.actor.clone(),
                action:      r.action.clone(),
                target:      r.target.clone(),
                metadata:    r.metadata.clone(),
                integrity:   r.integrity.clone(),
                confidence:  r.confidence,
                source:      r.source.clone(),
                priority:    r.priority.clone(),
                tags:        r.tags.clone(),
                version:     r.version,
                status:      r.status.clone(),
                expires_at:  r.expires_at,
            }).collect::<Vec<_>>();
            let total = response_records.len();
            Ok(Json(QueryMemoryResponse { records: response_records, total }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(QueryMemoryResponse { records: vec![], total: 0 }),
        )),
    }
}

/// GET /audit/verify — check Merkle chain integrity
#[cfg(feature = "web-server")]
async fn handle_audit_verify<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
) -> Json<AuditVerifyResponse> {
    match store.lock() {
        Ok(ms) => {
            match ms.audit_verify() {
                Ok((intact, count)) => Json(AuditVerifyResponse {
                    intact,
                    entry_count: count,
                    message: if intact {
                        format!("Audit log intact — {} entries verified", count)
                    } else {
                        "TAMPER DETECTED — Merkle chain broken".to_string()
                    },
                }),
                Err(e) => Json(AuditVerifyResponse {
                    intact: false,
                    entry_count: 0,
                    message: format!("Verification error: {}", e),
                }),
            }
        }
        Err(e) => Json(AuditVerifyResponse {
            intact: false, entry_count: 0,
            message: format!("Lock error: {}", e),
        }),
    }
}

/// GET /audit/export — download full audit log as JSON array
#[cfg(feature = "web-server")]
async fn handle_audit_export<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match store.lock() {
        Ok(ms) => match ms.audit_export() {
            Ok(entries) => {
                let total = entries.len();
                Ok(Json(serde_json::json!({
                    "entries": entries,
                    "total": total,
                    "exported_at": chrono::Utc::now().to_rfc3339(),
                })))
            },
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )),
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
        )),
    }
}

#[cfg(feature = "web-server")]
async fn handle_create_node(
    store: Arc<Mutex<crate::symbolic_store::SymbolicStore<crate::symbolic_store::InMemoryGraph>>>,
    Json(req): Json<CreateNodeRequest>,
) -> Json<GraphWriteResponse> {
    match store.lock() {
        Ok(mut ss) => {
            let props = req.properties.unwrap_or_default();
            let id = ss.add_node(&req.label, props);
            Json(GraphWriteResponse { success: true, id: Some(id.to_string()), error: None })
        }
        Err(e) => Json(GraphWriteResponse {
            success: false, id: None,
            error: Some(format!("Lock error: {}", e)),
        }),
    }
}

#[cfg(feature = "web-server")]
async fn handle_create_edge(
    store: Arc<Mutex<crate::symbolic_store::SymbolicStore<crate::symbolic_store::InMemoryGraph>>>,
    Json(req): Json<CreateEdgeRequest>,
) -> Json<GraphWriteResponse> {
    let from = match uuid::Uuid::parse_str(&req.from_id) {
        Ok(u) => u,
        Err(_) => return Json(GraphWriteResponse {
            success: false, id: None, error: Some("invalid from_id UUID".to_string()),
        }),
    };
    let to = match uuid::Uuid::parse_str(&req.to_id) {
        Ok(u) => u,
        Err(_) => return Json(GraphWriteResponse {
            success: false, id: None, error: Some("invalid to_id UUID".to_string()),
        }),
    };
    match store.lock() {
        Ok(mut ss) => {
            ss.add_edge(from, to, &req.relation);
            Json(GraphWriteResponse { success: true, id: None, error: None })
        }
        Err(e) => Json(GraphWriteResponse {
            success: false, id: None, error: Some(format!("Lock error: {}", e)),
        }),
    }
}

#[cfg(feature = "web-server")]
async fn handle_delete_node(
    store: Arc<Mutex<crate::symbolic_store::SymbolicStore<crate::symbolic_store::InMemoryGraph>>>,
    id: String,
) -> Json<GraphWriteResponse> {
    let node_id = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Json(GraphWriteResponse {
            success: false, id: None, error: Some("invalid UUID".to_string()),
        }),
    };
    match store.lock() {
        Ok(mut ss) => {
            let removed = ss.remove_node(node_id);
            Json(GraphWriteResponse {
                success: removed,
                id: if removed { Some(id) } else { None },
                error: if removed { None } else { Some("node not found".to_string()) },
            })
        }
        Err(e) => Json(GraphWriteResponse {
            success: false, id: None, error: Some(format!("Lock error: {}", e)),
        }),
    }
}

/// POST /memory/consolidate — merge near-duplicate records for an actor.
/// Keyword-similarity based dedup. ML-based dedup available in managed tier.
#[cfg(feature = "web-server")]
async fn handle_consolidate<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Query(params): Query<ConsolidateParams>,
) -> Json<serde_json::Value> {
    let threshold = params.threshold.unwrap_or(0.80).clamp(0.0, 1.0);
    let dry_run = params.dry_run.unwrap_or(false);

    match store.lock() {
        Ok(ms) => {
            let records = ms.all();
            let candidates: Vec<_> = records.iter()
                .filter(|r| params.actor.as_ref().map_or(true, |a| &r.actor == a))
                .collect();

            // Find pairs with keyword similarity above threshold
            let mut merge_groups: Vec<(String, String, f64)> = Vec::new(); // (keep_id, drop_id, similarity)
            for i in 0..candidates.len() {
                for j in (i + 1)..candidates.len() {
                    let words_i: std::collections::HashSet<&str> = candidates[i].target.split_whitespace().collect();
                    let words_j: std::collections::HashSet<&str> = candidates[j].target.split_whitespace().collect();
                    if words_i.is_empty() || words_j.is_empty() { continue; }
                    let intersection = words_i.intersection(&words_j).count();
                    let sim = intersection as f64 / words_i.len().max(words_j.len()) as f64;
                    if sim >= threshold {
                        // Keep the newer one (higher confidence wins)
                        let (keep, drop) = if candidates[i].timestamp >= candidates[j].timestamp {
                            (candidates[i].id.to_string(), candidates[j].id.to_string())
                        } else {
                            (candidates[j].id.to_string(), candidates[i].id.to_string())
                        };
                        merge_groups.push((keep, drop, sim));
                    }
                }
            }

            Json(serde_json::json!({
                "found_duplicates": merge_groups.len(),
                "dry_run": dry_run,
                "pairs": merge_groups.iter().map(|(k, d, s)| serde_json::json!({
                    "keep": k, "drop": d,
                    "similarity": (s * 100.0) as u32
                })).collect::<Vec<_>>(),
                "note": if dry_run {
                    "Dry run — no changes made. Re-run without dry_run=true to consolidate."
                } else {
                    "ML-based consolidation available in managed tier. Keyword consolidation: use GDPR forget on 'drop' IDs listed above."
                }
            }))
        }
        Err(e) => Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
    }
}

#[cfg(feature = "web-server")]
async fn handle_prometheus_metrics<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
) -> axum::response::Response<String> {
    let (total, by_type, actors, metered) = match store.lock() {
        Ok(ms) => {
            let records = ms.all();
            let total = records.len();
            let mut by_type: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            let mut actors: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for r in records { *by_type.entry(format!("{:?}", r.record_type)).or_insert(0) += 1; actors.insert(&r.actor); }
            (total, by_type, actors.len(), !load_api_keys().is_empty())
        }
        Err(_) => (0, std::collections::HashMap::new(), 0, false),
    };

    let mut lines = vec![
        "# HELP hipcortex_records_total Total memory records".to_string(),
        "# TYPE hipcortex_records_total gauge".to_string(),
        format!("hipcortex_records_total {}", total),
        "# HELP hipcortex_actors_total Unique actors".to_string(),
        "# TYPE hipcortex_actors_total gauge".to_string(),
        format!("hipcortex_actors_total {}", actors),
        "# HELP hipcortex_metering_enabled API key metering active".to_string(),
        "# TYPE hipcortex_metering_enabled gauge".to_string(),
        format!("hipcortex_metering_enabled {}", if metered { 1 } else { 0 }),
    ];
    for (t, count) in &by_type {
        lines.push(format!("hipcortex_records_by_type{{type=\"{}\"}} {}", t, count));
    }
    lines.push(String::new());

    axum::response::Response::builder()
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(lines.join("\n"))
        .unwrap()
}

#[cfg(feature = "web-server")]
pub async fn run_with_both_stores<B: MemoryBackend + Send + Sync + 'static>(
    addr: SocketAddr,
    symbolic_store: Arc<Mutex<SymbolicStore<InMemoryGraph>>>,
    memory_store: Arc<Mutex<MemoryStore<B>>>,
) {
    // Backward-compat: no AppState available here, use a no-op world model
    let world_model: Arc<RwLock<WorldModelEnhanced>> =
        Arc::new(RwLock::new(WorldModelEnhanced::new()));

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
        let wm = world_model.clone();
        post(move |Json(req): Json<AddMemoryRequest>| async move {
            handle_add_memory(store, wm, req).await
        })
    };

    let bulk_add_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<BulkAddRequest>| async move {
            handle_bulk_add(store, Json(req)).await
        })
    };

    let query_memory_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<QueryMemoryParams>| async move {
            handle_query_memory(store, params).await
        })
    };

    // Semantic / keyword search: POST /memory/search
    let search_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<SearchMemoryRequest>| async move {
            handle_search_memory(store, Json(req)).await
        })
    };

    // GDPR forget: DELETE /memory/forget/:actor
    let forget_route = {
        let ms = memory_store.clone();
        let ss = symbolic_store.clone();
        delete(move |Path(actor): Path<String>| async move {
            handle_forget_actor(ms, ss, actor).await
        })
    };

    // Embed and add: POST /memory/embed
    let embed_add_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<EmbedAndAddRequest>| async move {
            handle_embed_and_add(store, Json(req)).await
        })
    };

    // Coherence status: GET /coherence/status (backward-compat: fresh checker per request)
    let coherence_route = {
        let c = Arc::new(CoherenceChecker::new());
        get(move || { let cc = c.clone(); async move { handle_coherence_status(cc).await } })
    };

    // Live stats: GET /stats
    let stats_route = {
        let store = memory_store.clone();
        get(move || handle_stats(store))
    };

    // Data export: GET /memory/export?actor=optional
    let export_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<QueryMemoryParams>| async move {
            handle_export_memory(store, Query(params)).await
        })
    };

    // Flat search: GET /memory/search-flat?query=&actor=&limit=
    let search_flat_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<SearchFlatParams>| async move {
            handle_search_flat(store, Query(params)).await
        })
    };

    // PATCH /memory/update/:id — versioned in-place update
    let update_route = {
        let store = memory_store.clone();
        patch(move |Path(id): Path<String>, Json(req): Json<UpdateMemoryRequest>| async move {
            handle_update_memory(store, id, Json(req)).await
        })
    };

    // GET /memory/latest — most recent unique fact per (actor, action)
    let latest_route = {
        let store = memory_store.clone();
        get(move |Query(params): Query<LatestMemoryParams>| async move {
            handle_latest_memory(store, Query(params)).await
        })
    };

    let audit_verify_route = {
        let store = memory_store.clone();
        get(move || { let s = store.clone(); async move { handle_audit_verify(s).await } })
    };
    let audit_export_route = {
        let store = memory_store.clone();
        get(move || { let s = store.clone(); async move { handle_audit_export(s).await } })
    };

    let create_node_route = {
        let ss = symbolic_store.clone();
        post(move |Json(req): Json<CreateNodeRequest>| async move {
            handle_create_node(ss, Json(req)).await
        })
    };
    let create_edge_route = {
        let ss = symbolic_store.clone();
        post(move |Json(req): Json<CreateEdgeRequest>| async move {
            handle_create_edge(ss, Json(req)).await
        })
    };
    let delete_node_route = {
        let ss = symbolic_store.clone();
        delete(move |Path(id): Path<String>| async move {
            handle_delete_node(ss, id).await
        })
    };
    let consolidate_route = {
        let store = memory_store.clone();
        post(move |Query(params): Query<ConsolidateParams>| async move {
            handle_consolidate(store, Query(params)).await
        })
    };

    let ingest_route = {
        let store = memory_store.clone();
        let wm = world_model.clone();
        post(move |Json(req): Json<IngestRequest>| async move {
            handle_ingest(store, wm, Json(req)).await
        })
    };

    let quarantine_route = {
        let store = memory_store.clone();
        post(move |Path(id): Path<String>| { let s = store.clone(); async move { handle_quarantine_memory(s, id).await } })
    };
    let restore_route = {
        let store = memory_store.clone();
        post(move |Path(id): Path<String>| { let s = store.clone(); async move { handle_restore_memory(s, id).await } })
    };
    let corroborate_route = {
        let store = memory_store.clone();
        post(move |Path(id): Path<String>| { let s = store.clone(); async move { handle_corroborate(s, id).await } })
    };
    let contradict_route = {
        let store = memory_store.clone();
        post(move |Path(id): Path<String>| { let s = store.clone(); async move { handle_contradict(s, id).await } })
    };
    let context_route = {
        let store = memory_store.clone();
        post(move |Json(req): Json<ContextRequest>| async move {
            handle_memory_context(store, Json(req)).await
        })
    };

    let metrics_route = {
        let store = memory_store.clone();
        get(move || { let s = store.clone(); async move { handle_prometheus_metrics(s).await } })
    };

    let app = Router::new()
        .route("/", get(|| async { axum::response::Redirect::permanent("/pricing") }))
        .route("/health", get(|| async { "ok" }))
        .route("/graph", graph_route)
        .route("/node/:id", node_route)
        .route("/memory/add", add_memory_route)
        .route("/memory/bulk", bulk_add_route)
        .route("/memory/embed", embed_add_route)
        .route("/memory/query", query_memory_route)
        .route("/memory/search", search_route)
        .route("/memory/export", export_route)
        .route("/memory/forget/:actor", forget_route)
        .route("/memory/search-flat", search_flat_route)
        .route("/memory/update/:id", update_route)
        .route("/memory/latest", latest_route)
        .route("/audit/verify", audit_verify_route)
        .route("/audit/export", audit_export_route)
        .route("/coherence/status", coherence_route)
        .route("/coherence/inconsistencies", get(handle_coherence_inconsistencies_fresh))
        .route("/worldmodel/status", {
            let wm = world_model.clone();
            get(move || { let w = wm.clone(); async move { handle_worldmodel_status(w).await } })
        })
        .route("/webhooks", get(handle_list_webhooks).post(handle_register_webhook))
        .route("/webhooks/:id", delete(handle_delete_webhook))
        .route("/graph/node", create_node_route)
        .route("/graph/edge", create_edge_route)
        .route("/graph/node/:id", delete_node_route)
        .route("/memory/consolidate", consolidate_route)
        .route("/memory/ingest", ingest_route)
        .route("/memory/quarantine/:id", quarantine_route)
        .route("/memory/restore/:id", restore_route)
        .route("/memory/corroborate/:id", corroborate_route)
        .route("/memory/contradict/:id", contradict_route)
        .route("/memory/context", context_route)
        .route("/metrics", metrics_route)
        .route("/stats", stats_route)
        .route("/tier", get(handle_tier))
        .route("/pricing", get(handle_pricing))
        .route("/openapi.json", get(handle_openapi))
        .route("/ns", get(handle_list_namespaces))
        .route("/regulatory/hold", get(handle_list_regulatory_holds).post(handle_set_regulatory_hold))
        .route("/regulatory/hold/:actor", delete(handle_release_regulatory_hold))
        .layer(middleware::from_fn(api_key_middleware));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
}

/// Heuristic memory classification — no ML required.
/// Returns (record_type, priority, ttl_seconds, confidence, actor, action, tags)
#[cfg(feature = "web-server")]
fn classify_ingest(text: &str, actor_hint: Option<&str>, context_hint: Option<&str>) -> (String, String, Option<u64>, f32, String, String, Vec<String>) {
    let lower = text.to_lowercase();

    // ── record_type detection ─────────────────────────────────────────
    let record_type = if context_hint == Some("code") || contains_code_pattern(text) {
        "Procedural"
    } else if contains_any(&lower, &["said", "told me", "asked me", "replied", "mentioned", "responded", "user:", "assistant:", "human:"]) {
        "Reflexion"
    } else if contains_any(&lower, &["decided", "chose", "choosing", "selected", "going with", "will use", "adopted", "switched to", "we use", "use ", "using "]) {
        "Symbolic"
    } else {
        "Temporal"
    };

    // ── priority detection ────────────────────────────────────────────
    let priority = if contains_any(&lower, &["never ", "always ", "must ", "must not", "required", "constraint", "rule:", "policy:", "forbidden", "critical", "allerg"]) {
        "pinned"
    } else if contains_any(&lower, &["decided", "chose", "selected", "confirmed", "approved", "deployed", "released", "launch"]) {
        "high"
    } else if contains_any(&lower, &["maybe", "might ", "could ", "possibly", "not sure", "uncertain", "think ", "believe "]) {
        "low"
    } else {
        "normal"
    };

    // ── TTL detection (working vs long-term memory) ───────────────────
    let ttl = if context_hint == Some("sensor") {
        Some(3600u64) // sensor data: 1 hour
    } else if priority == "pinned" || record_type == "Symbolic" {
        None // decisions + constraints: permanent
    } else if contains_any(&lower, &["today", "tomorrow", "this morning", "tonight", "right now", "currently", "at the moment"]) {
        Some(86400u64) // today-specific: 24 hours
    } else if contains_any(&lower, &["this week", "by friday", "by monday", "by wednesday", "by thursday", "by tuesday", "next week"]) {
        Some(604800u64) // week-specific: 7 days
    } else if record_type == "Reflexion" {
        Some(86400u64) // conversation: 24 hours
    } else if record_type == "Temporal" {
        Some(86400u64) // temporal default: 24 hours
    } else {
        None
    };

    // ── confidence detection ──────────────────────────────────────────
    let confidence = if contains_any(&lower, &["maybe", "might", "possibly", "not sure", "uncertain", "think", "believe"]) {
        0.6f32
    } else if contains_any(&lower, &["confirmed", "verified", "deployed", "released", "decided", "chose"]) {
        0.95f32
    } else {
        0.85f32
    };

    // ── actor extraction ──────────────────────────────────────────────
    let actor = if let Some(a) = actor_hint {
        a.to_string()
    } else {
        extract_actor(text)
    };

    // ── action extraction ─────────────────────────────────────────────
    let action = extract_action(&lower);

    // ── tag extraction ────────────────────────────────────────────────
    let tags = extract_tags(&lower, context_hint);

    (record_type.to_string(), priority.to_string(), ttl, confidence, actor, action, tags)
}

#[cfg(feature = "web-server")]
fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

#[cfg(feature = "web-server")]
fn contains_code_pattern(text: &str) -> bool {
    text.contains("def ") || text.contains("fn ") || text.contains("class ") ||
    text.contains("function ") || text.contains("import ") || text.contains("return ") ||
    text.contains("```") || text.contains("//") || (text.contains("{") && text.contains("}"))
}

#[cfg(feature = "web-server")]
fn extract_actor(text: &str) -> String {
    // Pattern: First capitalized word followed by action verb = actor
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() >= 2 {
        let first = words[0];
        if first.len() > 1 && first.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            let action_verbs = ["decided", "chose", "said", "told", "asked", "replied",
                                "found", "created", "built", "deployed", "fixed", "updated",
                                "switched", "selected", "approved", "confirmed", "mentioned"];
            let second_lower = words[1].to_lowercase();
            if action_verbs.iter().any(|v| second_lower.starts_with(v)) {
                return first.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
            }
        }
    }
    "default".to_string()
}

#[cfg(feature = "web-server")]
fn extract_action(lower: &str) -> String {
    let action_map = [
        ("decided", "decided"), ("chose", "decided"), ("selected", "decided"),
        ("said", "said"), ("told", "said"), ("mentioned", "said"), ("replied", "said"),
        ("fixed", "fixed"), ("resolved", "fixed"), ("deployed", "deployed"),
        ("created", "created"), ("built", "created"), ("implemented", "created"),
        ("found", "observed"), ("noticed", "observed"), ("detected", "observed"),
        ("constraint", "constraint"), ("must ", "constraint"), ("never ", "constraint"),
        ("updated", "updated"), ("changed", "updated"), ("switched", "updated"),
    ];
    for (pattern, action) in &action_map {
        if lower.contains(pattern) {
            return action.to_string();
        }
    }
    "noted".to_string()
}

#[cfg(feature = "web-server")]
fn extract_tags(lower: &str, context: Option<&str>) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(ctx) = context { tags.push(ctx.to_string()); }

    let tag_map: &[(&[&str], &str)] = &[
        (&["postgresql", "mysql", "sqlite", "mongodb", "redis", "database", "db "], "database"),
        (&["react", "vue", "angular", "svelte", "frontend", "css", "html"], "frontend"),
        (&["fastapi", "django", "flask", "express", "backend", "api", "endpoint"], "backend"),
        (&["auth", "jwt", "oauth", "login", "password", "token", "session"], "auth"),
        (&["bug", "error", "crash", "exception", "fix", "broken", "failed"], "bug"),
        (&["deploy", "deployment", "kubernetes", "docker", "k8s", "ci/cd"], "infrastructure"),
        (&["test", "testing", "pytest", "jest", "spec", "coverage"], "testing"),
        (&["security", "vulnerability", "cve", "xss", "injection"], "security"),
        (&["performance", "latency", "slow", "optimize", "cache"], "performance"),
        (&["architecture", "design", "structure", "pattern"], "architecture"),
        (&["meeting", "standup", "sprint", "planning"], "meeting"),
    ];

    for (patterns, tag) in tag_map {
        if patterns.iter().any(|p| lower.contains(p)) {
            tags.push(tag.to_string());
        }
    }
    tags.dedup();
    tags
}

/// POST /memory/ingest — zero-config smart memory ingest.
/// Classifies plain text automatically: record_type, priority, TTL, tags, actor.
/// Target UX: client.remember("text") with no memory architecture required.
#[cfg(feature = "web-server")]
async fn handle_ingest<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<IngestResponse>)> {
    let (record_type_str, priority, ttl, confidence, actor, action, tags) =
        classify_ingest(&req.text, req.actor.as_deref(), req.context.as_deref());

    let record_type = match record_type_str.as_str() {
        "Symbolic"   => MemoryType::Symbolic,
        "Procedural" => MemoryType::Procedural,
        "Reflexion"  => MemoryType::Reflexion,
        _            => MemoryType::Temporal,
    };

    let mut metadata = serde_json::json!({});
    if let Some(sid) = &req.session_id {
        if let serde_json::Value::Object(ref mut m) = metadata {
            m.insert("session_id".to_string(), serde_json::json!(sid));
        }
    }

    let mut record = MemoryRecord::new(
        record_type,
        actor.clone(),
        action.clone(),
        req.text.clone(),
        metadata,
    );
    record.confidence = confidence;
    record.source = Some("auto-ingest".to_string());
    record.priority = priority.clone();
    record.tags = tags.clone();
    if let Some(ttl_secs) = ttl {
        record.expires_at = Some(
            (chrono::Utc::now().timestamp()) + ttl_secs as i64
        );
    }

    // Check contradiction
    let warning = store.try_lock().ok().and_then(|ms| {
        let existing = ms.find_by_actor(&actor);
        let new_words: std::collections::HashSet<&str> = req.text.split_whitespace().collect();
        let conflicts: Vec<serde_json::Value> = existing.iter().take(30)
            .filter_map(|r| {
                let old_words: std::collections::HashSet<&str> = r.target.split_whitespace().collect();
                if new_words.is_empty() || old_words.is_empty() || r.target == req.text { return None; }
                let overlap = new_words.intersection(&old_words).count();
                let ratio = overlap as f64 / new_words.len().max(old_words.len()) as f64;
                if ratio > 0.5 {
                    Some(serde_json::json!({"id": r.id.to_string(), "target": r.target, "overlap": (ratio*100.0) as u32}))
                } else { None }
            })
            .take(2)
            .collect();
        if conflicts.is_empty() { None } else { Some(serde_json::json!(conflicts)) }
    });

    let working_memory = ttl.is_some() && ttl.unwrap_or(0) <= 86400;
    let record_id = record.id.to_string();
    let rtype = format!("{:?}", record.record_type);

    match store.lock() {
        Ok(mut ms) => match ms.add(record.clone()) {
            Ok(_) => {
                // Auto-feed WorldModelEnhanced — non-blocking, best-effort
                if let Ok(mut wm) = world_model.try_write() {
                    let _ = wm.observe_transition(
                        record.actor.clone(),
                        record.action.clone(),
                        record.target.clone(),
                    );
                    if record.priority == "pinned" && record.record_type == MemoryType::Symbolic {
                        let _ = wm.add_causal_edge(record.actor.clone(), record.target.clone());
                    }
                }
                Ok(Json(IngestResponse {
                    record_id,
                    record_type: rtype,
                    priority,
                    tags,
                    ttl_seconds: ttl,
                    confidence,
                    actor,
                    action,
                    target: req.text,
                    working_memory,
                    warning,
                }))
            },
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(IngestResponse {
                record_id: String::new(), record_type: String::new(), priority: String::new(),
                tags: vec![], ttl_seconds: None, confidence: 0.0, actor: String::new(),
                action: String::new(), target: req.text,
                working_memory: false,
                warning: Some(serde_json::json!({"error": e.to_string()})),
            }))),
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(IngestResponse {
            record_id: String::new(), record_type: String::new(), priority: String::new(),
            tags: vec![], ttl_seconds: None, confidence: 0.0, actor: String::new(),
            action: String::new(), target: req.text,
            working_memory: false,
            warning: Some(serde_json::json!({"error": format!("Lock error: {}", e)})),
        }))),
    }
}

#[cfg(feature = "web-server")]
async fn handle_add_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    world_model: Arc<RwLock<WorldModelEnhanced>>,
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

    let actor_name = req.actor.clone();
    let mut record = MemoryRecord::new(
        record_type,
        req.actor,
        req.action,
        req.target,
        req.metadata.unwrap_or_else(|| serde_json::json!({})),
    );
    if let Some(ttl) = req.ttl_seconds {
        record.expires_at = Some(
            (chrono::Utc::now().timestamp()) + ttl as i64
        );
    }
    // Set confidence and source if provided
    if let Some(conf) = req.confidence {
        record.confidence = conf.clamp(0.0, 1.0);
    }
    if let Some(src) = req.source {
        record.source = Some(src);
    }
    if !req.tags.is_empty() {
        record.tags = req.tags;
    }
    if req.priority != "normal" {
        record.priority = req.priority;
    }

    // P0.3 — Store decay config in metadata for TemporalIndexer pickup
    if req.decay_factor.is_some() || req.decay_half_life_secs.is_some() {
        if let serde_json::Value::Object(ref mut map) = record.metadata {
            if let Some(df) = req.decay_factor {
                map.insert("decay_factor".to_string(), serde_json::json!(df.clamp(0.0, 2.0)));
            }
            if let Some(hl) = req.decay_half_life_secs {
                map.insert("decay_half_life_secs".to_string(), serde_json::json!(hl));
            }
        }
    }

    match store.lock() {
        Ok(mut ms) => {
            // Actor memory quota check (env var HIPCORTEX_ACTOR_MAX_RECORDS, default: unlimited)
            // Done inside the same lock to avoid TOCTOU and double-lock issues.
            if let Ok(max_str) = std::env::var("HIPCORTEX_ACTOR_MAX_RECORDS") {
                if let Ok(max) = max_str.parse::<usize>() {
                    let actor_count = ms.find_by_actor(&actor_name).len();
                    if actor_count >= max {
                        return Err((
                            StatusCode::from_u16(429).unwrap(),
                            Json(AddMemoryResponse {
                                success: false,
                                record_id: None,
                                error: Some(format!(
                                    "Actor '{}' has reached max {} records. Use GDPR forget to clear.",
                                    actor_name, max
                                )),
                                warning: None,
                            }),
                        ));
                    }
                }
            }

            // P0.2 — Sync contradiction check: keyword overlap with existing same-actor records
            let contradiction_warning: Option<serde_json::Value> = {
                let existing = ms.find_by_actor(&record.actor);
                let new_words: std::collections::HashSet<&str> = record.target
                    .split_whitespace().collect();
                let mut conflicts: Vec<serde_json::Value> = Vec::new();
                for existing_rec in existing.iter().take(50) {
                    if existing_rec.id == record.id { continue; }
                    let old_words: std::collections::HashSet<&str> = existing_rec.target
                        .split_whitespace().collect();
                    if new_words.is_empty() || old_words.is_empty() { continue; }
                    let overlap = new_words.intersection(&old_words).count();
                    let overlap_ratio = overlap as f64 / new_words.len().max(old_words.len()) as f64;
                    // High overlap + different content = possible contradiction
                    if overlap_ratio > 0.5 && existing_rec.target != record.target {
                        conflicts.push(serde_json::json!({
                            "id": existing_rec.id.to_string(),
                            "action": existing_rec.action,
                            "target": existing_rec.target,
                            "overlap_ratio": (overlap_ratio * 100.0) as u32
                        }));
                        if conflicts.len() >= 3 { break; }
                    }
                }
                if conflicts.is_empty() { None } else { Some(serde_json::json!(conflicts)) }
            };

            match ms.add(record.clone()) {
                Ok(_) => {
                    // Auto-feed WorldModelEnhanced — non-blocking (try_write), best-effort
                    // A failed lock acquisition simply skips the feed without blocking the request
                    if let Ok(mut wm) = world_model.try_write() {
                        let _ = wm.observe_transition(
                            record.actor.clone(),
                            record.action.clone(),
                            record.target.clone(),
                        );
                        // Register causal edge for pinned symbolic decisions
                        if record.priority == "pinned" && record.record_type == MemoryType::Symbolic {
                            let _ = wm.add_causal_edge(record.actor.clone(), record.target.clone());
                        }
                    }
                    // P0.4 — fire webhook (best-effort, non-blocking)
                    fire_webhook("memory.added", serde_json::json!({
                        "id": record.id.to_string(),
                        "actor": record.actor,
                        "action": record.action,
                        "target": record.target,
                    }));
                    Ok(Json(AddMemoryResponse {
                        success: true,
                        record_id: Some(record.id.to_string()),
                        error: None,
                        warning: contradiction_warning,
                    }))
                },
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AddMemoryResponse {
                        success: false,
                        record_id: None,
                        error: Some(e.to_string()),
                        warning: None,
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
                warning: None,
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
            // Multi-actor filter takes precedence over single actor
            if let Some(actors_str) = &params.actors {
                let actor_list: Vec<&str> = actors_str.split(',').map(|a| a.trim()).collect();
                filtered_records.retain(|r| actor_list.contains(&r.actor.as_str()));
            } else if let Some(actor) = &params.actor {
                filtered_records.retain(|r| r.actor == *actor);
            }
            // Exclude quarantined unless explicitly requested
            let include_quarantined = params.include_quarantined.as_deref() == Some("true");
            if !include_quarantined {
                filtered_records.retain(|r| r.status != "quarantine");
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

            // Exclude records past their TTL
            let now_ts = chrono::Utc::now().timestamp();
            filtered_records.retain(|r| {
                r.expires_at.map_or(true, |exp| exp > now_ts)
            });

            // Filter by tags (any match)
            if let Some(tags_str) = &params.tags {
                let tag_list: Vec<&str> = tags_str.split(',').map(|t| t.trim()).collect();
                filtered_records.retain(|r| tag_list.iter().any(|t| r.tags.contains(&t.to_string())));
            }
            // Filter by priority
            if let Some(priority) = &params.priority {
                filtered_records.retain(|r| &r.priority == priority);
            }
            // Filter by as_of timestamp (time-travel query)
            if let Some(as_of_str) = &params.as_of {
                if let Ok(as_of_ts) = chrono::DateTime::parse_from_rfc3339(as_of_str) {
                    let as_of_utc = as_of_ts.with_timezone(&chrono::Utc);
                    filtered_records.retain(|r| r.timestamp <= as_of_utc);
                }
            }

            // Apply limit
            let limit = params.limit.unwrap_or(100);
            filtered_records.truncate(limit);

            let response_records = filtered_records
                .into_iter()
                .map(|r| MemoryRecordResponse {
                    id:          r.id.to_string(),
                    record_type: format!("{:?}", r.record_type),
                    timestamp:   r.timestamp.to_rfc3339(),
                    actor:       r.actor.clone(),
                    action:      r.action.clone(),
                    target:      r.target.clone(),
                    metadata:    r.metadata.clone(),
                    integrity:   r.integrity.clone(),
                    confidence:  r.confidence,
                    source:      r.source.clone(),
                    priority:    r.priority.clone(),
                    tags:        r.tags.clone(),
                    version:     r.version,
                    status:      r.status.clone(),
                    expires_at:  r.expires_at,
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

#[cfg(feature = "web-server")]
async fn handle_forget_actor<B: MemoryBackend + Send + Sync + 'static>(
    memory_store: Arc<Mutex<MemoryStore<B>>>,
    symbolic_store: Arc<Mutex<crate::symbolic_store::SymbolicStore<crate::symbolic_store::InMemoryGraph>>>,
    actor: String,
) -> Result<Json<ForgetActorResponse>, (StatusCode, Json<ForgetActorResponse>)> {
    // Check regulatory hold — GDPR forget blocked if hold is active
    if let Ok(holds) = REGULATORY_HOLDS.lock() {
        if holds.iter().any(|h| h.actor == actor) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ForgetActorResponse {
                    success: false,
                    actor,
                    records_deleted: 0,
                    symbolic_nodes_deleted: 0,
                    error: Some("Regulatory hold active — GDPR forget blocked. Release hold first via DELETE /regulatory/hold/:actor".to_string()),
                }),
            ));
        }
    }

    // Delete from temporal/procedural/reflexion memory store
    let records_deleted = match memory_store.lock() {
        Ok(mut ms) => match ms.delete_by_actor(&actor) {
            Ok(ids) => ids.len(),
            Err(e) => return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ForgetActorResponse {
                    success: false,
                    actor,
                    records_deleted: 0,
                    symbolic_nodes_deleted: 0,
                    error: Some(e.to_string()),
                }),
            )),
        },
        Err(e) => return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ForgetActorResponse {
                success: false,
                actor,
                records_deleted: 0,
                symbolic_nodes_deleted: 0,
                error: Some(format!("Lock error: {}", e)),
            }),
        )),
    };

    // Delete matching symbolic graph nodes (nodes whose "actor" property equals the actor)
    let symbolic_nodes_deleted = match symbolic_store.lock() {
        Ok(mut ss) => {
            let nodes = ss.find_by_property("actor", &actor);
            let count = nodes.len();
            for node in nodes {
                ss.remove_node(node.id);
            }
            count
        }
        Err(_) => 0, // Best-effort: don't fail the request if symbolic lock fails
    };

    // P0.4 — fire webhook for bulk delete event
    fire_webhook("memory.deleted", serde_json::json!({
        "actor": actor,
        "records_deleted": records_deleted,
        "symbolic_nodes_deleted": symbolic_nodes_deleted,
    }));

    Ok(Json(ForgetActorResponse {
        success: true,
        actor,
        records_deleted,
        symbolic_nodes_deleted,
        error: None,
    }))
}

/// POST /webhooks — register a new webhook URL
#[cfg(feature = "web-server")]
async fn handle_register_webhook(
    Json(req): Json<RegisterWebhookRequest>,
) -> Json<RegisterWebhookResponse> {
    let id = uuid::Uuid::new_v4().to_string();
    let reg = WebhookRegistration {
        id: id.clone(),
        url: req.url.clone(),
        events: req.events.clone(),
    };
    if let Ok(mut hooks) = WEBHOOKS.lock() {
        hooks.push(reg);
    }
    Json(RegisterWebhookResponse { id, url: req.url, events: req.events })
}

/// DELETE /webhooks/:id — remove a webhook registration
#[cfg(feature = "web-server")]
async fn handle_delete_webhook(Path(id): Path<String>) -> StatusCode {
    if let Ok(mut hooks) = WEBHOOKS.lock() {
        let before = hooks.len();
        hooks.retain(|h| h.id != id);
        if hooks.len() < before { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND }
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// GET /webhooks — list all registered webhooks
#[cfg(feature = "web-server")]
async fn handle_list_webhooks() -> Json<serde_json::Value> {
    let hooks = WEBHOOKS.lock().map(|h| h.clone()).unwrap_or_default();
    let total = hooks.len();
    Json(serde_json::json!({"webhooks": hooks, "total": total}))
}

// ── G5: quarantine / restore ──────────────────────────────────────────────────

/// POST /memory/quarantine/:id — move a record to quarantine status.
/// Quarantined records are excluded from search/query by default.
#[cfg(feature = "web-server")]
async fn handle_quarantine_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    id_str: String,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = match uuid::Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return Err((StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "invalid UUID"})))),
    };
    match store.lock() {
        Ok(mut ms) => match ms.set_status(id, "quarantine") {
            Ok(_) => Ok(Json(serde_json::json!({"success": true, "id": id_str, "status": "quarantine"}))),
            Err(e) => Err((StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "error": e.to_string()})))),
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("Lock error: {}", e)})))),
    }
}

/// POST /memory/restore/:id — restore a quarantined record to active status.
#[cfg(feature = "web-server")]
async fn handle_restore_memory<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    id_str: String,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = match uuid::Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return Err((StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "invalid UUID"})))),
    };
    match store.lock() {
        Ok(mut ms) => match ms.set_status(id, "active") {
            Ok(_) => Ok(Json(serde_json::json!({"success": true, "id": id_str, "status": "active"}))),
            Err(e) => Err((StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "error": e.to_string()})))),
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("Lock error: {}", e)})))),
    }
}

// ── G8: corroborate / contradict ─────────────────────────────────────────────

/// POST /memory/corroborate/:id — increase confidence by 0.10 (max 1.0).
#[cfg(feature = "web-server")]
async fn handle_corroborate<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    id_str: String,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = match uuid::Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return Err((StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "invalid UUID"})))),
    };
    match store.lock() {
        Ok(mut ms) => match ms.corroborate(id) {
            Ok((before, after)) => Ok(Json(serde_json::json!({
                "success": true, "id": id_str,
                "confidence_before": before, "confidence_after": after
            }))),
            Err(e) => Err((StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "error": e.to_string()})))),
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("Lock error: {}", e)})))),
    }
}

/// POST /memory/contradict/:id — decrease confidence by 0.15. Auto-quarantines if < 0.30.
#[cfg(feature = "web-server")]
async fn handle_contradict<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    id_str: String,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = match uuid::Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => return Err((StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "invalid UUID"})))),
    };
    match store.lock() {
        Ok(mut ms) => match ms.contradict(id) {
            Ok((before, after, quarantined)) => Ok(Json(serde_json::json!({
                "success": true, "id": id_str,
                "confidence_before": before, "confidence_after": after,
                "quarantined": quarantined
            }))),
            Err(e) => Err((StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "error": e.to_string()})))),
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("Lock error: {}", e)})))),
    }
}

// ── G13: /memory/context — formatted LLM prompt context ──────────────────────

#[cfg(feature = "web-server")]
#[derive(serde::Deserialize)]
struct ContextRequest {
    query: String,
    actor: Option<String>,
    limit: Option<usize>,
    max_tokens: Option<usize>,
    format: Option<String>,  // "markdown" (default) | "plain" | "xml"
}

#[cfg(feature = "web-server")]
#[derive(serde::Serialize)]
struct ContextResponse {
    context: String,
    record_count: usize,
    estimated_tokens: usize,
}

/// POST /memory/context — search memory and return a formatted context block
/// ready to inject into an LLM prompt. Zero LLM calls — pure formatting.
#[cfg(feature = "web-server")]
async fn handle_memory_context<B: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<B>>>,
    Json(req): Json<ContextRequest>,
) -> Result<Json<ContextResponse>, (StatusCode, Json<ContextResponse>)> {
    let limit = req.limit.unwrap_or(10).min(50);
    let now_ts = chrono::Utc::now().timestamp();
    match store.lock() {
        Ok(ms) => {
            let mut results = ms.search_semantic(None, &req.query, limit, false);
            // Apply actor filter if provided
            if let Some(actor) = &req.actor {
                results.retain(|(r, _)| &r.actor == actor);
            }
            // Exclude expired
            results.retain(|(r, _)| r.expires_at.map_or(true, |exp| exp > now_ts));
            let record_count = results.len();
            let fmt = req.format.as_deref().unwrap_or("markdown");
            let lines: Vec<String> = results.iter().map(|(r, score)| {
                match fmt {
                    "xml" => format!("  <memory score=\"{:.2}\" confidence=\"{:.2}\" source=\"{}\">[{}] {}</memory>",
                        score, r.confidence, r.source.as_deref().unwrap_or("unknown"), r.action, r.target),
                    "plain" => format!("- [{}] {} (confidence: {:.0}%)",
                        r.action, r.target, r.confidence * 100.0),
                    _ => format!("- **[{}]** {} *(confidence: {:.0}%, source: {})*",
                        r.action, r.target, r.confidence * 100.0,
                        r.source.as_deref().unwrap_or("unknown")),
                }
            }).collect();
            let context = if lines.is_empty() {
                "No relevant memories found.".to_string()
            } else {
                match fmt {
                    "xml" => format!("<memories>\n{}\n</memories>", lines.join("\n")),
                    _ => format!("Relevant memories:\n{}", lines.join("\n")),
                }
            };
            // Truncate by max_tokens if requested (1 token ≈ 4 chars)
            let context = if let Some(max_tok) = req.max_tokens {
                let max_chars = max_tok * 4;
                if context.len() > max_chars { context[..max_chars].to_string() } else { context }
            } else { context };
            let estimated_tokens = context.len() / 4;
            Ok(Json(ContextResponse { context, record_count, estimated_tokens }))
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ContextResponse {
            context: format!("Error: {}", e), record_count: 0, estimated_tokens: 0,
        }))),
    }
}

// ── G5/G6: WorldModel state introspection REST ────────────────────────────────

/// GET /worldmodel/states — all observed states + actions in Dirichlet model
#[cfg(feature = "web-server")]
async fn handle_wm_states(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
) -> Json<serde_json::Value> {
    match world_model.read() {
        Ok(wm) => Json(serde_json::json!({
            "states":            wm.get_states(),
            "actions":           wm.get_actions(),
            "observation_count": wm.transition_count(),
        })),
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}

/// GET /worldmodel/transitions?state=S1 — all predictions from a given state
#[cfg(feature = "web-server")]
#[derive(serde::Deserialize)]
struct WmTransitionsParams { state: String }

#[cfg(feature = "web-server")]
async fn handle_wm_transitions(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Query(params): Query<WmTransitionsParams>,
) -> Json<serde_json::Value> {
    let actions = match world_model.read() {
        Ok(wm) => wm.get_actions(),
        Err(e) => return Json(serde_json::json!({"error": format!("lock: {}", e)})),
    };
    let mut predictions = Vec::new();
    if let Ok(wm) = world_model.read() {
        for action in &actions {
            if let Ok(pred) = wm.predict_next_state(&params.state, action) {
                predictions.push(serde_json::json!({
                    "action":            action,
                    "probabilities":     pred.probabilities,
                    "entropy":           pred.entropy,
                    "observation_count": pred.observation_count,
                }));
            }
        }
    }
    Json(serde_json::json!({
        "state":       params.state,
        "transitions": predictions,
    }))
}

/// GET /worldmodel/uncertainty — entropy for all (state, action) pairs, sorted desc
#[cfg(feature = "web-server")]
async fn handle_wm_uncertainty(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
) -> Json<serde_json::Value> {
    match world_model.read() {
        Ok(wm) => {
            let pairs: Vec<serde_json::Value> = wm.get_all_entropy().iter()
                .map(|(s, a, e)| serde_json::json!({"state": s, "action": a, "entropy": e}))
                .collect();
            let total = pairs.len();
            Json(serde_json::json!({"pairs": pairs, "total": total, "sorted_by": "entropy_desc"}))
        },
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}

// ── G6: WorldModel REST handlers ─────────────────────────────────────────────

/// POST /worldmodel/observe — feed {from, action, to} into Dirichlet transition model
#[cfg(feature = "web-server")]
async fn handle_wm_observe(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let from   = req["from"].as_str().unwrap_or("").to_string();
    let action = req["action"].as_str().unwrap_or("").to_string();
    let to     = req["to"].as_str().unwrap_or("").to_string();
    if from.is_empty() || action.is_empty() || to.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "from, action, to required"}));
    }
    match world_model.write() {
        Ok(mut wm) => match wm.observe_transition(from, action, to) {
            Ok(_)  => Json(serde_json::json!({"success": true, "total_transitions": wm.transition_count()})),
            Err(e) => Json(serde_json::json!({"success": false, "error": e})),
        },
        Err(e) => Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
    }
}

/// GET /worldmodel/predict?state=S1&action=A1 — P(s'|s,a) probability distribution
#[cfg(feature = "web-server")]
#[derive(serde::Deserialize)]
struct WmPredictParams {
    state: String,
    action: String,
}

#[cfg(feature = "web-server")]
async fn handle_wm_predict(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Query(params): Query<WmPredictParams>,
) -> Json<serde_json::Value> {
    match world_model.read() {
        Ok(wm) => match wm.predict_next_state(&params.state, &params.action) {
            Ok(pred) => Json(serde_json::json!({
                "from_state": pred.from_state,
                "action": pred.action,
                "probabilities": pred.probabilities,
                "entropy": pred.entropy,
                "observation_count": pred.observation_count,
            })),
            Err(e) => Json(serde_json::json!({"error": e})),
        },
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}

/// GET /worldmodel/entities — list Kalman-tracked entity IDs
#[cfg(feature = "web-server")]
async fn handle_wm_entities(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
) -> Json<serde_json::Value> {
    match world_model.read() {
        Ok(wm) => match wm.list_entities() {
            Ok(ids) => {
                let total = ids.len();
                Json(serde_json::json!({"entities": ids, "total": total}))
            },
            Err(e)  => Json(serde_json::json!({"entities": [], "error": e})),
        },
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}

/// POST /worldmodel/entity — register entity with initial Kalman state
#[cfg(feature = "web-server")]
async fn handle_wm_register_entity(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    use crate::world_model_enhanced::EntityState;
    let id = req["id"].as_str().unwrap_or("").to_string();
    if id.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "id required"}));
    }
    let dims = req["dimensions"].as_u64().unwrap_or(3) as usize;

    // Use provided initial_values if supplied, otherwise zero-initialise
    let properties: Vec<f64> = req["initial_values"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec![0.0; dims]);

    // Use provided initial_covariance if supplied, otherwise identity matrix
    let d = properties.len();
    let covariance: Vec<Vec<f64>> = req["initial_covariance"]
        .as_array()
        .map(|rows| rows.iter()
            .filter_map(|row| row.as_array()
                .map(|r| r.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>()))
            .collect::<Vec<Vec<f64>>>())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| {
            (0..d).map(|i| (0..d).map(|j| if i == j { 1.0f64 } else { 0.0 }).collect()).collect()
        });

    let initial = EntityState { properties, covariance };
    match world_model.write() {
        Ok(mut wm) => match wm.register_entity(id.clone(), initial) {
            Ok(_)  => Json(serde_json::json!({"success": true, "id": id})),
            Err(e) => Json(serde_json::json!({"success": false, "error": e})),
        },
        Err(e) => Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
    }
}

/// GET /worldmodel/causal — dump causal DAG edges
#[cfg(feature = "web-server")]
async fn handle_wm_causal(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
) -> Json<serde_json::Value> {
    match world_model.read() {
        Ok(wm) => {
            let edges: Vec<serde_json::Value> = wm.get_causal_edges().iter()
                .map(|e| serde_json::json!({"from": e.from, "to": e.to, "strength": e.strength}))
                .collect();
            let total = edges.len();
            Json(serde_json::json!({"edges": edges, "total": total}))
        },
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}

/// POST /worldmodel/causal/edge — add a causal edge to WorldModelEnhanced CausalGraph
/// Body: {"from": "X", "to": "Y"} — adds directed edge X → Y (cycle prevention enforced)
#[cfg(feature = "web-server")]
async fn handle_wm_causal_add_edge(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let from = req["from"].as_str().unwrap_or("").to_string();
    let to   = req["to"].as_str().unwrap_or("").to_string();
    if from.is_empty() || to.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "from and to required"}));
    }
    match world_model.write() {
        Ok(mut wm) => match wm.add_causal_edge(from.clone(), to.clone()) {
            Ok(_)  => Json(serde_json::json!({"success": true, "from": from, "to": to})),
            Err(e) => Json(serde_json::json!({"success": false, "error": e})),
        },
        Err(e) => Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
    }
}

/// POST /worldmodel/causal/intervention — P(Y|do(X=x)) do-calculus query
/// Body: {"outcome": "Y", "intervention_var": "X", "intervention_value": 1.0, "conditioned_on": {}}
#[cfg(feature = "web-server")]
async fn handle_wm_causal_intervention(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let outcome          = req["outcome"].as_str().unwrap_or("").to_string();
    let intervention_var = req["intervention_var"].as_str().unwrap_or("").to_string();
    let intervention_value = req["intervention_value"].as_f64().unwrap_or(0.0);
    if outcome.is_empty() || intervention_var.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "outcome and intervention_var required"}));
    }
    let conditioned_on: std::collections::HashMap<String, f64> = req["conditioned_on"]
        .as_object()
        .map(|obj| obj.iter()
            .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
            .collect())
        .unwrap_or_default();

    use crate::world_model_enhanced::InterventionQuery;
    let query = InterventionQuery { outcome, intervention_var, intervention_value, conditioned_on };

    match world_model.read() {
        Ok(wm) => match wm.causal_intervention(query) {
            Ok(result) => Json(serde_json::json!({"success": true, "outcome_probabilities": result})),
            Err(e)     => Json(serde_json::json!({"success": false, "error": e})),
        },
        Err(e) => Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
    }
}

/// POST /worldmodel/causal/counterfactual — "what if X had been x instead?"
/// Body: {"actual_state": {"X": 0.5, "Y": 0.3}, "intervention_var": "X", "intervention_value": 1.0}
#[cfg(feature = "web-server")]
async fn handle_wm_causal_counterfactual(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let intervention_var   = req["intervention_var"].as_str().unwrap_or("").to_string();
    let intervention_value = req["intervention_value"].as_f64().unwrap_or(0.0);
    if intervention_var.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "intervention_var required"}));
    }
    let actual_state: std::collections::HashMap<String, f64> = req["actual_state"]
        .as_object()
        .map(|obj| obj.iter()
            .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
            .collect())
        .unwrap_or_default();

    match world_model.read() {
        Ok(wm) => match wm.counterfactual(actual_state, intervention_var, intervention_value) {
            Ok(result) => Json(serde_json::json!({"success": true, "counterfactual_outcome": result})),
            Err(e)     => Json(serde_json::json!({"success": false, "error": e})),
        },
        Err(e) => Json(serde_json::json!({"success": false, "error": format!("lock: {}", e)})),
    }
}

// ── G7: AureusBridge REST handlers ───────────────────────────────────────────

/// POST /memory/reflect — run AureusBridge reflexion over memory context
#[cfg(feature = "web-server")]
async fn handle_memory_reflect<B: MemoryBackend + Send + Sync + 'static>(
    memory_store: Arc<Mutex<MemoryStore<B>>>,
    aureus: Arc<Mutex<AureusBridge>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let query = req["query"].as_str().unwrap_or("recent decisions").to_string();
    // Lock store first, then bridge (consistent lock ordering to avoid deadlock)
    let mut store = match memory_store.lock() {
        Ok(s)  => s,
        Err(e) => return Json(serde_json::json!({"error": format!("store lock: {}", e)})),
    };
    let mut bridge = match aureus.lock() {
        Ok(b)  => b,
        Err(e) => return Json(serde_json::json!({"error": format!("bridge lock: {}", e)})),
    };
    let llm_available = bridge.llm_configured();
    let loops_before = bridge.loops_run();
    let hyp = bridge.reflect_on_memory(&query, &mut *store);
    let is_fallback = bridge.loops_run() == loops_before; // no new loop ran → fallback
    Json(serde_json::json!({
        "hypothesis":    hyp.text,
        "confidence":    hyp.confidence,
        "evidence":      hyp.evidence,
        "loops_run":     bridge.loops_run(),
        "llm_available": llm_available,
        "is_fallback":   is_fallback,
    }))
}

/// GET /memory/hypotheses — AureusBridge reflexion metadata
#[cfg(feature = "web-server")]
async fn handle_memory_hypotheses(
    aureus: Arc<Mutex<AureusBridge>>,
) -> Json<serde_json::Value> {
    match aureus.lock() {
        Ok(bridge) => {
            let hyps: Vec<serde_json::Value> = bridge.top_hypotheses(10)
                .iter()
                .map(|h| serde_json::json!({
                    "text":       h.text,
                    "confidence": h.confidence,
                    "evidence":   h.evidence,
                }))
                .collect();
            Json(serde_json::json!({
                "loops_run":        bridge.loops_run(),
                "hypothesis_count": bridge.hypothesis_count(),
                "llm_available":    bridge.llm_configured(),
                "top_hypotheses":   hyps,
            }))
        },
        Err(e) => Json(serde_json::json!({"error": format!("lock: {}", e)})),
    }
}

/// GET /worldmodel/status — world model availability and real state summary.
#[cfg(feature = "web-server")]
async fn handle_worldmodel_status(
    world_model: Arc<RwLock<WorldModelEnhanced>>,
) -> Json<serde_json::Value> {
    let (total_transitions, entity_count) = match world_model.read() {
        Ok(wm) => (wm.transition_count(), wm.list_entities().unwrap_or_default().len()),
        Err(_) => (0, 0),
    };
    Json(serde_json::json!({
        "status": "available",
        "mode": "full",
        "total_transitions_observed": total_transitions,
        "tracked_entities": entity_count,
        "endpoints": {
            "observe": "POST /worldmodel/observe",
            "predict": "GET /worldmodel/predict?state=&action=",
            "entities": "GET /worldmodel/entities",
            "entity": "POST /worldmodel/entity",
            "causal": "GET /worldmodel/causal"
        }
    }))
}

/// GET /coherence/inconsistencies — list currently detected inconsistencies
#[cfg(feature = "web-server")]
/// Backward-compat wrapper: creates fresh CoherenceChecker per request.
/// Used by run_with_both_stores only.
#[cfg(feature = "web-server")]
async fn handle_coherence_inconsistencies_fresh() -> Json<serde_json::Value> {
    let checker = Arc::new(CoherenceChecker::new());
    handle_coherence_inconsistencies(checker).await
}

async fn handle_coherence_inconsistencies(
    coherence: Arc<CoherenceChecker>,
) -> Json<serde_json::Value> {
    match coherence.check_consistency() {
        Ok(reports) => {
            let items: Vec<serde_json::Value> = reports.iter().map(|r| serde_json::json!({
                "id": r.id,
                "type": format!("{:?}", r.inconsistency_type),
                "affected": r.affected_entities,
                "description": r.description,
                "severity": r.severity,
                "detected_at": r.detected_at,
            })).collect();
            let total = items.len();
            Json(serde_json::json!({
                "inconsistencies": items,
                "total": total,
                "checked_at": chrono::Utc::now().to_rfc3339()
            }))
        }
        Err(e) => Json(serde_json::json!({
            "inconsistencies": [],
            "total": 0,
            "error": e
        })),
    }
}

// ── G15/G16: SelfModel can-execute + capability registration ─────────────────

/// GET /self/can-execute?operation=add_memory — SelfModel decision engine query
#[cfg(feature = "web-server")]
#[derive(serde::Deserialize)]
struct SelfCanExecuteParams { operation: String }

#[cfg(feature = "web-server")]
async fn handle_self_can_execute(
    self_model: Arc<SelfModel>,
    Query(params): Query<SelfCanExecuteParams>,
) -> Json<serde_json::Value> {
    use crate::self_model::DecisionContext;
    let context = DecisionContext::default_context();
    match self_model.can_execute(&params.operation, context) {
        Ok(decision) => Json(serde_json::json!({
            "operation":        params.operation,
            "should_execute":   decision.should_execute,
            "confidence":       decision.confidence,
            "rationale":        decision.rationale,
            "expected_utility": decision.expected_utility,
            "predicted_resources": decision.predicted_resources.as_ref().map(|r| serde_json::json!({
                "cpu_percent":  r.cpu_percent,
                "memory_mb":    r.memory_mb,
            })),
        })),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

/// POST /self/capabilities — register a new capability at runtime
#[cfg(feature = "web-server")]
async fn handle_self_register_capability(
    self_model: Arc<SelfModel>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    use crate::self_model::CapabilityDescriptor;
    let name = req["name"].as_str().unwrap_or("").to_string();
    if name.is_empty() {
        return Json(serde_json::json!({"success": false, "error": "name required"}));
    }
    let cap = CapabilityDescriptor {
        name: name.clone(),
        description: req["description"].as_str().unwrap_or("").to_string(),
        required_cpu_percent: req["required_cpu_percent"].as_f64().unwrap_or(5.0),
        required_memory_mb:   req["required_memory_mb"].as_f64().unwrap_or(50.0),
        limitations: vec![],
    };
    match self_model.register_capability(cap) {
        Ok(_)  => Json(serde_json::json!({"success": true, "name": name})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e})),
    }
}

// ── G8: SelfModel REST handlers ──────────────────────────────────────────────

/// GET /self/health — SelfModel overall health score
#[cfg(feature = "web-server")]
async fn handle_self_health(
    self_model: Arc<SelfModel>,
) -> Json<serde_json::Value> {
    match self_model.get_health() {
        Ok(score) => Json(serde_json::json!({
            "healthy": score.overall >= 0.7,
            "overall": score.overall,
        })),
        Err(e) => Json(serde_json::json!({"healthy": false, "error": e})),
    }
}

/// GET /self/capabilities — list registered capability descriptors
#[cfg(feature = "web-server")]
async fn handle_self_capabilities(
    self_model: Arc<SelfModel>,
) -> Json<serde_json::Value> {
    let ops = ["add_memory", "search_memory", "query_memory", "ingest",
               "bulk_add", "forget", "reflect", "context"];
    let capabilities: Vec<serde_json::Value> = ops.iter().map(|op| {
        match self_model.get_capability(op) {
            Ok(cap) => serde_json::json!({
                "name": cap.name,
                "description": cap.description,
                "required_cpu_percent": cap.required_cpu_percent,
                "required_memory_mb": cap.required_memory_mb,
            }),
            Err(_) => serde_json::json!({"name": op, "registered": false}),
        }
    }).collect();
    let total = capabilities.len();
    Json(serde_json::json!({"capabilities": capabilities, "total": total}))
}

#[cfg(feature = "web-server")]
async fn handle_coherence_status(
    coherence: Arc<CoherenceChecker>,
) -> Json<CoherenceStatusResponse> {
    let metrics = match coherence.get_metrics() {
        Ok(m) => m,
        Err(_) => crate::coherence::CoherenceMetrics::new(),
    };
    Json(CoherenceStatusResponse {
        coherence_score: metrics.coherence_score,
        total_checks: metrics.total_checks,
        inconsistencies_found: metrics.inconsistencies_found,
        auto_resolutions_succeeded: metrics.auto_resolutions_succeeded,
        auto_resolutions_failed: metrics.auto_resolutions_failed,
        invariants_validated: metrics.invariants_validated,
        invariants_violated: metrics.invariants_violated,
        healthy: metrics.coherence_score >= 0.8,
    })
}
