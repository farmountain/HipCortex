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
