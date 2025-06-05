use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use clap::ValueEnum;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
pub enum MemoryType {
    Temporal,
    Symbolic,
    Procedural,
    Reflexion,
    Perception,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: Uuid,
    pub record_type: MemoryType,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub target: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl MemoryRecord {
    pub fn new(record_type: MemoryType, actor: String, action: String, target: String, metadata: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            record_type,
            timestamp: Utc::now(),
            actor,
            action,
            target,
            metadata,
        }
    }
}
