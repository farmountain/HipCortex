use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicNode {
    pub id: Uuid,
    pub labels: Vec<String>,
    #[serde(default = "default_weight")]
    pub weight: f32,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

fn default_weight() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicEdge {
    pub source: Uuid,
    pub target: Uuid,
    pub relation: RelationType,
    #[serde(default = "default_weight")]
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationType {
    AssociatedWith,
    PartOf,
    Causes,
    Custom(String),
}
