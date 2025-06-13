use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub sender: String,
    pub text: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenManusMessage {
    pub role: String,
    pub content: String,
}

pub struct ConversationMemory {
    messages: Vec<ConversationMessage>,
}

impl ConversationMemory {
    pub fn new() -> Self {
        Self { messages: Vec::new() }
    }

    pub fn add_message(&mut self, sender: &str, text: &str) -> Uuid {
        let msg = ConversationMessage {
            id: Uuid::new_v4(),
            sender: sender.to_string(),
            text: text.to_string(),
            timestamp: Utc::now(),
        };
        self.messages.push(msg.clone());
        msg.id
    }

    pub fn ingest_openmanus(&mut self, json: &str) -> Result<Uuid> {
        let m: OpenManusMessage = serde_json::from_str(json)?;
        Ok(self.add_message(&m.role, &m.content))
    }

    pub fn messages(&self) -> &[ConversationMessage] {
        &self.messages
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}
