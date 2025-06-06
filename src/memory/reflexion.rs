use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexionSnapshot {
    pub id: Uuid,
    pub input_trace: Option<Uuid>,
    pub thoughts: Vec<String>,
    pub outcome: Option<String>,
    pub feedback: Option<f32>,
}
