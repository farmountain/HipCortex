use super::LLMClient;
use reqwest::blocking::Client;
use serde_json::json;

pub struct OpenAIClient {
    pub api_key: String,
    pub model: String,
}

impl OpenAIClient {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

impl LLMClient for OpenAIClient {
    fn generate_response(&self, prompt: &str) -> String {
        let client = Client::new();
        let body = json!({
            "model": self.model,
            "messages": [{"role": "user", "content": prompt}],
        });
        let resp = client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send();
        match resp {
            Ok(r) => match r.json::<serde_json::Value>() {
                Ok(val) => val["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                Err(_) => "".into(),
            },
            Err(_) => "".into(),
        }
    }
}
