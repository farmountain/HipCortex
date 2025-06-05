use hipcortex::integration_layer::IntegrationLayer;

#[test]
fn integration_layer_connect() {
    let integration = IntegrationLayer::new();
    integration.connect();
}

#[test]
fn integration_layer_multiple_connects() {
    let integration = IntegrationLayer::new();
    integration.connect();
    integration.connect();
}
