use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralPolicy {
    pub id: Uuid,
    pub current_state: String,
    pub trigger_condition: Option<String>,
    pub effect: Option<String>,
    pub next_state: Option<String>,
    pub version: i32,
    pub reward: Option<f32>,
}
