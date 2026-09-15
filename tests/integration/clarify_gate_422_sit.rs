/// SIT: AC-CG-1 — POST /goal/:id/react with empty success_factors returns
/// 422 UNPROCESSABLE_ENTITY with clarify_questions array and clarify_endpoint,
/// not a silent 200 or bare error string.
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
        payloads::{GoalPayload, GoalStatus},
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
    use uuid::Uuid;

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

    fn insert_goal_with_empty_factors(store: &StoreRef, actor: &str) -> Uuid {
        let payload = GoalPayload {
            target_state: "test goal with no success factors".to_string(),
            success_factors: vec![],
            status: GoalStatus::Pending,
            ..Default::default()
        };
        let mut rec = MemoryRecord::new(
            MemoryType::Goal,
            actor.to_string(),
            "define".to_string(),
            "test goal".to_string(),
            serde_json::to_value(&payload).unwrap(),
        );
        let id = rec.id;
        store.lock().unwrap().add(rec);
        id
    }

    // AC-CG-1a: empty success_factors → 422 with clarify_questions array.
    #[tokio::test]
    async fn react_on_unclarified_goal_returns_422_with_questions() {
        let (state, store) = make_test_state();
        let goal_id = insert_goal_with_empty_factors(&store, "cg-actor");
        let (base, srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .post(format!("{}/goal/{}/react", base, goal_id))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        assert_eq!(
            resp.status().as_u16(),
            422,
            "empty success_factors must yield 422, not 200"
        );
        let body: serde_json::Value = resp.json().await.expect("422 body must be JSON");

        assert!(
            body["clarify_questions"].is_array(),
            "clarify_questions must be array, got: {body}"
        );
        assert!(
            body["clarify_questions"].as_array().unwrap().len() >= 1,
            "clarify_questions must have at least one question"
        );
        assert!(
            body["clarify_endpoint"]
                .as_str()
                .unwrap_or_default()
                .contains(&goal_id.to_string()),
            "clarify_endpoint must contain goal_id"
        );
        assert!(
            body["error"].as_str().unwrap_or_default().len() > 0,
            "error field must be non-empty"
        );
        assert_eq!(
            body["goal_id"].as_str().unwrap_or_default(),
            goal_id.to_string(),
            "goal_id must echo back"
        );

        srv.abort();
    }

    // AC-CG-1b: hint field present for self-prompt guidance.
    #[tokio::test]
    async fn react_422_includes_self_prompt_hint() {
        let (state, store) = make_test_state();
        let goal_id = insert_goal_with_empty_factors(&store, "cg-actor2");
        let (base, srv) = start_test_server(state).await;

        let resp = reqwest::Client::new()
            .post(format!("{}/goal/{}/react", base, goal_id))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 422);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(
            body["hint"].as_str().unwrap_or_default().len() > 0,
            "hint field must be present for self-prompt guidance, got: {body}"
        );

        srv.abort();
    }

    // AC-CG-1c: react on goal not found → 404, not 422.
    #[tokio::test]
    async fn react_on_missing_goal_returns_404() {
        let (state, _store) = make_test_state();
        let (base, srv) = start_test_server(state).await;
        let missing_id = Uuid::new_v4();

        let resp = reqwest::Client::new()
            .post(format!("{}/goal/{}/react", base, missing_id))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        assert_ne!(
            resp.status().as_u16(),
            200,
            "missing goal must not silently return 200"
        );
        assert_ne!(
            resp.status().as_u16(),
            422,
            "missing goal must not return 422 (clarify gate only fires when goal exists but lacks success_factors)"
        );

        srv.abort();
    }
}
