use hipcortex::world_model_client::{MockWorldModelClient, WorldModelClient};

#[test]
fn mock_predicts_next_state() {
    let client = MockWorldModelClient;
    let result = client.predict_next_state("robot", "move");
    assert_eq!(result, "robot -> move");
}
