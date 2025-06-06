use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use uuid::Uuid;
use std::time::SystemTime;

#[test]
fn insert_and_recent() {
    let mut indexer = TemporalIndexer::new(3, 3600);
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "t",
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
    };
    indexer.insert(trace.clone());
    let recents = indexer.get_recent(1);
    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0].data, "t");
}
