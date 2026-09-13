//! SIT — POST /v1/server/shutdown flushes worldmodel.json before exit.
//! cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite graceful_shutdown

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

    fn make_state(wm_path: Option<String>) -> AppState<InMemoryBackend> {
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
                std::env::temp_dir().join("hc-shutdown-test-archive.jsonl"),
            ))),
            tx_log: None,
            calibration,
            cognitive,
            forks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            twins: Arc::new(Mutex::new(std::collections::HashMap::new())),
            daemon: Arc::new(Mutex::new(SubstrateDaemon::new())),
            workspace_registry: Arc::new(Mutex::new(WorkspaceRegistry::new())),
            passive_capture_enabled: false,
            wm_path: wm_path.map(Arc::new),
        }
    }

    #[tokio::test]
    async fn shutdown_returns_200_with_status() {
        let addr: std::net::SocketAddr = "127.0.0.1:3098".parse().unwrap();
        let srv = tokio::spawn(async move {
            run_with_state(addr, make_state(None)).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3098/v1/server/shutdown")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"], "shutting down");
        srv.abort();
    }

    #[tokio::test]
    async fn shutdown_saves_worldmodel_json() {
        let wm_path = std::env::temp_dir().join(format!(
            "hipcortex-shutdown-test-{}.json",
            uuid::Uuid::new_v4()
        ));
        let wm_path_str = wm_path.to_string_lossy().to_string();

        let addr: std::net::SocketAddr = "127.0.0.1:3099".parse().unwrap();
        let srv = tokio::spawn(async move {
            run_with_state(addr, make_state(Some(wm_path_str))).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3099/v1/server/shutdown")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);

        srv.abort();

        assert!(wm_path.exists(), "worldmodel.json not saved on shutdown");
        let _ = std::fs::remove_file(&wm_path);
    }
}
