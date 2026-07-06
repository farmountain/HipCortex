//! SIT tests for v0.4.0 server-extension contract fixes.
//! G-LINK: POST /memory/link field name aliases
//! G-BELIEFS: GET /memory/live_beliefs loops_run key
//! G-RELATED: GET /memory/search/related results enrichment

use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::web_server::AppState;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::self_model::SelfModel;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use hipcortex::CausalTopoGraph;
use std::sync::{Arc, Mutex, RwLock};

fn make_state() -> AppState<InMemoryBackend> {
    AppState {
        memory_store:   Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model:    Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus:         Arc::new(Mutex::new(AureusBridge::new())),
        self_model:     Arc::new(SelfModel::new()),
        coherence:      Arc::new(CoherenceChecker::new()),
        topo_graph:     Arc::new(Mutex::new(CausalTopoGraph::new())),
    }
}

// ── G-LINK: serde alias tests ─────────────────────────────────────────────────

#[test]
fn test_memory_link_request_accepts_source_id_target_id() {
    use hipcortex::web_server::MemoryLinkRequest;
    let json = r#"{"source_id":"00000000-0000-0000-0000-000000000001","target_id":"00000000-0000-0000-0000-000000000002","relation":"supports"}"#;
    let req: MemoryLinkRequest = serde_json::from_str(json)
        .expect("extension-style source_id/target_id must deserialize");
    assert_eq!(req.from_id, "00000000-0000-0000-0000-000000000001");
    assert_eq!(req.to_id,   "00000000-0000-0000-0000-000000000002");
    assert_eq!(req.relation, "supports");
}

#[test]
fn test_memory_link_request_still_accepts_from_id_to_id() {
    use hipcortex::web_server::MemoryLinkRequest;
    let json = r#"{"from_id":"00000000-0000-0000-0000-000000000003","to_id":"00000000-0000-0000-0000-000000000004","relation":"caused_by"}"#;
    let req: MemoryLinkRequest = serde_json::from_str(json)
        .expect("SDK-style from_id/to_id must still deserialize");
    assert_eq!(req.from_id, "00000000-0000-0000-0000-000000000003");
    assert_eq!(req.to_id,   "00000000-0000-0000-0000-000000000004");
}

// ── G-BELIEFS: loops_run present at top level ─────────────────────────────────

#[tokio::test]
async fn test_live_beliefs_has_loops_run_at_top_level() {
    let state = make_state();
    let addr: std::net::SocketAddr = "127.0.0.1:3050".parse().unwrap();
    let srv = tokio::spawn(async move {
        hipcortex::web_server::run_with_state(addr, state).await;
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let resp = reqwest::get("http://127.0.0.1:3050/memory/live_beliefs")
        .await
        .expect("request failed");
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(
        body.get("loops_run").is_some(),
        "loops_run key missing — extension status bar will always show 0"
    );
    assert!(
        body["loops_run"].is_number(),
        "loops_run must be a number, got: {:?}",
        body["loops_run"]
    );

    srv.abort();
}

// ── G-RELATED: results key present with full record data ─────────────────────

#[tokio::test]
async fn test_search_related_returns_results_with_record_data() {
    let state = make_state();
    let addr: std::net::SocketAddr = "127.0.0.1:3051".parse().unwrap();
    let srv = tokio::spawn(async move {
        hipcortex::web_server::run_with_state(addr, state).await;
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let base = "http://127.0.0.1:3051";

    // Add memory A
    let body_a: serde_json::Value = client
        .post(&format!("{}/memory/add", base))
        .json(&serde_json::json!({"actor":"test","action":"decided","target":"use postgres"}))
        .send().await.unwrap()
        .json().await.unwrap();
    let id_a = body_a["record_id"].as_str().expect("record_id missing from add response").to_string();

    // Add memory B
    let body_b: serde_json::Value = client
        .post(&format!("{}/memory/add", base))
        .json(&serde_json::json!({"actor":"test","action":"confirmed","target":"postgres scales well"}))
        .send().await.unwrap()
        .json().await.unwrap();
    let id_b = body_b["record_id"].as_str().expect("record_id missing").to_string();

    // Link A → B (using from_id/to_id so this doesn't depend on Task 1 alias)
    let link = client
        .post(&format!("{}/memory/link", base))
        .json(&serde_json::json!({"from_id": id_a, "to_id": id_b, "relation": "supports"}))
        .send().await.unwrap();
    assert_eq!(link.status().as_u16(), 200, "link failed: {}", link.text().await.unwrap_or_default());

    // Search related from seed A
    let rel: serde_json::Value = client
        .get(&format!("{}/memory/search/related?seed_id={}&limit=10", base, id_a))
        .send().await.unwrap()
        .json().await.unwrap();

    // Must have results key (extension reads this)
    assert!(rel.get("results").is_some(), "results key missing — hipcortex_graph_search LM tool will always return empty");
    let results = rel["results"].as_array().expect("results must be array");
    assert!(!results.is_empty(), "expected at least one PPR result for linked seed");

    let first = &results[0];
    assert!(first.get("score").is_some(), "score missing from results[0]");
    assert!(first.get("record").is_some(), "record missing from results[0]");
    let record = &first["record"];
    assert!(record.get("id").is_some(),     "record.id missing");
    assert!(record.get("actor").is_some(),  "record.actor missing");
    assert!(record.get("action").is_some(), "record.action missing");
    assert!(record.get("target").is_some(), "record.target missing");
    // Verify it's actually the linked record B
    assert_eq!(record["action"], "confirmed",        "expected record B action");
    assert_eq!(record["actor"],  "test",             "expected record B actor");

    // Backward compat: related key still present
    assert!(rel.get("related").is_some(), "related key must still exist for backward compat");

    srv.abort();
}
