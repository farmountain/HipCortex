use hipcortex::semantic_compression::compress_embedding;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;

#[test]
fn store_compressed_embedding() {
    let path = "compress_uat.jsonl";
    let _ = std::fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let embedding: Vec<f32> = (0..8).map(|v| v as f32).collect();
    let compressed = compress_embedding(&embedding, 4);
    let record = MemoryRecord::new(
        MemoryType::Perception,
        "user".into(),
        "embedding".into(),
        "vec".into(),
        serde_json::json!({"embedding": compressed}),
    );
    store.add(record).unwrap();
    assert_eq!(store.all().len(), 1);
    std::fs::remove_file(path).unwrap();
}
