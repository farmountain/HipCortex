use hipcortex::vision_encoder::VisionEncoder;
use image::{DynamicImage, ImageOutputFormat, RgbImage};
use std::io::Cursor;

#[test]
fn encode_image_returns_average_rgb() {
    let img = RgbImage::from_fn(2, 2, |_, _| image::Rgb([255, 0, 0]));
    let img_dyn = DynamicImage::ImageRgb8(img);
    let emb = VisionEncoder::encode_image(&img_dyn);
    assert!((emb[0] - 1.0).abs() < 1e-6);
    assert!((emb[1]).abs() < 1e-6);
    assert!((emb[2]).abs() < 1e-6);
}

#[test]
fn encode_bytes_matches_encode_image() {
    let img = RgbImage::from_pixel(1, 1, image::Rgb([0, 255, 0]));
    let mut buf = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut buf, ImageOutputFormat::Png)
        .unwrap();
    let bytes = buf.into_inner();
    let emb_bytes = VisionEncoder::encode_bytes(&bytes).unwrap();
    let emb_image = VisionEncoder::encode_image(&DynamicImage::ImageRgb8(img));
    assert_eq!(emb_bytes.len(), 3);
    for (a, b) in emb_bytes.iter().zip(emb_image.iter()) {
        assert!((a - b).abs() < 1e-6);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn parallel_encoding_matches() {
    let img = RgbImage::from_pixel(1, 1, image::Rgb([10, 20, 30]));
    let dynimg = DynamicImage::ImageRgb8(img);
    let seq = VisionEncoder::encode_image(&dynimg);
    let par = VisionEncoder::encode_images_parallel(&[dynimg]);
    assert_eq!(par[0], seq);
}
