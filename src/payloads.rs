//! Typed payload helpers for MemoryRecord.metadata.
//! These structs serialize into/from the existing `metadata: serde_json::Value` field.
//! HipCortex stores them as opaque JSON — no execution logic lives here.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JTMS truth-maintenance label for a belief node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum JtmsLabel {
    In,
    Out,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessFactor {
    pub name: String,
    pub weight: f32,
    pub satisfied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalPayload {
    pub target_state: String,
    pub acceptance_criteria: Vec<String>,
    pub success_factors: Vec<SuccessFactor>,
    #[serde(default = "default_max_iterations")]
    pub max_react_iterations: u32,
    pub status: GoalStatus,
    #[serde(default)]
    pub current_iteration: u32,
}

fn default_max_iterations() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPayload {
    pub procedure: String,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub expected_outcomes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum EpistemicStatus {
    Observed,
    Deduced,
    #[default]
    Hypothetical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefPayload {
    pub proposition: String,
    #[serde(default)]
    pub justification: String,
    /// IDs of MemoryRecords this belief contradicts.
    #[serde(default)]
    pub contradicts: Vec<Uuid>,

    // v0.7.0 additions — all #[serde(default)] for backward compat
    #[serde(default = "default_belief_confidence")]
    pub confidence: f32,
    #[serde(default)]
    pub epistemic_status: EpistemicStatus,
    #[serde(default)]
    pub causal_source_ids: Vec<Uuid>,
    #[serde(default)]
    pub half_life_ms: u64,
    #[serde(default)]
    pub tx_origin: Option<u64>,

    // JTMS fields — all default for backward compat
    /// Truth-maintenance label.
    #[serde(default)]
    pub jtms_label: JtmsLabel,
    /// Belief/record IDs this belief depends on (must all be In for this to be In).
    #[serde(default)]
    pub in_list: Vec<Uuid>,
    /// Belief IDs that must be Out for this to be In.
    #[serde(default)]
    pub out_list: Vec<Uuid>,
    /// Belief IDs that depend on THIS belief (back-pointers for cascade).
    #[serde(default)]
    pub dependents: Vec<Uuid>,
}

fn default_belief_confidence() -> f32 {
    0.5
}
