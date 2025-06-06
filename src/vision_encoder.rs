use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use std::path::Path;

pub struct VisionEncoder;

impl VisionEncoder {
    /// Encode a DynamicImage into a simple RGB mean vector.
    pub fn encode_image(image: &DynamicImage) -> Vec<f32> {
        let (w, h) = image.dimensions();
        let rgb = image.to_rgb8();
        let mut r_sum = 0u64;
        let mut g_sum = 0u64;
        let mut b_sum = 0u64;
        for p in rgb.pixels() {
            r_sum += p[0] as u64;
            g_sum += p[1] as u64;
            b_sum += p[2] as u64;
        }
        let total = (w * h) as f32 * 255.0;
        vec![
            r_sum as f32 / total,
            g_sum as f32 / total,
            b_sum as f32 / total,
        ]
    }

    /// Encode raw image bytes (PNG/JPEG) into an embedding.
    pub fn encode_bytes(bytes: &[u8]) -> Result<Vec<f32>> {
        let img = image::load_from_memory(bytes)?;
        Ok(Self::encode_image(&img))
    }

    /// Encode image from a file path.
    pub fn encode_path<P: AsRef<Path>>(path: P) -> Result<Vec<f32>> {
        let img = image::open(path)?;
        Ok(Self::encode_image(&img))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{RgbImage, DynamicImage};

    #[test]
    fn encode_simple() {
        let img = RgbImage::from_pixel(1,1, image::Rgb([0,0,0]));
        let img_dyn = DynamicImage::ImageRgb8(img);
        let emb = VisionEncoder::encode_image(&img_dyn);
        assert_eq!(emb.len(),3);
    }
}
