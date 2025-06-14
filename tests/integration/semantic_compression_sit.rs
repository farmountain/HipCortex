use hipcortex::semantic_compression::compress_embedding;
use hipcortex::vision_encoder::VisionEncoder;
use image::{DynamicImage, RgbImage};

#[test]
fn vision_compress_round_trip() {
    let img = RgbImage::from_pixel(1, 1, image::Rgb([10, 20, 30]));
    let emb = VisionEncoder::encode_image(&DynamicImage::ImageRgb8(img));
    let compressed = compress_embedding(&emb, 2);
    assert_eq!(compressed.len(), 2);
}
