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
    pub fn adapt(input: PerceptInput) {
        if !ADAPTER_LIMITER.allow() {
            println!("[PerceptionAdapter] rate limit exceeded");
            return;
        }
        match input.modality {
            Modality::Text => {
                println!("[PerceptionAdapter] Text: {:?}", input.text);
            }
            Modality::ImageEmbedding => {
                println!("[PerceptionAdapter] Embedding: {:?}", input.embedding);
            }
            Modality::Image => {
                if let Some(bytes) = input.image_data {
                    match crate::vision_encoder::VisionEncoder::encode_bytes(&bytes) {
                        Ok(emb) => {
                            println!("[PerceptionAdapter] Image embedding: {:?}", emb);
                        }
                        Err(e) => {
                            println!("[PerceptionAdapter] Image encoding error: {}", e);
                        }
                    }
                } else {
                    println!("[PerceptionAdapter] No image data");
                }
            }
            Modality::SymbolicConcept => {
                println!(
                    "[PerceptionAdapter] Symbolic concept tags: {:?}",
                    input.tags
                );
            }
            _ => {
                println!("[PerceptionAdapter] Input: {:?}", input);
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
