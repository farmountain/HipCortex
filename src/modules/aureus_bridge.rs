use crate::memory_store::MemoryStore;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::llm_clients::LLMClient;

pub struct AureusConfig {
    pub enable_cot: bool,
}

impl Default for AureusConfig {
    fn default() -> Self {
        Self { enable_cot: false }
    }
}

pub struct AureusBridge {
    loops: usize,
    llm: Option<Box<dyn LLMClient>>,
    config: AureusConfig,
}

impl AureusBridge {
    pub fn new() -> Self {
        Self {
            loops: 0,
            llm: None,
            config: AureusConfig::default(),
        }
    }

    pub fn with_client(client: Box<dyn LLMClient>) -> Self {
        Self {
            loops: 0,
            llm: Some(client),
            config: AureusConfig::default(),
        }
    }

    pub fn set_client(&mut self, client: Box<dyn LLMClient>) {
        self.llm = Some(client);
    }

    pub fn configure(&mut self, cfg: AureusConfig) {
        self.config = cfg;
    }

    pub fn reflexion_loop(&mut self, context: &str, store: &mut MemoryStore) {
        self.loops += 1;
        if let Some(client) = &self.llm {
            let prompt = if self.config.enable_cot {
                format!("Think step by step. {}", context)
            } else {
                context.to_string()
            };
            let response = client.generate_response(&prompt);
            let record = MemoryRecord::new(
                MemoryType::Reflexion,
                "aureus".into(),
                "llm".into(),
                response,
                serde_json::json!({}),
            );
            let _ = store.add(record);
        } else {
            println!("[AureusBridge] No LLM client configured.");
        }
    }

    pub fn loops_run(&self) -> usize {
        self.loops
    }

    pub fn reset(&mut self) {
        self.loops = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_clients::mock::MockClient;

    #[test]
    fn run_loop_increments_counter() {
        let mut bridge = AureusBridge::with_client(Box::new(MockClient));
        let mut store = MemoryStore::new("test_bridge.jsonl").unwrap();
        store.clear();
        bridge.reflexion_loop("ctx", &mut store);
        assert_eq!(bridge.loops_run(), 1);
    }
}
