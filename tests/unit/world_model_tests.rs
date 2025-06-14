use hipcortex::world_model::WorldModel;
use serde_json::json;

#[test]
fn add_and_export() {
    let wm = WorldModel::new();
    wm.add_state("room", json!({"size": "large"}));
    let graph = wm.export();
    assert!(!graph.0.is_empty());
}
