use super::{LLMClient, LanguageModelClient};
use reqwest::blocking::Client;
use serde_json::json;

/// DeepSeek HTTP connector.
pub struct DeepSeekClient {
    pub base_url: String,
}

impl DeepSeekClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }
}

impl LanguageModelClient for DeepSeekClient {
    fn generate(&self, prompt: &str) -> String {
        let client = Client::new();
        let url = format!("{}/generate", self.base_url.trim_end_matches('/'));
        let resp = client.post(&url).json(&json!({"prompt": prompt})).send();
        match resp {
            Ok(r) => r
                .json::<serde_json::Value>()
                .map(|v| v["text"].as_str().unwrap_or("").to_string())
                .unwrap_or_default(),
            Err(_) => "".into(),
        }
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        let client = Client::new();
        let url = format!("{}/embed", self.base_url.trim_end_matches('/'));
        let resp = client.post(&url).json(&json!({"text": text})).send();
        match resp {
            Ok(r) => r
                .json::<serde_json::Value>()
                .map(|v| {
                    v["embedding"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|x| x.as_f64().map(|f| f as f32))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => vec![],
        }
    }

    fn chat(&self, history: Vec<String>, new_input: &str) -> String {
        let client = Client::new();
        let url = format!("{}/chat", self.base_url.trim_end_matches('/'));
        let body = json!({"history": history, "input": new_input});
        let resp = client.post(&url).json(&body).send();
        match resp {
            Ok(r) => r
                .json::<serde_json::Value>()
                .map(|v| v["reply"].as_str().unwrap_or("").to_string())
                .unwrap_or_default(),
            Err(_) => "".into(),
        }
    }
}

impl LLMClient for DeepSeekClient {
    fn generate_response(&self, prompt: &str) -> String {
        self.generate(prompt)
    }
}
