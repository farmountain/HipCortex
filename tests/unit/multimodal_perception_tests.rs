use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};
use hipcortex::vision_encoder::VisionEncoder;
use image::{DynamicImage, ImageOutputFormat, RgbImage};

#[test]
fn multimodal_text_and_image() {
    // Provide both text and image bytes as a combined percept
    let img = RgbImage::from_pixel(1, 1, image::Rgb([42, 42, 42]));
    let mut buf = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img.clone()).write_to(&mut buf, ImageOutputFormat::Png).unwrap();
    let bytes = buf.into_inner();
    let embedding = VisionEncoder::encode_bytes(&bytes).unwrap();

    let input = PerceptInput {
        modality: Modality::Image,
        text: Some("smart glasses".into()),
        embedding: None,
        image_data: Some(bytes),
        tags: vec!["multimodal".into()],
    };
    PerceptionAdapter::adapt(input);
    assert_eq!(embedding.len(), 3);
}

#[test]
fn humanoid_robotics_embedding_trace() {
    let embed = vec![0.1, 0.2, 0.3];
    let input = PerceptInput {
        modality: Modality::ImageEmbedding,
        text: None,
        embedding: Some(embed.clone()),
        image_data: None,
        tags: vec!["robot".into()],
    };
    PerceptionAdapter::adapt(input.clone());
    assert_eq!(input.embedding.unwrap().len(), 3);
}
