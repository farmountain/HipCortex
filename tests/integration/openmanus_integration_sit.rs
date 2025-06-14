use hipcortex::conversation_memory::ConversationMemory;
use hipcortex::integration_layer::IntegrationLayer;
use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};

#[test]
fn openmanus_message_through_adapter_and_layer() {
    let json = r#"{"role":"user","content":"ping"}"#;
    let mut conv = ConversationMemory::new();
    conv.ingest_openmanus(json).unwrap();
    let msg = conv.messages().last().unwrap().clone();

    let input = PerceptInput {
        modality: Modality::AgentMessage,
        text: Some(msg.text.clone()),
        embedding: None,
        image_data: None,
        tags: vec!["sit".into()],
    };
    let out = PerceptionAdapter::adapt(input);
    assert!(out.is_none());

    let mut layer = IntegrationLayer::new();
    layer.connect();
    layer.add_api_key("k".into());
    layer.send_message("k", &msg.text);
    assert!(layer.is_connected());
}
