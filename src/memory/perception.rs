use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Modality {
    Text,
    Image,
    Embedding,
    Symbol,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionInput {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: Option<String>,
    pub modality: Modality,
    pub data: serde_json::Value,
    pub meaning: Option<String>,
}
