/// Chain-of-Thought: input -> symbol parse -> PCA -> compressed vector

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

impl PerceptionAdapter {
    pub fn adapt(input: PerceptInput) -> Option<Vec<f32>> {
        if !ADAPTER_LIMITER.allow() {
            println!("[PerceptionAdapter] rate limit exceeded");
            return None;
        }
        match input.modality {
            Modality::Text => {
                println!("[PerceptionAdapter] Text: {:?}", input.text);
                None
            }
            Modality::ImageEmbedding => {
                if let Some(embed) = input.embedding {
                    let comp =
                        crate::semantic_compression::compress_embedding(&embed, COMPRESS_DIM);
                    println!("[PerceptionAdapter] Embedding: {:?}", comp);
                    Some(comp)
                } else {
                    println!("[PerceptionAdapter] Embedding: None");
                    None
                }
            }
            Modality::Image => {
                if let Some(bytes) = input.image_data {
                    match crate::vision_encoder::VisionEncoder::encode_bytes(&bytes) {
                        Ok(emb) => {
                            let comp =
                                crate::semantic_compression::compress_embedding(&emb, COMPRESS_DIM);
                            println!("[PerceptionAdapter] Image embedding: {:?}", comp);
                            Some(comp)
                        }
                        Err(e) => {
                            println!("[PerceptionAdapter] Image encoding error: {}", e);
                            None
                        }
                    }
                } else {
                    println!("[PerceptionAdapter] No image data");
                    None
                }
            }
            Modality::SymbolicConcept => {
                println!(
                    "[PerceptionAdapter] Symbolic concept tags: {:?}",
                    input.tags
                );
                None
            }
            _ => {
                println!("[PerceptionAdapter] Input: {:?}", input);
                None
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
        PerceptionAdapter::adapt(input);
    }

    #[test]
    fn rate_limit() {
        for _ in 0..12 {
            PerceptionAdapter::adapt(PerceptInput {
                modality: Modality::Text,
                text: Some("x".into()),
                embedding: None,
                image_data: None,
                tags: vec![],
            });
        }
    }
}
