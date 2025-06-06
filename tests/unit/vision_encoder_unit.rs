use hipcortex::vision_encoder::VisionEncoder;
use image::{RgbImage, DynamicImage};

#[test]
fn encode_rgb() {
    let img = RgbImage::from_pixel(1,1, image::Rgb([0,0,255]));
    let dyn = DynamicImage::ImageRgb8(img);
    let emb = VisionEncoder::encode_image(&dyn);
    assert_eq!(emb.len(),3);
}
