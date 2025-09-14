use hipcortex::procedural_cache::{FSMState, ProceduralCache, ProceduralTrace};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn add_and_retrieve_map() {
    let mut cache = ProceduralCache::new();
    let id = Uuid::new_v4();
    cache.add_trace(ProceduralTrace {
        id,
        current_state: FSMState::Start,
        memory: HashMap::new(),
    });
    cache.add_map_version(id, json!({"s":1}), 0.9);
    assert!(cache.latest_map(id).is_some());
}
