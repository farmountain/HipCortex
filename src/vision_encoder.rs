use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use nalgebra::DMatrix;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::path::Path;

pub struct VisionEncoder;

const DEFAULT_GRID: usize = 4;
const DEFAULT_PCA_DIM: usize = 8;

impl VisionEncoder {
    /// Encode a DynamicImage into a normalized embedding.
    pub fn encode_image(image: &DynamicImage) -> Vec<f32> {
        let grid_feats = Self::encode_image_grid(image, DEFAULT_GRID);
        let reduced = Self::pca_reduce(&grid_feats, DEFAULT_PCA_DIM);
        let normed = Self::l2_normalize(&reduced);
        let ent = Self::estimate_entropy(&normed);
        println!("[VisionEncoder] entropy {:.3}", ent);
        normed
    }

    #[cfg(feature = "parallel")]
    pub fn encode_images_parallel(images: &[DynamicImage]) -> Vec<Vec<f32>> {
        images.par_iter().map(Self::encode_image).collect()
    }

    #[cfg(feature = "gpu")]
    pub async fn encode_image_gpu(bytes: &[u8]) -> Result<Vec<f32>> {
        // For now the GPU path simply loads the image and calls the CPU encoder
        // to keep results consistent across platforms.
        let img = image::load_from_memory(bytes)?;
        Ok(Self::encode_image(&img))
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

    /// Divide the image into a grid and compute mean RGB per patch.
    pub fn encode_image_grid(image: &DynamicImage, grid: usize) -> Vec<f32> {
        let (w, h) = image.dimensions();
        if grid == 0 || w < grid as u32 || h < grid as u32 {
            return Self::legacy_mean(image);
        }
        let rgb = image.to_rgb8();
        let mut out = Vec::with_capacity(grid * grid * 3);
        for gy in 0..grid {
            let y0 = (gy as u32 * h) / grid as u32;
            let y1 = ((gy as u32 + 1) * h) / grid as u32;
            for gx in 0..grid {
                let x0 = (gx as u32 * w) / grid as u32;
                let x1 = ((gx as u32 + 1) * w) / grid as u32;
                let mut r_sum = 0u64;
                let mut g_sum = 0u64;
                let mut b_sum = 0u64;
                let mut count = 0u64;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let p = rgb.get_pixel(x, y);
                        r_sum += p[0] as u64;
                        g_sum += p[1] as u64;
                        b_sum += p[2] as u64;
                        count += 1;
                    }
                }
                if count == 0 {
                    out.extend_from_slice(&[0.0, 0.0, 0.0]);
                } else {
                    let denom = count as f32 * 255.0;
                    out.push(r_sum as f32 / denom);
                    out.push(g_sum as f32 / denom);
                    out.push(b_sum as f32 / denom);
                }
            }
        }
        out
    }

    fn legacy_mean(image: &DynamicImage) -> Vec<f32> {
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

    /// Reduce a high dimensional vector with PCA.
    pub fn pca_reduce(data: &[f32], new_dim: usize) -> Vec<f32> {
        if new_dim == 0 || data.is_empty() {
            return vec![];
        }
        if data.len() < new_dim {
            return crate::semantic_compression::compress_embedding(data, new_dim);
        }
        let rows = data.len() - new_dim + 1;
        let mut mat = Vec::with_capacity(rows * new_dim);
        for i in 0..rows {
            for j in 0..new_dim {
                mat.push(data[i + j]);
            }
        }
        let mut m = DMatrix::from_row_slice(rows, new_dim, &mat);
        for c in 0..new_dim {
            let mean: f32 = (0..rows).map(|r| m[(r, c)]).sum::<f32>() / rows as f32;
            for r in 0..rows {
                m[(r, c)] -= mean;
            }
        }
        let vt = m.svd(true, true).v_t.unwrap();
        let mut out = vec![0.0; new_dim];
        for i in 0..new_dim {
            let mut val = 0.0;
            for j in 0..new_dim {
                val += data[j] * vt[(i, j)];
            }
            out[i] = val;
        }
        out
    }

    /// Normalize a vector to unit length.
    pub fn l2_normalize(v: &[f32]) -> Vec<f32> {
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            v.iter().map(|x| x / norm).collect()
        } else {
            v.to_vec()
        }
    }

    /// Estimate Shannon entropy of a vector.
    pub fn estimate_entropy(v: &[f32]) -> f32 {
        let sum: f32 = v.iter().map(|x| x.abs()).sum();
        if sum == 0.0 {
            return 0.0;
        }
        let mut ent = 0.0;
        for x in v {
            let p = x.abs() / sum;
            if p > 0.0 {
                ent -= p * p.log2();
            }
        }
        ent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage};

    #[test]
    fn encode_image_basic() {
        let img = RgbImage::from_pixel(1, 1, image::Rgb([0, 0, 255]));
        let dynimg = DynamicImage::ImageRgb8(img);
        let emb = VisionEncoder::encode_image(&dynimg);
        assert_eq!(emb.len(), 8);
    }
}
