use hipcortex::memory_store::MemoryStore;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use std::fs;

#[test]
fn add_record() {
    let path = "unit_store.jsonl";
    let _ = fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let rec = MemoryRecord::new(MemoryType::Symbolic, "a".into(), "b".into(), "c".into(), serde_json::json!({}));
    store.add(rec.clone()).unwrap();
    assert_eq!(store.all().len(), 1);
    fs::remove_file(path).unwrap();
}
