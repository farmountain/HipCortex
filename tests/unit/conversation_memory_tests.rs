use hipcortex::conversation_memory::ConversationMemory;

#[test]
fn add_and_len() {
    let mut cm = ConversationMemory::new();
    cm.add_message("user", "hello");
    assert_eq!(cm.len(), 1);
    assert_eq!(cm.messages()[0].text, "hello");
}

#[test]
fn ingest_openmanus_json() {
    let mut cm = ConversationMemory::new();
    let json = r#"{"role":"assistant","content":"hi"}"#;
    cm.ingest_openmanus(json).unwrap();
    assert_eq!(cm.len(), 1);
    assert_eq!(cm.messages()[0].sender, "assistant");
}
