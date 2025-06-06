use crate::llm_clients::LLMClient;
use crate::aureus_bridge::AureusBridge;
use crate::memory_store::MemoryStore;

pub struct IntegrationLayer {
    connected: bool,
    llm: Option<Box<dyn LLMClient>>,
}

impl IntegrationLayer {
    pub fn new() -> Self {
        Self {
            connected: false,
            llm: None,
        }
    }

    pub fn connect(&mut self) {
        self.connected = true;
        println!("[IntegrationLayer] Connected.");
    }

    pub fn set_client(&mut self, client: Box<dyn LLMClient>) {
        self.llm = Some(client);
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        println!("[IntegrationLayer] Disconnected.");
    }

    pub fn send_message(&self, message: &str) {
        if self.connected {
            println!("[IntegrationLayer] Sent: {}", message);
        } else {
            println!("[IntegrationLayer] Not connected. Dropping message.");
        }
    }

    pub fn invoke_llm(&self, prompt: &str) -> Option<String> {
        self.llm.as_ref().map(|c| c.generate_response(prompt))
    }

    pub fn trigger_reflexion(
        &self,
        bridge: &mut AureusBridge,
        context: &str,
        store: &mut MemoryStore,
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
        layer.set_client(Box::new(DummyLLM));
        assert!(layer.is_connected());
        let resp = layer.invoke_llm("hi");
        assert_eq!(resp.unwrap(), "ok");
    }
}
