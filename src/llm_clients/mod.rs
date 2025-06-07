pub trait LLMClient: Send + Sync {
    fn generate_response(&self, prompt: &str) -> String;
}

pub mod claude;
pub mod mock;
pub mod ollama;
pub mod openai;
