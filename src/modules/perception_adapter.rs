/// Chain-of-Thought: input -> symbol parse -> PCA -> compressed vector
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::math::{entropy::estimate_entropy, norm::l2_normalize, pca::pca_reduce};

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

impl PerceptionAdapter {
    pub fn adapt(input: PerceptInput) -> Result<Vec<f32>, AdapterError> {
        if !ADAPTER_LIMITER.allow() {
            return Err(AdapterError::RateLimited);
        }
        match input.modality {
            Modality::Text => {
                let text = input.text.ok_or(AdapterError::InvalidInput("missing text"))?;
                let emb = text_to_embedding(&text, COMPRESS_DIM * 4);
                let comp = pca_reduce(&emb, COMPRESS_DIM);
                let out = l2_normalize(comp);
                let ent = estimate_entropy(&out);
                println!("[PerceptionAdapter] entropy {:.3}", ent);
                Ok(out)
            }
            Modality::ImageEmbedding => {
                if let Some(embed) = input.embedding {
                    let comp = pca_reduce(&embed, COMPRESS_DIM);
                    let out = l2_normalize(comp);
                    println!("[PerceptionAdapter] entropy {:.3}", estimate_entropy(&out));
                    Ok(out)
                } else {
                    Err(AdapterError::InvalidInput("missing embedding"))
                }
            }
            Modality::Image => {
                if let Some(bytes) = input.image_data {
                    match crate::vision_encoder::VisionEncoder::encode_bytes(&bytes) {
                        Ok(emb) => {
                            let comp = pca_reduce(&emb, COMPRESS_DIM);
                            let out = l2_normalize(comp);
                            println!("[PerceptionAdapter] entropy {:.3}", estimate_entropy(&out));
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
                let comp = pca_reduce(&emb, COMPRESS_DIM);
                let out = l2_normalize(comp);
                println!("[PerceptionAdapter] entropy {:.3}", estimate_entropy(&out));
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
}
