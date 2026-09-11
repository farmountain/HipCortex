// Server-side passive capture middleware.
//
// Every successful mutation request (POST/PUT/DELETE) writes a Temporal record
// automatically — regardless of which client channel produced it (MCP, VSIX,
// Claude Code, direct REST). No client changes required.
//
// Env: HIPCORTEX_PASSIVE_CAPTURE=false  → disables capture entirely.
//      X-Actor: <name>                  → client-supplied actor (header takes priority).

#[cfg(feature = "web-server")]
use axum::{
    http::{Method, Request},
    middleware::Next,
    response::Response,
};
#[cfg(feature = "web-server")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "web-server")]
use uuid::Uuid;

#[cfg(feature = "web-server")]
use crate::memory_record::{MemoryRecord, MemoryType};
#[cfg(feature = "web-server")]
use crate::memory_store::MemoryStore;
#[cfg(feature = "web-server")]
use crate::persistence::MemoryBackend;

/// Paths that never generate a capture — read-only or meta endpoints.
#[cfg(feature = "web-server")]
const PASSTHROUGH: &[&str] = &[
    "/",
    "/health",
    "/pricing",
    "/stats",
    "/metrics",
    "/openapi.json",
    "/ns",
    "/memory/search",
    "/memory/search-flat",
    "/memory/query",
    "/memory/export",
    "/memory/latest",
    "/memory/hypotheses",
    "/audit/verify",
    "/audit/export",
    "/coherence/status",
    "/coherence/inconsistencies",
    "/worldmodel/status",
    "/worldmodel/predict",
    "/worldmodel/rollout",
    "/worldmodel/entities",
    "/worldmodel/causal",
    "/worldmodel/states",
    "/worldmodel/transitions",
    "/worldmodel/uncertainty",
    "/graph/search",
    "/v1/state/export",
    "/v1/cognitive/report",
    "/self/health",
    "/self/capabilities",
    "/self/can-execute",
    "/predict/entity",
];

#[cfg(feature = "web-server")]
fn is_passthrough(path: &str, method: &Method) -> bool {
    if matches!(
        method,
        &Method::GET | &Method::HEAD | &Method::OPTIONS
    ) {
        return true;
    }
    PASSTHROUGH
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{}/", p)))
}

/// Read `HIPCORTEX_PASSIVE_CAPTURE` env var **once** at server startup.
/// Pass the result into the closure that wraps `passive_capture_mw`.
#[cfg(feature = "web-server")]
pub fn passive_capture_enabled() -> bool {
    std::env::var("HIPCORTEX_PASSIVE_CAPTURE")
        .map(|v| v.to_lowercase() != "false" && v != "0")
        .unwrap_or(true)
}

/// Middleware: capture every successful mutation as a Temporal record.
///
/// `enabled` is resolved once at `build_app` time via `passive_capture_enabled()`.
/// Wire via closure to inject store + flag:
/// ```ignore
/// let cap = memory_store.clone();
/// let enabled = passive_capture_enabled();
/// app.layer(middleware::from_fn(move |req, next| {
///     let s = cap.clone();
///     passive_capture_mw(s, enabled, req, next)
/// }))
/// ```
#[cfg(feature = "web-server")]
pub async fn passive_capture_mw<ReqBody: Send + 'static, MB: MemoryBackend + Send + Sync + 'static>(
    store: Arc<Mutex<MemoryStore<MB>>>,
    enabled: bool,
    req: Request<ReqBody>,
    next: Next<ReqBody>,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    if !enabled || is_passthrough(&path, &method) {
        return next.run(req).await;
    }

    // X-Actor header → body field fallback handled at write time by clients.
    // Default to "unknown-channel" so capture always writes.
    let actor = req
        .headers()
        .get("x-actor")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown-channel")
        .to_string();

    let response = next.run(req).await;

    if response.status().is_success() {
        let action = path
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("unknown")
            .to_string();
        let method_str = method.to_string();
        let path_clone = path.clone();

        tokio::spawn(async move {
            let now = chrono::Utc::now();
            let record = MemoryRecord {
                id: Uuid::new_v4(),
                record_type: MemoryType::Temporal,
                timestamp: now,
                actor,
                action,
                target: path_clone,
                metadata: serde_json::json!({
                    "method": method_str,
                    "source": "server-passive-capture"
                }),
                integrity: None,
                access_count: 0,
                last_accessed: now,
                relevance_score: 0.5,
                content_hash: None,
                expires_at: None,
                confidence: 0.9,
                source: Some("server-passive-capture".to_string()),
                version: 0,
                tags: vec!["server-passive-capture".to_string()],
                priority: "low".to_string(),
                status: "active".to_string(),
                evidence: vec![],
                derived_from: None,
                react_iteration: None,
            };
            if let Ok(mut s) = store.lock() {
                let _ = s.add(record);
            }
        });
    }

    response
}
