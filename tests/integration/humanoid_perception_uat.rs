use hipcortex::integration_layer::IntegrationLayer;
use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use std::time::SystemTime;
use uuid::Uuid;

#[test]
fn user_flow_humanoid_robotics_trace() {
    let mut layer = IntegrationLayer::new();
    layer.connect();
    layer.add_api_key("r1".into());

    let mut indexer = TemporalIndexer::new(2, 3600);
    let input = PerceptInput {
        modality: Modality::AgentMessage,
        text: Some("robot perception".into()),
        embedding: None,
        image_data: None,
        tags: vec!["humanoid".into()],
    };
    let out = PerceptionAdapter::adapt(input.clone());
    assert!(out.is_none());

    indexer.insert(TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: input.text.unwrap(),
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
    });

    layer.send_message("r1", "trace stored");
    assert!(layer.is_connected());
    assert_eq!(indexer.get_recent(1).len(), 1);
}
