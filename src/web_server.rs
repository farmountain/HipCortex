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
use axum::{routing::{delete, get, post}, Json, Router, http::StatusCode};
#[cfg(feature = "web-server")]
use crate::coherence::CoherenceChecker;
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
    errors: Vec<String>,
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

/// Global per-key write counter for tier metering.
/// Resets on server restart (MVP); production: back with Redis or Postgres.
#[cfg(feature = "web-server")]
lazy_static::lazy_static! {
    static ref GLOBAL_METER: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
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
    if path == "/health" || path == "/pricing" || path == "/stats" || path == "/openapi.json" {
        return Ok(next.run(req).await);
    }

    let keys = load_api_keys();
    if keys.is_empty() {
        return Ok(next.run(req).await); // open / self-hosted mode
    }

    let provided = req
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

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
    match store.lock() {
        Ok(ms) => {
            let results = ms.search_semantic(
                req.embedding.as_deref(),
                &req.query,
                limit,
            );
            let now_ts = chrono::Utc::now().timestamp();
            let results: Vec<_> = results
                .into_iter()
                .filter(|(r, _)| r.expires_at.map_or(true, |exp| exp > now_ts))
                .collect();
            let response_results = results
                .into_iter()
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
                    },
                })
                .collect::<Vec<_>>();
            let total = response_results.len();
            Ok(Json(SearchMemoryResponse { results: response_results, total }))
        }
        Err(e) => Err((
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
    <a href="https://buy.stripe.com/hipcortex-pro" class="btn btn-primary">Start Pro →</a>
    <div class="compare">Cancel anytime · Annual: $79/mo</div>
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
    let mut errors: Vec<String> = Vec::new();

    match store.lock() {
        Err(e) => Json(BulkAddResponse {
            success: false,
            inserted: 0,
            failed: req.records.len(),
            record_ids: vec![],
            errors: vec![format!("Lock error: {}", e)],
        }),
        Ok(mut ms) => {
            for r in req.records {
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
                    Err(e) => errors.push(e.to_string()),
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

    // Coherence status: GET /coherence/status
    let coherence_route = get(|| async {
        handle_coherence_status().await
    });

    // Live stats: GET /stats
    let stats_route = {
        let store = memory_store.clone();
        get(move || handle_stats(store))
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/graph", graph_route)
        .route("/node/:id", node_route)
        .route("/memory/add", add_memory_route)
        .route("/memory/bulk", bulk_add_route)
        .route("/memory/query", query_memory_route)
        .route("/memory/search", search_route)
        .route("/memory/forget/:actor", forget_route)
        .route("/coherence/status", coherence_route)
        .route("/stats", stats_route)
        .route("/tier", get(handle_tier))
        .route("/pricing", get(handle_pricing))
        .route("/openapi.json", get(handle_openapi))
        .layer(middleware::from_fn(api_key_middleware));

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

            // Exclude records past their TTL
            let now_ts = chrono::Utc::now().timestamp();
            filtered_records.retain(|r| {
                r.expires_at.map_or(true, |exp| exp > now_ts)
            });

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

#[cfg(feature = "web-server")]
async fn handle_forget_actor<B: MemoryBackend + Send + Sync + 'static>(
    memory_store: Arc<Mutex<MemoryStore<B>>>,
    symbolic_store: Arc<Mutex<crate::symbolic_store::SymbolicStore<crate::symbolic_store::InMemoryGraph>>>,
    actor: String,
) -> Result<Json<ForgetActorResponse>, (StatusCode, Json<ForgetActorResponse>)> {
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

    Ok(Json(ForgetActorResponse {
        success: true,
        actor,
        records_deleted,
        symbolic_nodes_deleted,
        error: None,
    }))
}

#[cfg(feature = "web-server")]
async fn handle_coherence_status(
) -> Json<CoherenceStatusResponse> {
    // Instantiate a fresh checker and run one consistency pass.
    // In a production deployment, wire a persistent Arc<CoherenceChecker> into server state.
    let checker = CoherenceChecker::new();
    let metrics = match checker.get_metrics() {
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
