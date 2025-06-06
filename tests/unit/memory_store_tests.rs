use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use std::fs;

#[test]
fn test_add_and_query_memory_store() {
    let path = "test_memory.jsonl";
    let _ = fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let record = MemoryRecord::new(MemoryType::Symbolic, "user".into(), "is".into(), "tester".into(), serde_json::json!({}));
    store.add(record.clone()).unwrap();
    let all = store.all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].actor, "user");
    fs::remove_file(path).unwrap();
}
