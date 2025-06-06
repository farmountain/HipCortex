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

impl PerceptionAdapter {
    pub fn adapt(input: PerceptInput) {
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
                println!("[PerceptionAdapter] Symbolic concept tags: {:?}", input.tags);
            }
            _ => {
                println!("[PerceptionAdapter] Input: {:?}", input);
            }
        }
    }
}
