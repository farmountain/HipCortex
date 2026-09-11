/// SIT: Server-side passive capture middleware.
/// Verifies every successful mutation writes a Temporal record
/// regardless of which channel sent the request.
///
/// Uses reqwest + real TCP server to avoid axum-test/axum-0.6 version skew.
/// Uses `AppState.passive_capture_enabled` (not env var) to avoid inter-test races.
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
        symbolic_store::SymbolicStore,
        topological_memory::CausalTopoGraph,
        web_server::{build_app, AppState},
        workspace::WorkspaceRegistry,
        world_model_enhanced::WorldModelEnhanced,
    };
    use std::sync::{Arc, Mutex, RwLock};

    type Store = Arc<Mutex<MemoryStore<InMemoryBackend>>>;
    type TestState = AppState<InMemoryBackend>;

    fn make_test_state(passive_enabled: bool) -> (TestState, Store) {
        let memory_store: Store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
        let store_ref = memory_store.clone();
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
        let archive_path = std::env::temp_dir()
            .join(format!("hc-passive-test-{}.jsonl", uuid::Uuid::new_v4()));
        let state = AppState {
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
            daemon: Arc::new(Mutex::new(SubstrateDaemon::new())),
            workspace_registry: Arc::new(Mutex::new(WorkspaceRegistry::new())),
            passive_capture_enabled: passive_enabled,
        };
        (state, store_ref)
    }

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
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        (format!("http://127.0.0.1:{}", addr.port()), handle)
    }

    #[tokio::test]
    async fn test_capture_fires_on_add_memory() {
        let (state, store) = make_test_state(true);
        let (base, srv) = start_test_server(state).await;

        reqwest::Client::new()
            .post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": "test-actor",
                "action": "wrote",
                "target": "test.rs",
                "record_type": "Temporal"
            }))
            .header("x-actor", "mcp-test")
            .send()
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        srv.abort();

        let guard = store.lock().unwrap();
        let capture = guard
            .all()
            .iter()
            .find(|r| r.source.as_deref() == Some("server-passive-capture"));

        assert!(capture.is_some(), "passive capture record must exist after POST /memory/add");
        let cap = capture.unwrap();
        assert_eq!(cap.actor, "mcp-test", "actor must come from X-Actor header");
        assert!(
            cap.tags.contains(&"server-passive-capture".to_string()),
            "capture must be tagged server-passive-capture"
        );
    }

    #[tokio::test]
    async fn test_no_capture_on_get() {
        let (state, store) = make_test_state(true);
        let (base, srv) = start_test_server(state).await;

        reqwest::Client::new()
            .get(format!("{}/health", base))
            .send()
            .await
            .ok();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        srv.abort();

        let guard = store.lock().unwrap();
        let captures: Vec<_> = guard
            .all()
            .iter()
            .filter(|r| r.source.as_deref() == Some("server-passive-capture"))
            .collect();
        assert!(captures.is_empty(), "GET must never produce a capture record");
    }

    #[tokio::test]
    async fn test_no_capture_when_disabled() {
        let (state, store) = make_test_state(false);
        let (base, srv) = start_test_server(state).await;

        reqwest::Client::new()
            .post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": "test-actor",
                "action": "wrote",
                "target": "test.rs",
                "record_type": "Temporal"
            }))
            .send()
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        srv.abort();

        let guard = store.lock().unwrap();
        let captures: Vec<_> = guard
            .all()
            .iter()
            .filter(|r| r.source.as_deref() == Some("server-passive-capture"))
            .collect();
        assert!(
            captures.is_empty(),
            "passive_capture_enabled=false must suppress all captures"
        );
    }

    #[tokio::test]
    async fn test_unknown_channel_actor_when_no_header() {
        let (state, store) = make_test_state(true);
        let (base, srv) = start_test_server(state).await;

        reqwest::Client::new()
            .post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": "test-actor",
                "action": "wrote",
                "target": "test.rs",
                "record_type": "Temporal"
            }))
            .send()
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        srv.abort();

        let guard = store.lock().unwrap();
        let capture = guard
            .all()
            .iter()
            .find(|r| r.source.as_deref() == Some("server-passive-capture"));

        if let Some(cap) = capture {
            assert_eq!(
                cap.actor, "unknown-channel",
                "actor must default to unknown-channel when X-Actor header absent"
            );
        }
    }
}
