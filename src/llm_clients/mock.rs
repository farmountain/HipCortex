use super::LLMClient;

pub struct MockClient;

impl LLMClient for MockClient {
    fn generate_response(&self, prompt: &str) -> String {
        format!("mock: {}", prompt)
    }
}
