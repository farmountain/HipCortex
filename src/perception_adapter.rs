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
                println!("[PerceptionAdapter] Image bytes: {}", input.image_data.map_or(0, |d| d.len()));
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
