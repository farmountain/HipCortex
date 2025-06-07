use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::snapshot_manager::SnapshotManager;
use std::fs;

#[test]
fn test_snapshot_save_and_load() {
    let path = "snap_memory.jsonl";
    let _ = fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let record = MemoryRecord::new(
        MemoryType::Temporal,
        "agent".into(),
        "do".into(),
        "something".into(),
        serde_json::json!({}),
    );
    store.add(record.clone()).unwrap();
    let archive = SnapshotManager::save(path, "testsnap").unwrap();
    fs::remove_file(path).unwrap();
    SnapshotManager::load(&archive, ".").unwrap();
    let store2 = MemoryStore::new(path).unwrap();
    assert_eq!(store2.all().len(), 1);
    fs::remove_file(path).unwrap();
    fs::remove_file(archive).unwrap();
}
