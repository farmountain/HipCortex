use hipcortex::memory_processor::MemoryProcessor;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use uuid::Uuid;

#[test]
fn deduplicate_records() {
    let mut recs = vec![
        MemoryRecord { id: Uuid::new_v4(), record_type: MemoryType::Symbolic, timestamp: chrono::Utc::now(), actor: "a".into(), action: "a".into(), target: "t".into(), metadata: serde_json::json!({}) },
    ];
    recs.push(recs[0].clone());
    MemoryProcessor::deduplicate(&mut recs);
    assert_eq!(recs.len(), 1);
}
