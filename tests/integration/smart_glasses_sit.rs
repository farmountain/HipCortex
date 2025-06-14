use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};
use hipcortex::retrieval_pipeline::recent_symbols;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::vision_encoder::VisionEncoder;
use image::{DynamicImage, ImageOutputFormat, RgbImage};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

#[test]
fn smart_glasses_capture_and_retrieve() {
    let mut store = SymbolicStore::new();
    let mut indexer = TemporalIndexer::new(4, 3600);

    // simulate smart glasses snapshot
    let img = RgbImage::from_pixel(1, 1, image::Rgb([0, 0, 255]));
    let mut buf = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img).write_to(&mut buf, ImageOutputFormat::Png).unwrap();
    let bytes = buf.into_inner();
    let embedding = VisionEncoder::encode_bytes(&bytes).unwrap();

    let node_id = store.add_node("View", HashMap::new());
    indexer.insert(TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: node_id,
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
    });

    let input = PerceptInput {
        modality: Modality::Image,
        text: None,
        embedding: Some(embedding),
        image_data: Some(bytes),
        tags: vec!["glasses".into()],
    };
    PerceptionAdapter::adapt(input);

    let nodes = recent_symbols(&store, &indexer, 1);
    assert_eq!(nodes[0].label, "View");
}
