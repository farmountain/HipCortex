use hipcortex::world_model::WorldModel;
use serde_json::json;

fn main() {
    let model = WorldModel::new();
    model.add_state("robot", json!({"status": "idle"}));
    let graph = model.export();
    println!("nodes: {}", graph.0.len());
}
