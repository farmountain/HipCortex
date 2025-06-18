use hipcortex::vision_encoder::VisionEncoder;
use image::{DynamicImage, ImageOutputFormat, RgbImage};
use std::io::Cursor;

#[test]
fn encode_image_basic_len_and_norm() {
    let img = RgbImage::from_fn(4, 4, |x, y| image::Rgb([(x * 10) as u8, (y * 20) as u8, 0]));
    let img_dyn = DynamicImage::ImageRgb8(img);
    let emb = VisionEncoder::encode_image(&img_dyn);
    assert_eq!(emb.len(), 8);
    let norm = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-6);
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
    assert_eq!(emb_bytes, emb_image);
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

#[cfg(feature = "gpu")]
#[test]
fn gpu_encoding_fallbacks() {
    let img = RgbImage::from_pixel(2, 2, image::Rgb([10, 20, 30]));
    let mut buf = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut buf, ImageOutputFormat::Png)
        .unwrap();
    let bytes = buf.into_inner();
    let gpu = pollster::block_on(VisionEncoder::encode_image_gpu(&bytes)).unwrap();
    let cpu = VisionEncoder::encode_image(&DynamicImage::ImageRgb8(img));
    assert_eq!(gpu, cpu);
}

#[test]
fn grid_output_shape() {
    let mut img = RgbImage::new(2, 2);
    img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
    img.put_pixel(1, 0, image::Rgb([0, 255, 0]));
    img.put_pixel(0, 1, image::Rgb([0, 0, 255]));
    img.put_pixel(1, 1, image::Rgb([255, 255, 255]));
    let dynimg = DynamicImage::ImageRgb8(img);
    let grid = VisionEncoder::encode_image_grid(&dynimg, 2);
    assert_eq!(grid.len(), 12);
    assert!((grid[0] - 1.0).abs() < 1e-6);
    assert!((grid[4] - 1.0).abs() < 1e-6); // green patch
}

#[test]
fn pca_reduce_dimension() {
    let data: Vec<f32> = (0..48).map(|v| v as f32).collect();
    let reduced = VisionEncoder::pca_reduce(&data, 4);
    assert_eq!(reduced.len(), 4);
}

#[test]
fn l2_norm_and_entropy() {
    let v = vec![3.0, 4.0];
    let normed = VisionEncoder::l2_normalize(&v);
    let norm = normed.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-6);
    let ent = VisionEncoder::estimate_entropy(&normed);
    assert!(ent > 0.0);
}
