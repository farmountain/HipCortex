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
