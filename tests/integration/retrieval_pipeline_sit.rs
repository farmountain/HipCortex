use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};
use hipcortex::retrieval_pipeline::recent_symbols;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

#[test]
fn retrieval_round_trip() {
    let mut store = SymbolicStore::new();
    let mut indexer = TemporalIndexer::new(4, 3600);

    let id = store.add_node("Article", HashMap::new());

    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("article content".into()),
        embedding: None,
        image_data: None,
        tags: vec![],
    };
    let out = PerceptionAdapter::adapt(input).unwrap();
    assert_eq!(out.len(), 4);

    indexer.insert(TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: id,
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
    });

    let result = recent_symbols(&store, &indexer, 1);
    assert_eq!(result[0].label, "Article");
}
