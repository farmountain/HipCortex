//! TransitionView and PredictionError — derived read models for KARM.
//!
//! Chain-of-thought: KARM needs to reconstruct what transitions happened
//! since a given cursor. Both models are derived from existing data
//! (open_intents Vec + CalibrationTracker) — no second store.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionView {
    pub intent_id: Uuid,
    /// op field from ActionIntent (e.g. "probe_entity:filesystem")
    pub action: String,
    /// target_entity from ActionIntent, empty string if None
    pub target: String,
    pub actor: String,
    /// true when status == Received (host confirmed success)
    pub ok: bool,
    /// "Open" | "InFlight" | "Received" | "Expired" | "Denied"
    pub status: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionError {
    pub global_ewma: f32,
    /// true when prediction_error_ewma > 0.3 (KARM rotation signal)
    pub uncertain: bool,
    pub timestamp: DateTime<Utc>,
}
