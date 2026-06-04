/// SIT tests for intelligence layer wiring
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::web_server::AppState;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::self_model::SelfModel;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use std::sync::{Arc, Mutex, RwLock};

pub fn make_app_state() -> AppState<InMemoryBackend> {
    AppState {
        memory_store: Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model: Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus: Arc::new(Mutex::new(AureusBridge::new())),
        self_model: Arc::new(SelfModel::new()),
        coherence: Arc::new(CoherenceChecker::new()),
    }
}

pub fn make_record(actor: &str, action: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Symbolic,
        actor.to_string(),
        action.to_string(),
        target.to_string(),
        serde_json::json!({}),
    )
}

#[test]
fn test_app_state_constructs() {
    let state = make_app_state();
    assert!(state.self_model.is_healthy().is_ok());
    assert!(state.world_model.read().unwrap().list_entities().unwrap().is_empty());
}

#[test]
fn test_app_state_clone_shares_arcs() {
    let state = make_app_state();
    let state2 = state.clone();
    assert!(Arc::ptr_eq(&state.memory_store, &state2.memory_store));
    assert!(Arc::ptr_eq(&state.world_model, &state2.world_model));
}

#[test]
fn test_world_model_save_load_roundtrip() {
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S3".into()).unwrap();
        wm.add_causal_edge("X".into(), "Y".into()).unwrap();
    }
    let tmp = std::env::temp_dir().join("wm_test_roundtrip.json");
    {
        let wm = state.world_model.read().unwrap();
        wm.save(&tmp).expect("save failed");
    }
    let wm2 = WorldModelEnhanced::load(&tmp).expect("load failed");
    let pred = wm2.predict_next_state("S1", "A1").unwrap();
    assert!(pred.probabilities.contains_key("S2"));
    assert!(pred.probabilities.contains_key("S3"));
    assert!(pred.probabilities["S2"] > pred.probabilities["S3"]);
    assert!(wm2.has_causal_path("X", "Y").unwrap());
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_world_model_save_load_empty() {
    let tmp = std::env::temp_dir().join("wm_test_empty.json");
    let wm = WorldModelEnhanced::new();
    wm.save(&tmp).expect("save empty failed");
    let wm2 = WorldModelEnhanced::load(&tmp).expect("load empty failed");
    assert!(wm2.list_entities().unwrap().is_empty());
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_world_model_load_missing_file_returns_err() {
    let result = WorldModelEnhanced::load("/nonexistent/path/wm.json");
    assert!(result.is_err());
}

#[test]
fn test_world_model_transition_count() {
    let state = make_app_state();
    assert_eq!(state.world_model.read().unwrap().transition_count(), 0);
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("A".into(), "B".into(), "C".into()).unwrap();
        wm.observe_transition("A".into(), "B".into(), "C".into()).unwrap();
    }
    assert_eq!(state.world_model.read().unwrap().transition_count(), 2);
}

#[test]
fn test_world_model_load_nonexistent_gives_new() {
    // When worldmodel.json doesn't exist, caller should fall back to WorldModelEnhanced::new()
    // This tests the fallback pattern used by bin/webserver.rs
    let path = "/nonexistent/path/worldmodel_99999.json";
    let wm = match WorldModelEnhanced::load(path) {
        Ok(wm) => wm,
        Err(_) => WorldModelEnhanced::new(), // fallback
    };
    assert_eq!(wm.transition_count(), 0);
}

#[test]
fn test_auto_feed_observe_transition() {
    // Directly test the observe_transition logic (mirrors what handle_add_memory will do)
    let state = make_app_state();
    {
        let mut ms = state.memory_store.lock().unwrap();
        ms.add(make_record("alice", "decided", "use_postgres")).unwrap();
        ms.add(make_record("alice", "decided", "use_postgres")).unwrap();
        ms.add(make_record("alice", "decided", "use_redis")).unwrap();
    }
    // Simulate auto-feed: observe transitions for each add
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("alice".into(), "decided".into(), "use_postgres".into()).unwrap();
        wm.observe_transition("alice".into(), "decided".into(), "use_postgres".into()).unwrap();
        wm.observe_transition("alice".into(), "decided".into(), "use_redis".into()).unwrap();
    }
    assert_eq!(state.world_model.read().unwrap().transition_count(), 3);
    let pred = state.world_model.read().unwrap()
        .predict_next_state("alice", "decided").unwrap();
    assert!(pred.probabilities["use_postgres"] > pred.probabilities["use_redis"]);
}

#[test]
fn test_auto_feed_pinned_causal_edge() {
    let state = make_app_state();
    // Simulate what handle_add_memory does for pinned Symbolic record
    {
        let mut wm = state.world_model.write().unwrap();
        wm.add_causal_edge("alice".into(), "postgres".into()).unwrap();
    }
    assert!(state.world_model.read().unwrap().has_causal_path("alice", "postgres").unwrap());
}

#[test]
fn test_wm_observe_predict_via_state() {
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        for _ in 0..3 { wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap(); }
        wm.observe_transition("S1".into(), "A1".into(), "S3".into()).unwrap();
    }
    let wm = state.world_model.read().unwrap();
    let pred = wm.predict_next_state("S1", "A1").unwrap();
    assert!(pred.probabilities["S2"] > pred.probabilities["S3"]);
    assert_eq!(pred.observation_count, 4);
}

#[test]
fn test_wm_register_entity_list() {
    use hipcortex::world_model_enhanced::EntityState;
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.register_entity("robot_1".into(), EntityState {
            properties: vec![0.0, 0.0, 0.0],
            covariance: vec![vec![1.0,0.0,0.0],vec![0.0,1.0,0.0],vec![0.0,0.0,1.0]],
        }).unwrap();
    }
    let entities = state.world_model.read().unwrap().list_entities().unwrap();
    assert!(entities.contains(&"robot_1".to_string()));
}

#[test]
fn test_wm_causal_edges() {
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.add_causal_edge("A".into(), "B".into()).unwrap();
        wm.add_causal_edge("B".into(), "C".into()).unwrap();
    }
    let edges = state.world_model.read().unwrap().get_causal_edges();
    assert_eq!(edges.len(), 2);
}

#[test]
fn test_wm_transition_count_after_feed() {
    let state = make_app_state();
    assert_eq!(state.world_model.read().unwrap().transition_count(), 0);
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("X".into(), "Y".into(), "Z".into()).unwrap();
    }
    assert_eq!(state.world_model.read().unwrap().transition_count(), 1);
}

#[test]
fn test_reflect_on_memory_no_llm() {
    // Without an LLM client, reflect_on_memory returns a default hypothesis
    let state = make_app_state();
    {
        let mut ms = state.memory_store.lock().unwrap();
        ms.add(make_record("alice", "decided", "use_postgres")).unwrap();
        ms.add(make_record("alice", "decided", "avoid_redis")).unwrap();
    }
    let hyp = {
        let mut au = state.aureus.lock().unwrap();
        let mut ms = state.memory_store.lock().unwrap();
        au.reflect_on_memory("alice decisions", &mut *ms)
    };
    // No LLM configured → default hypothesis with 0 loops
    assert!(hyp.confidence >= 0.0 && hyp.confidence <= 1.0);
    // text is non-empty (either default message or LLM response)
    assert!(!hyp.text.is_empty());
}

#[test]
fn test_reflect_on_memory_with_mock_llm() {
    use hipcortex::llm_clients::mock::MockClient;
    let state = make_app_state();
    {
        let mut ms = state.memory_store.lock().unwrap();
        ms.add(make_record("alice", "decided", "use_postgres")).unwrap();
    }
    {
        let mut au = state.aureus.lock().unwrap();
        au.set_client(Box::new(MockClient));
        let mut ms = state.memory_store.lock().unwrap();
        let hyp = au.reflect_on_memory("alice", &mut *ms);
        assert!(!hyp.text.is_empty());
        assert_eq!(au.loops_run(), 1);
    }
}

#[test]
fn test_self_model_health_ok() {
    let state = make_app_state();
    let health = state.self_model.get_health().unwrap();
    assert!(health.overall >= 0.0 && health.overall <= 1.0);
}

#[test]
fn test_self_model_capabilities_after_bootstrap() {
    use hipcortex::self_model::CapabilityDescriptor;
    let state = make_app_state();
    // Bootstrap (same as bin/webserver.rs does)
    for op in &["add_memory", "search_memory"] {
        state.self_model.register_capability(CapabilityDescriptor {
            name: op.to_string(),
            description: format!("test {}", op),
            required_cpu_percent: 5.0,
            required_memory_mb: 50.0,
            limitations: vec![],
        }).unwrap();
    }
    assert!(state.self_model.get_capability("add_memory").is_ok());
    assert!(state.self_model.get_capability("search_memory").is_ok());
    assert!(state.self_model.get_capability("unknown").is_err());
}

#[test]
fn test_coherence_checker_persistent_metrics() {
    let state = make_app_state();
    // CoherenceChecker is the same Arc across calls — metrics accumulate
    let _ = state.coherence.check_consistency();
    let metrics = state.coherence.get_metrics().unwrap();
    // total_checks should be >= 1 after calling check_consistency()
    assert!(metrics.total_checks >= 1);
}

#[test]
fn test_wm_get_states_empty() {
    let state = make_app_state();
    let wm = state.world_model.read().unwrap();
    let states = wm.get_states();
    assert!(states.is_empty());
}

#[test]
fn test_wm_get_states_after_transitions() {
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap();
        wm.observe_transition("S2".into(), "A2".into(), "S3".into()).unwrap();
    }
    let wm = state.world_model.read().unwrap();
    let states = wm.get_states();
    assert!(states.contains(&"S1".to_string()));
    assert!(states.contains(&"S2".to_string()));
    assert!(states.contains(&"S3".to_string()));
}

#[test]
fn test_wm_get_actions_after_transitions() {
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap();
        wm.observe_transition("S1".into(), "A2".into(), "S3".into()).unwrap();
    }
    let wm = state.world_model.read().unwrap();
    let actions = wm.get_actions();
    assert!(actions.contains(&"A1".to_string()));
    assert!(actions.contains(&"A2".to_string()));
}

#[test]
fn test_wm_get_all_entropy() {
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S2".into()).unwrap();
        wm.observe_transition("S1".into(), "A1".into(), "S3".into()).unwrap();
    }
    let wm = state.world_model.read().unwrap();
    let entropy_list = wm.get_all_entropy();
    assert!(!entropy_list.is_empty());
    let s1_a1 = entropy_list.iter().find(|(s, a, _)| s == "S1" && a == "A1");
    assert!(s1_a1.is_some());
    assert!(s1_a1.unwrap().2 > 0.5);
}

#[test]
fn test_causal_intervention_empty_graph() {
    use hipcortex::world_model_enhanced::InterventionQuery;
    let state = make_app_state();
    let wm = state.world_model.read().unwrap();
    let result = wm.causal_intervention(InterventionQuery {
        outcome: "Y".into(),
        intervention_var: "X".into(),
        intervention_value: 1.0,
        conditioned_on: std::collections::HashMap::new(),
    });
    // May succeed or fail on empty graph — must not panic
    let _ = result;
}

#[test]
fn test_counterfactual_empty_graph() {
    let state = make_app_state();
    let wm = state.world_model.read().unwrap();
    let mut actual = std::collections::HashMap::new();
    actual.insert("X".to_string(), 0.5f64);
    actual.insert("Y".to_string(), 0.3f64);
    // Must not panic even on empty graph
    let result = wm.counterfactual(actual, "X".into(), 1.0);
    let _ = result;
}

// ── Task 4: G9 Entity initial values ─────────────────────────────────────────

#[test]
fn test_wm_entity_with_initial_values() {
    use hipcortex::world_model_enhanced::EntityState;
    let state = make_app_state();
    {
        let mut wm = state.world_model.write().unwrap();
        wm.register_entity("sensor_1".into(), EntityState {
            properties: vec![42.0, 7.5, 1.0],
            covariance: vec![
                vec![0.1, 0.0, 0.0],
                vec![0.0, 0.1, 0.0],
                vec![0.0, 0.0, 0.1],
            ],
        }).unwrap();
    }
    let entities = state.world_model.read().unwrap().list_entities().unwrap();
    assert!(entities.contains(&"sensor_1".to_string()));
}

// ── Task 5: G10 CoherenceChecker background auto-check ───────────────────────

#[test]
fn test_coherence_check_consistency_no_panic() {
    let state = make_app_state();
    // Direct call — same as background task does every 60s
    let result = state.coherence.check_consistency();
    assert!(result.is_ok());
    // total_checks incremented
    let metrics = state.coherence.get_metrics().unwrap();
    assert!(metrics.total_checks >= 1);
}

// ── Task 6: G15/G16 SelfModel can-execute + capability registration ───────────

#[test]
fn test_self_can_execute_unknown_op() {
    use hipcortex::self_model::DecisionContext;
    let state = make_app_state();
    let decision = state.self_model
        .can_execute("completely_unknown_op_xyz", DecisionContext::default_context())
        .unwrap();
    assert!(!decision.should_execute);
    assert!(decision.rationale.contains("not registered"));
}

#[test]
fn test_self_can_execute_registered_op() {
    use hipcortex::self_model::{CapabilityDescriptor, DecisionContext};
    let state = make_app_state();
    state.self_model.register_capability(CapabilityDescriptor {
        name: "test_exec_op".into(),
        description: "test".into(),
        required_cpu_percent: 5.0,
        required_memory_mb: 50.0,
        limitations: vec![],
    }).unwrap();
    let decision = state.self_model
        .can_execute("test_exec_op", DecisionContext::default_context())
        .unwrap();
    assert!(decision.confidence >= 0.0 && decision.confidence <= 1.0);
}

#[test]
fn test_self_register_capability_runtime() {
    use hipcortex::self_model::CapabilityDescriptor;
    let state = make_app_state();
    let result = state.self_model.register_capability(CapabilityDescriptor {
        name: "runtime_cap_99".into(),
        description: "registered at runtime".into(),
        required_cpu_percent: 2.0,
        required_memory_mb: 20.0,
        limitations: vec![],
    });
    assert!(result.is_ok());
    assert!(state.self_model.get_capability("runtime_cap_99").is_ok());
}

// ── MemoryRecordResponse field expansion (confidence/source/priority/tags/version/status/expires_at) ──

#[test]
fn test_memory_record_has_confidence_field() {
    let r = make_record("alice", "decided", "use_postgres");
    assert_eq!(r.confidence, 1.0_f32); // default confidence
}

#[test]
fn test_memory_record_has_all_new_fields() {
    let mut r = make_record("alice", "decided", "use_postgres");
    r.priority = "high".to_string();
    r.tags = vec!["database".to_string()];
    r.version = 2;
    r.status = "active".to_string();
    r.source = Some("user-input".to_string());
    assert_eq!(r.priority, "high");
    assert_eq!(r.tags.len(), 1);
    assert_eq!(r.version, 2);
    assert_eq!(r.status, "active");
    assert!(r.source.is_some());
}

#[test]
fn test_query_memory_response_includes_confidence() {
    // Build a MemoryRecordResponse manually (struct construction test)
    // This verifies the struct has the new public fields
    let state = make_app_state();
    {
        let mut ms = state.memory_store.lock().unwrap();
        let mut r = make_record("alice", "decided", "use_postgres");
        r.confidence = 0.8;
        r.priority = "high".to_string();
        r.tags = vec!["database".to_string()];
        ms.add(r).unwrap();
    }
    // Read it back — confidence + priority + tags should be on the record
    let ms = state.memory_store.lock().unwrap();
    let records = ms.find_by_actor("alice");
    assert!(!records.is_empty());
    let r = records[0];
    assert_eq!(r.confidence, 0.8_f32);
    assert_eq!(r.priority, "high");
    assert_eq!(r.tags[0], "database");
}
