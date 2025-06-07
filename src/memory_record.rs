use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    #[serde(default)]
    pub integrity: Option<String>,
}

impl MemoryRecord {
    pub fn new(
        record_type: MemoryType,
        actor: String,
        action: String,
        target: String,
        metadata: serde_json::Value,
    ) -> Self {
        let mut rec = Self {
            id: Uuid::new_v4(),
            record_type,
            timestamp: Utc::now(),
            actor,
            action,
            target,
            metadata,
            integrity: None,
        };
        let hash = rec.compute_hash();
        rec.integrity = Some(hash);
        rec
    }

    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut clone = self.clone();
        clone.integrity = None;
        let data = serde_json::to_vec(&clone).unwrap();
        let hash = Sha256::digest(&data);
        hex::encode(hash)
    }
}
