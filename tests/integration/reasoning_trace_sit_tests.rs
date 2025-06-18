use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use std::time::SystemTime;
use uuid::Uuid;

#[test]
fn store_reasoning_trace_via_adapter_and_indexer() {
    let mut indexer = TemporalIndexer::new(2, 3600);
    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("remember me".to_string()),
        embedding: None,
        image_data: None,
        tags: vec!["sit".into()],
    };
    let out = PerceptionAdapter::adapt(input.clone()).unwrap();
    assert_eq!(out.len(), 4);
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: input.text.unwrap(),
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
    };
    indexer.insert(trace);
    let recent = indexer.get_recent(1);
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].data, "remember me");
}
