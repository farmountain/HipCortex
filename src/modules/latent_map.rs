use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Versioned latent map treated as a world model snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatentMapVersion {
    pub map: Value,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

impl LatentMapVersion {
    /// Create a new version with the current timestamp.
    pub fn new(map: Value, confidence: f32) -> Self {
        Self {
            map,
            confidence,
            timestamp: Utc::now(),
        }
    }
}
