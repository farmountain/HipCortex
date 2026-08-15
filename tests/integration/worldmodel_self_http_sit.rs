//! HTTP SIT: worldmodel + self routes on run_with_state (web-server).
//! cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite worldmodel_self

use hipcortex::archive_store::ArchiveStore;
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::self_model::{CapabilityDescriptor, SelfModel};
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::web_server::AppState;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::CausalTopoGraph;
use std::sync::{Arc, Mutex, RwLock};

fn make_state() -> AppState<InMemoryBackend> {
    let self_model = Arc::new(SelfModel::new());
    let _ = self_model.register_capability(CapabilityDescriptor {
        name: "add_memory".into(),
        description: "test".into(),
        required_cpu_percent: 5.0,
        required_memory_mb: 50.0,
        limitations: vec![],
    });
    AppState {
        memory_store: Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::<InMemoryGraph>::new())),
        world_model: Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model,
        coherence: Arc::new(CoherenceChecker::new()),
        topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
        archive_store: Arc::new(Mutex::new(ArchiveStore::new(
            std::env::temp_dir().join("hc-test-wm-archive.jsonl"),
        ))),
        tx_log: None,
        calibration: Arc::new(CalibrationTracker::new()),
    }
}

#[tokio::test]
async fn worldmodel_observe_predict_rollout_and_self_surface() {
    let state = make_state();
    let addr: std::net::SocketAddr = "127.0.0.1:3061".parse().unwrap();
    let srv = tokio::spawn(async move {
        hipcortex::web_server::run_with_state(addr, state).await;
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    let client = reqwest::Client::new();
    let base = "http://127.0.0.1:3061";

    // OpenAPI must advertise intelligence surface (Mac audit was counting openapi only)
    let openapi: serde_json::Value = client
        .get(format!("{}/openapi.json", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let paths = openapi["paths"].as_object().expect("paths object");
    for key in [
        "/self/health",
        "/self/can-execute",
        "/worldmodel/status",
        "/worldmodel/observe",
        "/worldmodel/predict",
        "/worldmodel/rollout",
        "/memory/live_beliefs",
        "/memory/link",
    ] {
        assert!(paths.contains_key(key), "openapi missing {}", key);
    }

    // Self surface
    let health: serde_json::Value = client
        .get(format!("{}/self/health", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(health.get("overall").is_some() || health.get("healthy").is_some());

    let gate: serde_json::Value = client
        .get(format!("{}/self/can-execute?operation=add_memory", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        gate.get("should_execute").is_some() || gate.get("error").is_some(),
        "can-execute shape: {}",
        gate
    );

    // Observe with alias body (state/next_state)
    let obs: serde_json::Value = client
        .post(format!("{}/worldmodel/observe", base))
        .json(&serde_json::json!({
            "state": "idle",
            "action": "start",
            "next_state": "running"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(obs["success"], true, "observe alias body: {}", obs);

    // Canonical from/action/to also works
    let obs2: serde_json::Value = client
        .post(format!("{}/worldmodel/observe", base))
        .json(&serde_json::json!({
            "from": "idle",
            "action": "start",
            "to": "running"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(obs2["success"], true);

    // Predict after observe
    let pred: serde_json::Value = client
        .post(format!("{}/worldmodel/predict", base))
        .json(&serde_json::json!({ "state": "idle", "action": "start" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        pred.get("error").is_none()
            || pred.get("distribution").is_some()
            || pred.get("predicted_state").is_some()
            || pred.get("probabilities").is_some(),
        "predict: {}",
        pred
    );
    // Prefer success path after observe
    assert!(
        pred.get("error").is_none(),
        "predict should succeed after observe: {}",
        pred
    );

    // Rollout multi-step (Dirichlet MAP — no trained predictor required)
    let roll: serde_json::Value = client
        .post(format!("{}/worldmodel/rollout", base))
        .json(&serde_json::json!({
            "initial_state": "idle",
            "actions": ["start"]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        roll.get("error").is_none(),
        "dirichlet rollout should succeed after observe: {}",
        roll
    );
    assert_eq!(roll["predicted_state"], "running");
    assert_eq!(roll["mode"], "dirichlet");

    // MCTS mode picks an action from observed transitions
    let mcts: serde_json::Value = client
        .post(format!("{}/worldmodel/rollout", base))
        .json(&serde_json::json!({
            "initial_state": "idle",
            "mode": "mcts",
            "iterations": 20,
            "max_depth": 2
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(mcts.get("best_action").is_some(), "mcts rollout: {}", mcts);

    // Live beliefs
    let beliefs: serde_json::Value = client
        .get(format!("{}/memory/live_beliefs", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(beliefs.get("intel").is_some() || beliefs.get("source").is_some());

    srv.abort();
}
