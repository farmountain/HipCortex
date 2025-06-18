use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::decay::DecayType;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[test]
fn edge_indexer_prunes_old_entries() {
    let mut indexer = TemporalIndexer::new(2, 3600);
    for i in 0..3u8 {
        indexer.insert(TemporalTrace {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            data: i,
            relevance: 1.0,
            decay_factor: 1.0,
            last_access: SystemTime::now(),
            decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
        });
    }
    assert_eq!(indexer.get_recent(3).len(), 2);
}
