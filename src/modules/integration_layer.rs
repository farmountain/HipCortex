use crate::aureus_bridge::AureusBridge;
use crate::llm_clients::LLMClient;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::openmanus_bridge;
use crate::mcp_bridge;
use crate::perception_adapter::{Modality, PerceptionSession};
use crate::persistence::MemoryBackend;
use serde::Deserialize;
use std::collections::HashSet;

pub struct IntegrationLayer {
    connected: bool,
    llm: Option<Box<dyn LLMClient>>,
    api_keys: HashSet<String>,
    oauth_tokens: HashSet<String>,
    perception: PerceptionSession,
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
            perception: PerceptionSession::new(),
        }
    }

    /// Builder to attach a (possibly intel-augmented) PerceptionSession for agent defaults.
    pub fn with_perception_session(mut self, ps: PerceptionSession) -> Self {
        self.perception = ps;
        self
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
        if crate::safety_guardrail::SAFETY_GUARDRAIL
            .lock()
            .unwrap()
            .check_precondition(message)
            .is_err()
        {
            return;
        }
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

    pub fn handle_openmanus(&self, key: &str, payload: &str) {
        match openmanus_bridge::decode_request(payload) {
            Ok(p) => {
                if let Some(text) = p.text {
                    self.send_message(key, &text);
                }
            }
            Err(_) => println!("[IntegrationLayer] invalid OpenManus payload"),
        }
    }

    pub fn handle_mcp(&self, key: &str, payload: &str) {
        match mcp_bridge::decode_request(payload) {
            Ok(p) => {
                if matches!(p.modality, Modality::AgentMessage) {
                    // default intel hooks (self health gate, world update, coherence) via existing PerceptionSession
                    // gated by SAFETY_GUARDRAIL (per engine-agent-defaults spec)
                    if let Some(text) = &p.text {
                        if crate::safety_guardrail::SAFETY_GUARDRAIL
                            .lock()
                            .unwrap()
                            .check_precondition(text)
                            .is_err()
                        {
                            // violation logged by guardrail; still forward text but skip intel/auto for safety
                        } else {
                            let _ = self.perception.adapt(p.clone());
                            let actor = p.tags.get(0).cloned().unwrap_or_else(|| "agent".to_string());
                            let mut rec = MemoryRecord::new(
                                MemoryType::Temporal,
                                actor.clone(),
                                "perceive".to_string(),
                                text.clone(),
                                serde_json::json!({"source": "agent-auto-ingest"}),
                            );
                            rec.priority = "low".to_string();
                            rec.source = Some("agent-auto-ingest".to_string());
                            rec.tags = p.tags.clone();
                            println!("[IntegrationLayer] auto low-pri Temporal ingest source=agent-auto-ingest actor={}", actor);

                            // Automatic world model maintenance from the agent stream (no explicit "remember this decision" or "update world model" trigger required from user or LLM).
                            // The perception hooks already did entity embedding + coherence + self health gate.
                            // This additionally feeds a transition (text as action) into the Dirichlet model so the substrate keeps its predictive state up to date automatically.
                            // This is the beginning of the substrate not behaving like it has dementia for "latest update of state, world model".
                            self.perception.record_perceived_action(text.clone());
                        }
                    }
                }
                if let Some(text) = p.text {
                    self.send_message(key, &text);
                }
            }
            Err(_) => println!("[IntegrationLayer] invalid MCP payload"),
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

    #[cfg(feature = "web-server")]
    pub async fn run_uat_server(addr: std::net::SocketAddr) {
        use axum::{routing::post, Json, Router};

        async fn translate(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
            let json = serde_json::to_string(&payload).unwrap();
            if payload.get("role").is_some() {
                match openmanus_bridge::decode_request(&json) {
                    Ok(p) => Json(serde_json::json!({
                        "text": p.text,
                        "tags": p.tags
                    })),
                    Err(_) => Json(serde_json::json!({"error": "invalid"})),
                }
            } else if payload.get("agent").is_some() {
                match mcp_bridge::decode_request(&json) {
                    Ok(p) => Json(serde_json::json!({
                        "text": p.text,
                        "tags": p.tags
                    })),
                    Err(_) => Json(serde_json::json!({"error": "invalid"})),
                }
            } else {
                Json(serde_json::json!({"error": "unknown"}))
            }
        }

        let app = Router::new().route("/translate", post(translate));
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
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
