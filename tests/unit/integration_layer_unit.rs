use hipcortex::integration_layer::IntegrationLayer;

#[test]
fn basic_connect() {
    let mut layer = IntegrationLayer::new();
    assert!(!layer.is_connected());
    layer.connect();
    assert!(layer.is_connected());
}
