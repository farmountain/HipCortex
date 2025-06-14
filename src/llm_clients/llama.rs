use super::LLMClient;
use reqwest::blocking::Client;
use serde_json::json;

/// Simple connector for llama.cpp style HTTP endpoints.
pub struct LlamaClient {
    pub base_url: String,
}

impl LlamaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }
}

impl LLMClient for LlamaClient {
    fn generate_response(&self, prompt: &str) -> String {
        let client = Client::new();
        let url = format!("{}/v1/generate", self.base_url.trim_end_matches('/'));
        let body = json!({"prompt": prompt});
        let resp = client.post(&url).json(&body).send();
        match resp {
            Ok(r) => match r.json::<serde_json::Value>() {
                Ok(val) => val["response"].as_str().unwrap_or("").to_string(),
                Err(_) => "".into(),
            },
            Err(_) => "".into(),
        }
    }
}
