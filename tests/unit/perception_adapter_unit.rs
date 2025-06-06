use hipcortex::perception_adapter::{PerceptionAdapter, PerceptInput, Modality};

#[test]
fn adapt_text() {
    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("hi".to_string()),
        embedding: None,
        image_data: None,
        tags: vec![],
    };
    PerceptionAdapter::adapt(input);
}
