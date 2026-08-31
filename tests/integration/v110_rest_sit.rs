#![cfg(feature = "web-server")]
/// HTTP-level SIT: v1.1.0 REST routes (cognitive/report, goals, actions/authorized,
/// memory/:id/provenance). Uses a real local TCP server + reqwest to avoid axum-test
/// version skew (axum-test 12.x requires axum 0.7; we use axum 0.6).
/// Run: cargo test --no-default-features --features "petgraph_backend,web-server" \
///           --test integration_suite v110_rest

use hipcortex::{
    archive_store::ArchiveStore,
    aureus_bridge::AureusBridge,
    cognitive_gc::CognitiveGC,
    cognitive_state::CognitiveHandle,
    coherence::CoherenceChecker,
    memory_record::{MemoryRecord, MemoryType},
    memory_store::MemoryStore,
    InMemoryBackend,
    self_model::{calibration::CalibrationTracker, SelfModel},
    symbolic_store::SymbolicStore,
    topological_memory::CausalTopoGraph,
    web_server::{build_app, AppState},
    world_model_enhanced::WorldModelEnhanced,
};
use std::sync::{Arc, Mutex, RwLock};

type TestState = AppState<InMemoryBackend>;

fn make_test_state() -> TestState {
    let memory_store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let coherence = Arc::new(CoherenceChecker::new());
    let calibration = Arc::new(CalibrationTracker::new());
    let world_model = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let self_model = Arc::new(SelfModel::new());
    let cognitive = Arc::new(CognitiveHandle::new(
        Arc::clone(&memory_store),
        Arc::clone(&world_model),
        Arc::clone(&self_model),
        None,
        Arc::clone(&coherence),
        Arc::clone(&calibration),
        Arc::new(CognitiveGC::new()),
    ));
    let archive_path = std::env::temp_dir()
        .join(format!("hipcortex-rest-test-{}.jsonl", uuid::Uuid::new_v4()));
    AppState {
        memory_store,
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model,
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model,
        coherence,
        topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
        archive_store: Arc::new(Mutex::new(ArchiveStore::new(archive_path))),
        tx_log: None,
        calibration,
        cognitive,
        forks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        twins: Arc::new(Mutex::new(std::collections::HashMap::new())),
    }
}

/// Spin up a real local server on a random port. Returns (base_url, shutdown_handle).
async fn start_test_server(state: TestState) -> (String, tokio::task::JoinHandle<()>) {
    let app = build_app(state);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service())
            .await
            .unwrap_or_default();
    });
    // Give the server a moment to bind.
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    (format!("http://127.0.0.1:{}", addr.port()), handle)
}

// ── AC-1 ─────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn ac1_rest_cognitive_report_returns_all_keys() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/v1/cognitive/report?actor=rest_agent", base))
        .send().await.unwrap()
        .json().await.unwrap();
    srv.abort();
    assert!(body.get("active_goals").is_some(), "missing active_goals: {body}");
    assert!(body.get("authorized_actions").is_some(), "missing authorized_actions: {body}");
    assert!(body.get("next_recommendation").is_some(), "missing next_recommendation: {body}");
}

// ── AC-2 ─────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn ac2_rest_goals_filtered_by_status() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let client = reqwest::Client::new();

    // Seed via REST — parse_record_type_alias now maps "Goal" correctly.
    for (target, status) in [("g_pending", "Pending"), ("g_failed", "Failed")] {
        let resp = client.post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": "rest_agent", "action": "pursue", "target": target,
                "record_type": "Goal",
                "metadata": { "status": status, "target_state": target },
            }))
            .send().await.unwrap();
        assert!(resp.status().is_success(), "add failed for {target}: {}", resp.status());
    }

    let body: serde_json::Value = client
        .get(format!("{}/v1/goals?actor=rest_agent&status=pending", base))
        .send().await.unwrap().json().await.unwrap();
    srv.abort();

    assert_eq!(body["count"], 1, "expected 1 Pending goal, got {body}");
    assert_eq!(body["goals"][0]["actor"], "rest_agent");
}

// ── AC-3 ─────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn ac3_rest_authorized_actions_has_ops() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/v1/actions/authorized", base))
        .send().await.unwrap()
        .json().await.unwrap();
    srv.abort();
    let ops = body["authorized"].as_array().expect("authorized must be array");
    assert!(ops.len() >= 3, "expected ≥3 authorized ops, got {}: {body}", ops.len());
    assert!(ops[0].get("op").is_some(), "each op must have 'op' key: {ops:?}");
}

// ── AC-4 ─────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn ac4_rest_provenance_chain_returns_ancestor() {
    let state = make_test_state();

    // Seed parent + child directly into the shared store before the server starts.
    let child_id = {
        let mut store = state.cognitive.memory.lock().unwrap();
        let parent = MemoryRecord::new(
            MemoryType::Goal,
            "rest_agent".into(),
            "root".into(),
            "parent_goal".into(),
            serde_json::json!({}),
        );
        let parent_id = parent.id;
        store.add(parent).unwrap();
        let mut child = MemoryRecord::new(
            MemoryType::Temporal,
            "rest_agent".into(),
            "observe".into(),
            "child_obs".into(),
            serde_json::json!({}),
        );
        child.derived_from = Some(parent_id);
        let id = child.id;
        store.add(child).unwrap();
        id
    };

    let (base, srv) = start_test_server(state).await;
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/v1/memory/{}/provenance", base, child_id))
        .send().await.unwrap()
        .json().await.unwrap();
    srv.abort();

    let depth = body["depth"].as_u64().unwrap_or(0);
    assert!(depth >= 1, "chain must contain ≥1 ancestor, got depth={depth}: {body}");
}

// ── AC-5 ─────────────────────────────────────────────────────────────────────
// (renumbered: original AC-5 provenance-bad-uuid kept below)

// ── parse_record_type_alias round-trip tests (RTA) ───────────────────────────

async fn add_and_query(base: &str, record_type: &str, actor: &str) -> serde_json::Value {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/memory/add", base))
        .json(&serde_json::json!({
            "actor": actor, "action": "test", "target": "t",
            "record_type": record_type,
        }))
        .send().await.unwrap();
    assert!(resp.status().is_success(), "add failed: {}", resp.status());
    client.get(format!("{}/memory/query?actor={}", base, actor))
        .send().await.unwrap().json().await.unwrap()
}

#[tokio::test]
async fn rta1_goal_stored_as_goal() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let body = add_and_query(&base, "Goal", "rta_goal").await;
    srv.abort();
    assert_eq!(body["records"][0]["record_type"], "Goal", "Goal not stored as Goal: {body}");
}

#[tokio::test]
async fn rta2_belief_stored_as_belief() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let body = add_and_query(&base, "Belief", "rta_belief").await;
    srv.abort();
    assert_eq!(body["records"][0]["record_type"], "Belief", "Belief not stored as Belief: {body}");
}

#[tokio::test]
async fn rta3_skill_stored_as_skill() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let body = add_and_query(&base, "Skill", "rta_skill").await;
    srv.abort();
    assert_eq!(body["records"][0]["record_type"], "Skill", "Skill not stored as Skill: {body}");
}

#[tokio::test]
async fn rta4_decision_stored_as_decision() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let body = add_and_query(&base, "Decision", "rta_decision").await;
    srv.abort();
    assert_eq!(body["records"][0]["record_type"], "Decision", "Decision not stored as Decision: {body}");
}

#[tokio::test]
async fn rta5_existing_aliases_still_map() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let client = reqwest::Client::new();
    for (alias, expected) in [("Episodic", "Temporal"), ("Reflexive", "Reflexion"), ("Semantic", "Symbolic")] {
        let actor = format!("rta_{}", alias.to_lowercase());
        let resp = client.post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": actor, "action": "test", "target": "t",
                "record_type": alias,
            }))
            .send().await.unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = client
            .get(format!("{}/memory/query?actor={}", base, actor))
            .send().await.unwrap().json().await.unwrap();
        assert_eq!(body["records"][0]["record_type"], expected,
            "alias {alias} → expected {expected}: {body}");
    }
    srv.abort();
}

#[tokio::test]
async fn rta6_unknown_type_falls_to_temporal() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let body = add_and_query(&base, "Bogus", "rta_bogus").await;
    srv.abort();
    assert_eq!(body["records"][0]["record_type"], "Temporal",
        "unknown type must default to Temporal: {body}");
}

// ── AC-5 (provenance bad UUID) ────────────────────────────────────────────────
#[tokio::test]
async fn ac5_rest_provenance_bad_uuid_returns_400() {
    let (base, srv) = start_test_server(make_test_state()).await;
    let resp = reqwest::Client::new()
        .get(format!("{}/v1/memory/not-a-uuid/provenance", base))
        .send().await.unwrap();
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap();
    srv.abort();
    assert_eq!(status, 400, "expected 400 for bad UUID: {body}");
    assert!(body.get("error").is_some(), "400 response must have 'error' key: {body}");
}
