use crate::aureus_bridge::AureusBridge;
use crate::llm_clients::LLMClient;
use crate::memory_store::MemoryStore;
use crate::persistence::MemoryBackend;
use serde::Deserialize;
use std::collections::HashSet;

pub struct IntegrationLayer {
    connected: bool,
    llm: Option<Box<dyn LLMClient>>,
    api_keys: HashSet<String>,
    oauth_tokens: HashSet<String>,
}

fn validate_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.trim().is_empty() || s.len() > 512 {
        return Err(serde::de::Error::custom("text invalid"));
    }
    Ok(s)
}

#[derive(Deserialize)]
struct InputPayload {
    #[serde(deserialize_with = "validate_text")]
    text: String,
}

impl InputPayload {
    fn valid(&self) -> bool {
        !self.text.trim().is_empty() && self.text.len() <= 512
    }
}

impl IntegrationLayer {
    pub fn new() -> Self {
        Self {
            connected: false,
            llm: None,
            api_keys: HashSet::new(),
            oauth_tokens: HashSet::new(),
        }
    }

    pub fn connect(&mut self) {
        self.connected = true;
        println!("[IntegrationLayer] Connected.");
    }

    pub fn add_api_key(&mut self, key: String) {
        self.api_keys.insert(key);
    }

    pub fn add_oauth_token(&mut self, token: String) {
        self.oauth_tokens.insert(token);
    }

    fn authenticated(&self, key: &str) -> bool {
        self.api_keys.contains(key) || self.oauth_tokens.contains(key)
    }

    pub fn set_client(&mut self, client: Box<dyn LLMClient>) {
        self.llm = Some(client);
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        println!("[IntegrationLayer] Disconnected.");
    }

    pub fn send_message(&self, key: &str, message: &str) {
        if !self.authenticated(key) {
            println!("[IntegrationLayer] unauthorized");
            return;
        }
        if self.connected {
            println!("[IntegrationLayer] Sent: {}", message);
        } else {
            println!("[IntegrationLayer] Not connected. Dropping message.");
        }
    }

    pub fn send_message_json(&self, key: &str, payload: &str) {
        match serde_json::from_str::<InputPayload>(payload) {
            Ok(p) if p.valid() => self.send_message(key, &p.text),
            _ => println!("[IntegrationLayer] invalid payload"),
        }
    }

    pub fn invoke_llm(&self, key: &str, prompt: &str) -> Option<String> {
        if !self.authenticated(key) {
            println!("[IntegrationLayer] unauthorized");
            return None;
        }
        self.llm.as_ref().map(|c| c.generate_response(prompt))
    }

    pub fn trigger_reflexion<B: MemoryBackend>(
        &self,
        bridge: &mut AureusBridge,
        context: &str,
        store: &mut MemoryStore<B>,
    ) {
        bridge.reflexion_loop(context, store);
    }

    pub fn handle_n8n_webhook(&self, payload: &str) {
        println!("[IntegrationLayer] n8n webhook payload: {}", payload);
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyLLM;
    impl LLMClient for DummyLLM {
        fn generate_response(&self, _prompt: &str) -> String {
            "ok".to_string()
        }
    }

    #[test]
    fn connect_and_invoke() {
        let mut layer = IntegrationLayer::new();
        layer.connect();
        layer.add_api_key("k".into());
        layer.set_client(Box::new(DummyLLM));
        assert!(layer.is_connected());
        let resp = layer.invoke_llm("k", "hi");
        assert_eq!(resp.unwrap(), "ok");
    }
}
