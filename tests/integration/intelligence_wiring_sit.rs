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
