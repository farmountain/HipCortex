use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub agent_tags: Vec<String>,
    pub trigger_type: TriggerType,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    Message,
    Action,
    Observation,
    Custom(String),
}

fn default_weight() -> f32 {
    1.0
}
