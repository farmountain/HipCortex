use hipcortex::archive_store::ArchiveStore;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use tempfile::tempdir;

#[test]
fn test_archive_store_append_and_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("archive.jsonl");

    let mut store = ArchiveStore::new(&path);
    let record = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "b".into(),
        "c".into(),
        serde_json::json!({}),
    );
    let id = record.id;
    store.append(record).unwrap();

    let loaded = store.load_all().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, id);
}

#[test]
fn test_archive_store_empty_file_returns_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty_archive.jsonl");
    let store = ArchiveStore::new(&path);
    let loaded = store.load_all().unwrap();
    assert!(loaded.is_empty());
}
