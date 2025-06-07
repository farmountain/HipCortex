use hipcortex::integration_layer::IntegrationLayer;

#[test]
fn integration_layer_connect() {
    let mut integration = IntegrationLayer::new();
    integration.connect();
}

#[test]
fn integration_layer_multiple_connects() {
    let mut integration = IntegrationLayer::new();
    integration.connect();
    integration.connect();
}

#[test]
fn integration_layer_send_disconnect() {
    let mut integration = IntegrationLayer::new();
    integration.connect();
    integration.add_api_key("k".into());
    integration.send_message_json("k", "{\"text\": \"hello\"}");
    integration.disconnect();
    integration.connect();
}

#[test]
fn oauth_authentication() {
    let mut integration = IntegrationLayer::new();
    integration.connect();
    integration.add_oauth_token("token".into());
    integration.send_message("token", "hi");
}

#[test]
fn integration_layer_is_connected() {
    let mut integration = IntegrationLayer::new();
    assert!(!integration.is_connected());
    integration.connect();
    assert!(integration.is_connected());
    integration.disconnect();
    assert!(!integration.is_connected());
}
