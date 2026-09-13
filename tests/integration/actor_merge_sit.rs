//! SIT — POST /v1/actor/merge moves all records source→target (MOVE semantics).
//! cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite actor_merge

#[cfg(feature = "web-server")]
mod tests {
    use hipcortex::{
        archive_store::ArchiveStore,
        aureus_bridge::AureusBridge,
        cognitive_gc::CognitiveGC,
        cognitive_state::CognitiveHandle,
        coherence::CoherenceChecker,
        memory_store::MemoryStore,
        persistence::InMemoryBackend,
        self_model::{calibration::CalibrationTracker, SelfModel},
        substrate_daemon::SubstrateDaemon,
        symbolic_store::{InMemoryGraph, SymbolicStore},
        topological_memory::CausalTopoGraph,
        web_server::{run_with_state, AppState},
        workspace::WorkspaceRegistry,
        world_model_enhanced::WorldModelEnhanced,
    };
    use std::sync::{Arc, Mutex, RwLock};

    fn make_state() -> AppState<InMemoryBackend> {
        let memory_store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
        let world_model = Arc::new(RwLock::new(WorldModelEnhanced::new()));
        let self_model = Arc::new(SelfModel::new());
        let coherence = Arc::new(CoherenceChecker::new());
        let calibration = Arc::new(CalibrationTracker::new());
        let cognitive = Arc::new(CognitiveHandle::new(
            Arc::clone(&memory_store),
            Arc::clone(&world_model),
            Arc::clone(&self_model),
            None,
            Arc::clone(&coherence),
            Arc::clone(&calibration),
            Arc::new(CognitiveGC::new()),
        ));
        AppState {
            memory_store,
            symbolic_store: Arc::new(Mutex::new(SymbolicStore::<InMemoryGraph>::new())),
            world_model,
            aureus: Arc::new(Mutex::new(AureusBridge::new())),
            self_model,
            coherence,
            topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
            archive_store: Arc::new(Mutex::new(ArchiveStore::new(
                std::env::temp_dir().join("hc-actor-merge-sit-archive.jsonl"),
            ))),
            tx_log: None,
            calibration,
            cognitive,
            forks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            twins: Arc::new(Mutex::new(std::collections::HashMap::new())),
            daemon: Arc::new(Mutex::new(SubstrateDaemon::new())),
            workspace_registry: Arc::new(Mutex::new(WorkspaceRegistry::new())),
            passive_capture_enabled: false,
            wm_path: None,
        }
    }

    #[tokio::test]
    async fn merge_moves_records_and_removes_source() {
        let addr: std::net::SocketAddr = "127.0.0.1:3102".parse().unwrap();
        let srv = tokio::spawn(async move {
            run_with_state(addr, make_state()).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        let client = reqwest::Client::new();
        let base = "http://127.0.0.1:3102";

        // Add 2 records for "alice"
        for i in 0..2 {
            client
                .post(&format!("{}/memory/add", base))
                .json(&serde_json::json!({"actor":"alice","action":"did","target":format!("thing-{}",i)}))
                .send().await.unwrap();
        }
        // Add 1 record for "bob"
        client
            .post(&format!("{}/memory/add", base))
            .json(&serde_json::json!({"actor":"bob","action":"did","target":"bob-thing"}))
            .send().await.unwrap();

        // Merge alice → bob
        let merge: serde_json::Value = client
            .post(&format!("{}/v1/actor/merge", base))
            .json(&serde_json::json!({"source":"alice","target":"bob"}))
            .send().await.unwrap()
            .json().await.unwrap();

        assert!(merge["ok"].as_bool().unwrap_or(false), "merge must succeed: {:?}", merge);
        assert_eq!(merge["moved"].as_u64().unwrap_or(0), 2, "must move exactly 2 alice records");

        // Query: all records should belong to "bob", none to "alice"
        let q: serde_json::Value = client
            .get(&format!("{}/memory/query?limit=20", base))
            .send().await.unwrap()
            .json().await.unwrap();
        let records = q["records"].as_array().unwrap();
        let alice_count = records.iter().filter(|r| r["actor"] == "alice").count();
        let bob_count = records.iter().filter(|r| r["actor"] == "bob").count();
        assert_eq!(alice_count, 0, "source actor must have 0 records after merge");
        assert_eq!(bob_count, 3, "target actor must have 3 records (2 moved + 1 original)");

        srv.abort();
    }

    #[tokio::test]
    async fn merge_same_actor_is_noop() {
        let addr: std::net::SocketAddr = "127.0.0.1:3103".parse().unwrap();
        let srv = tokio::spawn(async move {
            run_with_state(addr, make_state()).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let resp: serde_json::Value = reqwest::Client::new()
            .post("http://127.0.0.1:3103/v1/actor/merge")
            .json(&serde_json::json!({"source":"alice","target":"alice"}))
            .send().await.unwrap()
            .json().await.unwrap();

        assert!(resp["ok"].as_bool().unwrap_or(false));
        assert_eq!(resp["moved"].as_u64().unwrap_or(99), 0);
        srv.abort();
    }

    #[tokio::test]
    async fn merge_rejects_empty_actor() {
        let addr: std::net::SocketAddr = "127.0.0.1:3104".parse().unwrap();
        let srv = tokio::spawn(async move {
            run_with_state(addr, make_state()).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let resp: serde_json::Value = reqwest::Client::new()
            .post("http://127.0.0.1:3104/v1/actor/merge")
            .json(&serde_json::json!({"source":"","target":"bob"}))
            .send().await.unwrap()
            .json().await.unwrap();

        assert!(!resp["ok"].as_bool().unwrap_or(true), "empty source must fail");
        assert!(resp.get("error").is_some());
        srv.abort();
    }
}
