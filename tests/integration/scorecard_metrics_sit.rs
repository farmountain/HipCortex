/// SIT: AC-SC-1 — Scorecard live block exposes distinct api_mutations_captured
/// and env_receipts fields so passive-capture audit is never conflated with
/// env grounding.
#[cfg(feature = "web-server")]
mod tests {
    use hipcortex::{
        archive_store::ArchiveStore,
        aureus_bridge::AureusBridge,
        cognitive_gc::CognitiveGC,
        cognitive_state::CognitiveHandle,
        coherence::CoherenceChecker,
        memory_record::{MemoryRecord, MemoryType},
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
    type StoreRef = Arc<Mutex<MemoryStore<InMemoryBackend>>>;

    fn make_test_state() -> (TestState, StoreRef) {
        let memory_store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
        let store_ref = memory_store.clone();
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
        let state = AppState {
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

    // AC-SC-1a: fields present and numeric even with empty store.
    #[tokio::test]
    async fn scorecard_live_has_metric_fields() {
        let (state, _store) = make_test_state();
        let (base, srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .get(format!("{}/substrate/scorecard?actor=sc-test", base))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        let live = &body["live"];
        assert!(live.is_object(), "live block must be present");
        assert!(
            live["api_mutations_captured"].is_number(),
            "api_mutations_captured must be numeric, got: {live}"
        );
        assert!(
            live["env_receipts"].is_number(),
            "env_receipts must be numeric, got: {live}"
        );

        srv.abort();
    }

    // AC-SC-1b: api_mutations_captured counts server-passive-capture records for actor.
    #[tokio::test]
    async fn api_mutations_captured_counts_passive_records() {
        let (state, store) = make_test_state();

        {
            let mut ms = store.lock().unwrap();
            let mut r1 = MemoryRecord::new(
                MemoryType::Temporal,
                "sc-actor".to_string(),
                "POST /memory/add".to_string(),
                "record stored".to_string(),
                serde_json::Value::Null,
            );
            r1.source = Some("server-passive-capture".to_string());
            ms.add(r1);

            let mut r2 = MemoryRecord::new(
                MemoryType::Temporal,
                "sc-actor".to_string(),
                "PUT /memory/update".to_string(),
                "record updated".to_string(),
                serde_json::Value::Null,
            );
            r2.source = Some("server-passive-capture".to_string());
            ms.add(r2);

            // different actor — must not count toward sc-actor
            let mut r3 = MemoryRecord::new(
                MemoryType::Temporal,
                "other-actor".to_string(),
                "POST /memory/add".to_string(),
                "should not count".to_string(),
                serde_json::Value::Null,
            );
            r3.source = Some("server-passive-capture".to_string());
            ms.add(r3);
        }

        let (base, srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .get(format!("{}/substrate/scorecard?actor=sc-actor", base))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(
            body["live"]["api_mutations_captured"], 2,
            "must count 2 passive-capture records for sc-actor"
        );

        srv.abort();
    }

    // AC-SC-1c: env_receipts counts action-contains-receipt records for actor.
    #[tokio::test]
    async fn env_receipts_counts_receipt_actions() {
        let (state, store) = make_test_state();

        {
            let mut ms = store.lock().unwrap();
            for i in 0..3 {
                ms.add(MemoryRecord::new(
                    MemoryType::Temporal,
                    "env-actor".to_string(),
                    format!("receipt_{}", i),
                    "env observation".to_string(),
                    serde_json::Value::Null,
                ));
            }
            ms.add(MemoryRecord::new(
                MemoryType::Temporal,
                "env-actor".to_string(),
                "observed".to_string(),
                "something".to_string(),
                serde_json::Value::Null,
            ));
            ms.add(MemoryRecord::new(
                MemoryType::Temporal,
                "other-actor".to_string(),
                "receipt_x".to_string(),
                "other env".to_string(),
                serde_json::Value::Null,
            ));
        }

        let (base, srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .get(format!("{}/substrate/scorecard?actor=env-actor", base))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(
            body["live"]["env_receipts"], 3,
            "must count 3 receipt records for env-actor"
        );

        srv.abort();
    }

    // AC-SC-1d: fields are independent — passive capture != env receipts.
    #[tokio::test]
    async fn passive_capture_and_env_receipts_are_independent() {
        let (state, store) = make_test_state();

        {
            let mut ms = store.lock().unwrap();
            for _ in 0..2 {
                let mut r = MemoryRecord::new(
                    MemoryType::Temporal,
                    "mix-actor".to_string(),
                    "POST /memory/add".to_string(),
                    "captured".to_string(),
                    serde_json::Value::Null,
                );
                r.source = Some("server-passive-capture".to_string());
                ms.add(r);
            }
            ms.add(MemoryRecord::new(
                MemoryType::Temporal,
                "mix-actor".to_string(),
                "receipt_env_check".to_string(),
                "env grounding".to_string(),
                serde_json::Value::Null,
            ));
        }

        let (base, srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .get(format!("{}/substrate/scorecard?actor=mix-actor", base))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["live"]["api_mutations_captured"], 2);
        assert_eq!(body["live"]["env_receipts"], 1);

        srv.abort();
    }
}
