#[derive(Debug, Clone)]
pub enum Modality {
    Text,
    ImageEmbedding,
    SymbolicConcept,
    AgentMessage,
}

#[derive(Debug, Clone)]
pub struct PerceptInput {
    pub modality: Modality,
    pub text: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub tags: Vec<String>,
}

pub struct PerceptionAdapter;

impl PerceptionAdapter {
    pub fn adapt(input: PerceptInput) {
        println!("[PerceptionAdapter] Input: {:?}", input);
    }
}
