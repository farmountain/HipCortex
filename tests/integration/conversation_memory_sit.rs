use hipcortex::conversation_memory::ConversationMemory;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;

#[test]
fn openmanus_round_trip_into_store() {
    let mut conv = ConversationMemory::new();
    conv.add_message("user", "hello");
    conv.add_message("assistant", "hi there");
    let path = "test_conv_store.jsonl";
    let _ = std::fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    for msg in conv.messages() {
        let rec = MemoryRecord::new(
            MemoryType::Symbolic,
            msg.sender.clone(),
            "says".into(),
            msg.text.clone(),
            serde_json::json!({}),
        );
        store.add(rec).unwrap();
    }
    assert_eq!(store.all().len(), 2);
    std::fs::remove_file(path).unwrap();
}
