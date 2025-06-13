use hipcortex::conversation_memory::ConversationMemory;
use hipcortex::integration_layer::IntegrationLayer;

#[test]
fn user_flow_conversation_memory() {
    let mut layer = IntegrationLayer::new();
    layer.connect();
    layer.add_api_key("k".into());
    let mut conv = ConversationMemory::new();
    conv.add_message("user", "Hello");
    conv.add_message("assistant", "Hi!");
    assert_eq!(conv.len(), 2);
    layer.send_message("k", "conversation complete");
    assert!(layer.is_connected());
}
