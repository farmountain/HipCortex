use std::time::{Duration, SystemTime};
use uuid::Uuid;

use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::decay::DecayType;

#[test]
fn get_recent_zero_returns_empty() {
    let indexer: TemporalIndexer<i32> = TemporalIndexer::new(2, 3600);
    let results = indexer.get_recent(0);
    assert!(results.is_empty());
}

#[test]
fn inserting_trace_preserves_id() {
    let mut indexer = TemporalIndexer::new(2, 3600);
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: 42,
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now(),
        decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
    };
    let expected_id = trace.id;
    indexer.insert(trace);
    assert_eq!(indexer.get_recent(1)[0].id, expected_id);
}
