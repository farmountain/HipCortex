use super::{LLMClient, LanguageModelClient};
use std::process::Command;

/// Simple connector for a local command line model.
pub struct LocalLLMClient {
    pub command: String,
}

impl LocalLLMClient {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    fn run(&self, arg: &str) -> String {
        Command::new(&self.command)
            .arg(arg)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    }
}

impl LanguageModelClient for LocalLLMClient {
    fn generate(&self, prompt: &str) -> String {
        self.run(prompt)
    }

    fn embed(&self, _text: &str) -> Vec<f32> {
        Vec::new()
    }

    fn chat(&self, history: Vec<String>, new_input: &str) -> String {
        let mut joined = history.join("\n");
        if !joined.is_empty() {
            joined.push_str("\n");
        }
        joined.push_str(new_input);
        self.run(&joined)
    }
}

impl LLMClient for LocalLLMClient {
    fn generate_response(&self, prompt: &str) -> String {
        self.generate(prompt)
    }
}
