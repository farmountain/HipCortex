//! SIT — POST /v1/backup creates a data-directory tar.gz snapshot.
//! cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite backup

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
                std::env::temp_dir().join("hc-backup-sit-archive.jsonl"),
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
    async fn backup_endpoint_returns_ok_key() {
        let addr: std::net::SocketAddr = "127.0.0.1:3100".parse().unwrap();
        let srv = tokio::spawn(async move {
            run_with_state(addr, make_state(None)).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3100/v1/backup")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body.get("ok").is_some(), "response must have 'ok' key");
        srv.abort();
    }

    #[tokio::test]
    async fn backup_creates_archive_file() {
        let uid = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("hc-backup-sit-{}", uid));
        let data_dir = root.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("worldmodel.json"), b"{\"version\":1}").unwrap();
        std::fs::write(data_dir.join("memory.jsonl"), b"").unwrap();

        let wm_path = data_dir.join("worldmodel.json").to_string_lossy().to_string();

        let addr: std::net::SocketAddr = "127.0.0.1:3101".parse().unwrap();
        let srv = tokio::spawn(async move {
            run_with_state(addr, make_state(Some(wm_path))).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3101/v1/backup")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(
            body["ok"].as_bool().unwrap_or(false),
            "backup must succeed: {:?}",
            body
        );
        let archive_path = body["path"].as_str().expect("path must be in response");
        assert!(
            std::path::Path::new(archive_path).exists(),
            "archive not found at: {}",
            archive_path
        );
        srv.abort();
        let _ = std::fs::remove_dir_all(&root);
    }
}
