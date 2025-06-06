use super::LLMClient;
use reqwest::blocking::Client;
use serde_json::json;

pub struct OllamaClient {
    pub base_url: String,
    pub model: String,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
        }
    }
}

impl LLMClient for OllamaClient {
    fn generate_response(&self, prompt: &str) -> String {
        let client = Client::new();
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let body = json!({"model": self.model, "prompt": prompt});
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
