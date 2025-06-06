pub trait LLMClient: Send + Sync {
    fn generate_response(&self, prompt: &str) -> String;
}

pub mod openai;
pub mod ollama;
pub mod claude;
pub mod mock;
