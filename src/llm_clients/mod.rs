pub trait LLMClient: Send + Sync {
    fn generate_response(&self, prompt: &str) -> String;
}

/// Extended language model client used by plugin hosts.
pub trait LanguageModelClient: Send + Sync {
    fn generate(&self, prompt: &str) -> String;
    fn embed(&self, text: &str) -> Vec<f32>;
    fn chat(&self, history: Vec<String>, new_input: &str) -> String;
}

pub mod claude;
pub mod deepseek_client;
pub mod falcon_client;
pub mod llama;
pub mod local_llm_client;
pub mod mistral_client;
pub mod mock;
pub mod ollama;
pub mod openai;
