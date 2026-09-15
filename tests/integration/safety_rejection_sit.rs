/// SIT: AC-SM-1 — SafetyClassifier 403 returns structured JSON, not bare body.
///
/// A raw 16-digit CI run ID matches the credit-card PII pattern
/// `\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b`.  The server must refuse
/// with HTTP 403 AND a JSON body `{"success":false,"error":"…"}` so MCP
/// clients can surface the reason instead of showing "server down".
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

    type TestState = AppState<InMemoryBackend>;

    fn make_test_state() -> TestState {
        let memory_store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
        let world_model = Arc::new(RwLock::new(WorldModelEnhanced::new()));
        let self_model = Arc::new(SelfModel::new());
        let coherence = Arc::new(CoherenceChecker::new());
        let calibration = Arc::new(CalibrationTracker::new());
        let cognitive = Arc::new(CognitiveHandle::new(
            memory_store.clone(),
            world_model.clone(),
            self_model.clone(),
            None,
            coherence.clone(),
            calibration.clone(),
            Arc::new(CognitiveGC::new()),
        ));
        AppState {
            memory_store,
            symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
            world_model,
            aureus: Arc::new(Mutex::new(AureusBridge::new())),
            self_model,
            coherence,
            topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
            archive_store: Arc::new(Mutex::new(ArchiveStore::new_in_memory())),
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

    // AC-SM-1: 16-digit number in target field → 403 with JSON body, not bare 403.
    #[tokio::test]
    async fn ci_run_id_refused_with_json_body() {
        let (base, srv) = start_test_server(make_test_state()).await;

        let resp = reqwest::Client::new()
            .post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": "ci-agent",
                "action": "ran",
                "target": "1234567890123456",
                "record_type": "Temporal"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 403, "16-digit CI run ID must be refused");
        let body: serde_json::Value = resp.json().await.expect("403 body must be valid JSON");
        assert_eq!(body["success"], false, "success must be false in 403 body");
        assert!(
            body["error"].as_str().unwrap_or_default().len() > 0,
            "403 body must contain non-empty error string, got: {body}"
        );

        srv.abort();
    }

    // AC-SM-2: prose with CI run ID embedded in sentence is also refused.
    #[tokio::test]
    async fn ci_run_id_in_prose_refused() {
        let (base, srv) = start_test_server(make_test_state()).await;

        let resp = reqwest::Client::new()
            .post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": "ci-agent",
                "action": "completed",
                "target": "build run 5432109876543210 finished",
                "record_type": "Temporal"
            }))
            .send()
            .await
            .unwrap();

        // Must be 403 with JSON, not 200 and not bare 403.
        assert_eq!(resp.status().as_u16(), 403);
        let body: serde_json::Value = resp.json().await.expect("403 body must be JSON");
        assert_eq!(body["success"], false);

        srv.abort();
    }

    // AC-SM-3: short numeric IDs (< 16 digits) pass through fine.
    #[tokio::test]
    async fn short_numeric_id_accepted() {
        let (base, srv) = start_test_server(make_test_state()).await;

        let resp = reqwest::Client::new()
            .post(format!("{}/memory/add", base))
            .json(&serde_json::json!({
                "actor": "ci-agent",
                "action": "ran",
                "target": "run-12345",
                "record_type": "Temporal"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(
            resp.status().as_u16(),
            200,
            "short numeric ID must not be refused"
        );

        srv.abort();
    }
}
