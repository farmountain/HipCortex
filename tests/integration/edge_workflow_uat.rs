use hipcortex::integration_layer::IntegrationLayer;

#[test]
fn user_runs_edge_workflow() {
    let mut layer = IntegrationLayer::new();
    layer.connect();
    layer.add_api_key("k".into());
    layer.send_message("k", "workflow complete");
    assert!(layer.is_connected());
}
