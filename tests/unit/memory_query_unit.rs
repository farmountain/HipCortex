use hipcortex::memory_query::MemoryQuery;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use uuid::Uuid;

#[test]
fn query_by_type() {
    let rec = MemoryRecord { id: Uuid::new_v4(), record_type: MemoryType::Temporal, timestamp: chrono::Utc::now(), actor: "a".into(), action: "a".into(), target: "t".into(), metadata: serde_json::json!({}) };
    let res = MemoryQuery::by_type(&[rec.clone()], MemoryType::Temporal);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, rec.id);
}
