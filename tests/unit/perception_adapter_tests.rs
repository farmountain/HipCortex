use hipcortex::perception_adapter::{AdapterError, Modality, PerceptInput, PerceptionAdapter};

#[test]
fn adapt_text_input() {
    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("test text".to_string()),
        embedding: None,
        image_data: None,
        tags: vec!["tag1".to_string()],
    };
    let out = PerceptionAdapter::adapt(input).unwrap();
    assert!(out.len() <= 4);
}

#[test]
fn adapt_embedding_input() {
    let input = PerceptInput {
        modality: Modality::ImageEmbedding,
        text: None,
        embedding: Some(vec![0.1, 0.2, 0.3]),
        image_data: None,
        tags: vec![],
    };
    let out = PerceptionAdapter::adapt(input).unwrap();
    assert!(out.len() <= 4);
}

#[test]
fn adapt_invalid_input() {
    let input = PerceptInput {
        modality: Modality::AgentMessage,
        text: None,
        embedding: None,
        image_data: None,
        tags: vec![],
    };
    let err = PerceptionAdapter::adapt(input).unwrap_err();
    assert!(matches!(err, AdapterError::InvalidInput(_)));
}

#[test]
fn adapt_image_input() {
    let input = PerceptInput {
        modality: Modality::Image,
        text: None,
        embedding: None,
        image_data: Some(vec![1, 2, 3]),
        tags: vec!["img".to_string()],
    };
    let err = PerceptionAdapter::adapt(input).unwrap_err();
    assert!(matches!(err, AdapterError::ImageEncoding(_)));
}

#[test]
fn adapt_symbolic_concept() {
    let input = PerceptInput {
        modality: Modality::SymbolicConcept,
        text: None,
        embedding: None,
        image_data: None,
        tags: vec!["concept".to_string()],
    };
    let out = PerceptionAdapter::adapt(input).unwrap();
    assert!(out.is_empty());
}
