use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::decay::DecayType;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[test]
fn insert_and_get_recent() {
    let mut indexer = TemporalIndexer::new(1, 10);
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "test",
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
        decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
    };
    indexer.insert(trace);
    assert_eq!(indexer.get_recent(1).len(), 1);
}
