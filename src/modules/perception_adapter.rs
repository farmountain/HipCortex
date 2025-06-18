/// Chain-of-Thought: input -> symbol parse -> PCA -> compressed vector
use nalgebra::DMatrix;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub enum Modality {
    Text,
    ImageEmbedding,
    Image,
    SymbolicConcept,
    AgentMessage,
}

#[derive(Debug, Clone)]
pub struct PerceptInput {
    pub modality: Modality,
    pub text: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub image_data: Option<Vec<u8>>,
    pub tags: Vec<String>,
}

pub struct PerceptionAdapter;

#[derive(Debug)]
pub enum AdapterError {
    RateLimited,
    InvalidInput(&'static str),
    ImageEncoding(anyhow::Error),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::RateLimited => write!(f, "rate limit exceeded"),
            AdapterError::InvalidInput(s) => write!(f, "invalid input: {}", s),
            AdapterError::ImageEncoding(e) => write!(f, "image encoding error: {}", e),
        }
    }
}

impl std::error::Error for AdapterError {}

const COMPRESS_DIM: usize = 4;

struct RateLimiter {
    capacity: u32,
    tokens: std::sync::Mutex<(u32, std::time::Instant)>,
}

impl RateLimiter {
    fn new(capacity: u32) -> Self {
        Self {
            capacity,
            tokens: std::sync::Mutex::new((capacity, std::time::Instant::now())),
        }
    }

    fn allow(&self) -> bool {
        let mut guard = self.tokens.lock().unwrap();
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(guard.1);
        if elapsed.as_secs() >= 60 {
            guard.0 = self.capacity;
            guard.1 = now;
        }
        if guard.0 == 0 {
            return false;
        }
        guard.0 -= 1;
        true
    }
}

lazy_static::lazy_static! {
    static ref ADAPTER_LIMITER: RateLimiter = RateLimiter::new(10);
}

fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in v.iter_mut() {
            *val /= norm;
        }
    }
    v
}

fn shannon_entropy(v: &[f32]) -> f32 {
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

fn text_to_embedding(text: &str, dim: usize) -> Vec<f32> {
    let mut vec = vec![0.0; dim];
    for token in text.split_whitespace() {
        let mut hasher = DefaultHasher::new();
        token.hash(&mut hasher);
        let idx = (hasher.finish() as usize) % dim;
        vec[idx] += 1.0;
    }
    vec
}

fn pca_compress(data: &[f32], new_dim: usize) -> Vec<f32> {
    if new_dim == 0 || data.is_empty() {
        return vec![];
    }
    if data.len() < new_dim {
        return crate::semantic_compression::compress_embedding(data, new_dim);
    }
    let rows = data.len() - new_dim + 1;
    let mut mat_data = Vec::with_capacity(rows * new_dim);
    for i in 0..rows {
        for j in 0..new_dim {
            mat_data.push(data[i + j]);
        }
    }
    let mut m = DMatrix::from_row_slice(rows, new_dim, &mat_data);
    for c in 0..new_dim {
        let mean: f32 = (0..rows).map(|r| m[(r, c)]).sum::<f32>() / rows as f32;
        for r in 0..rows {
            m[(r, c)] -= mean;
        }
    }
    let svd = m.svd(true, true);
    let vt = svd.v_t.unwrap();
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

impl PerceptionAdapter {
    pub fn adapt(input: PerceptInput) -> Result<Vec<f32>, AdapterError> {
        if !ADAPTER_LIMITER.allow() {
            return Err(AdapterError::RateLimited);
        }
        match input.modality {
            Modality::Text => {
                let text = input.text.ok_or(AdapterError::InvalidInput("missing text"))?;
                let emb = text_to_embedding(&text, COMPRESS_DIM * 4);
                let comp = pca_compress(&emb, COMPRESS_DIM);
                let out = l2_normalize(comp);
                let ent = shannon_entropy(&out);
                println!("[PerceptionAdapter] entropy {:.3}", ent);
                Ok(out)
            }
            Modality::ImageEmbedding => {
                if let Some(embed) = input.embedding {
                    let comp = pca_compress(&embed, COMPRESS_DIM);
                    let out = l2_normalize(comp);
                    println!("[PerceptionAdapter] entropy {:.3}", shannon_entropy(&out));
                    Ok(out)
                } else {
                    Err(AdapterError::InvalidInput("missing embedding"))
                }
            }
            Modality::Image => {
                if let Some(bytes) = input.image_data {
                    match crate::vision_encoder::VisionEncoder::encode_bytes(&bytes) {
                        Ok(emb) => {
                            let comp = pca_compress(&emb, COMPRESS_DIM);
                            let out = l2_normalize(comp);
                            println!("[PerceptionAdapter] entropy {:.3}", shannon_entropy(&out));
                            Ok(out)
                        }
                        Err(e) => Err(AdapterError::ImageEncoding(e)),
                    }
                } else {
                    Err(AdapterError::InvalidInput("no image data"))
                }
            }
            Modality::SymbolicConcept => {
                if input.tags.is_empty() {
                    Err(AdapterError::InvalidInput("empty tags"))
                } else {
                    println!("[PerceptionAdapter] Symbolic concept tags: {:?}", input.tags);
                    Ok(Vec::new())
                }
            }
            Modality::AgentMessage => {
                let text = input.text.ok_or(AdapterError::InvalidInput("missing text"))?;
                let emb = text_to_embedding(&text, COMPRESS_DIM * 4);
                let comp = pca_compress(&emb, COMPRESS_DIM);
                let out = l2_normalize(comp);
                println!("[PerceptionAdapter] entropy {:.3}", shannon_entropy(&out));
                Ok(out)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapt_text() {
        let input = PerceptInput {
            modality: Modality::Text,
            text: Some("hi".to_string()),
            embedding: None,
            image_data: None,
            tags: vec![],
        };
        let out = PerceptionAdapter::adapt(input).unwrap();
        assert_eq!(out.len(), COMPRESS_DIM);
    }

    #[test]
    fn rate_limit() {
        let mut last = None;
        for _ in 0..12 {
            last = Some(PerceptionAdapter::adapt(PerceptInput {
                modality: Modality::Text,
                text: Some("x".into()),
                embedding: None,
                image_data: None,
                tags: vec![],
            }));
        }
        let result = last.unwrap();
        assert!(matches!(result.unwrap_err(), AdapterError::RateLimited));
    }

    #[test]
    fn pca_reconstruction_error() {
        let data: Vec<f32> = (0..8).map(|v| v as f32).collect();
        let compressed = pca_compress(&data, 4);
        assert_eq!(compressed.len(), 4);

        // Recompute PCA basis for reconstruction
        let rows = data.len() - 4 + 1;
        let mut mat_data = Vec::with_capacity(rows * 4);
        for i in 0..rows {
            for j in 0..4 {
                mat_data.push(data[i + j]);
            }
        }
        let mut m = DMatrix::from_row_slice(rows, 4, &mat_data);
        for c in 0..4 {
            let mean: f32 = (0..rows).map(|r| m[(r, c)]).sum::<f32>() / rows as f32;
            for r in 0..rows {
                m[(r, c)] -= mean;
            }
        }
        let vt = m.svd(true, true).v_t.unwrap();
        let mut reconstruct = vec![0.0f32; 4];
        for i in 0..4 {
            for j in 0..4 {
                reconstruct[j] += compressed[i] * vt[(i, j)];
            }
        }
        let err = data[..4]
            .iter()
            .zip(reconstruct.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt();
        assert!(err < 1e-3);
    }

    #[test]
    fn l2_normalization_unit() {
        let v = vec![3.0, 4.0];
        let normed = l2_normalize(v);
        let norm = normed.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn entropy_threshold() {
        let input = PerceptInput {
            modality: Modality::ImageEmbedding,
            text: None,
            embedding: Some(vec![1.0, 2.0, 3.0, 4.0, 0.5, 0.2, 0.1, 0.0]),
            image_data: None,
            tags: vec![],
        };
        let out = PerceptionAdapter::adapt(input).unwrap();
        let ent = shannon_entropy(&out);
        assert!(ent > 0.1);
    }
}
