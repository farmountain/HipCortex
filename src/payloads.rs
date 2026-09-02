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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum GoalStatus {
    #[default]
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Abandoned,
}

/// Controls how the daemon drives a goal through its ReAct iterations.
/// `FullCycle` (default): `ReactEngine::run()` exhausts all iterations in one daemon tick.
/// `StepByStep`: daemon advances exactly one ReAct iteration per tick via `run_one_step()`,
/// keeping the goal InProgress across ticks — enables CriticGate veto at iter ≥ 1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum GoalExecutionMode {
    #[default]
    FullCycle,
    StepByStep,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalPayload {
    pub target_state: String,
    pub acceptance_criteria: Vec<String>,
    pub success_factors: Vec<SuccessFactor>,
    #[serde(default = "default_max_iterations")]
    pub max_react_iterations: u32,
    pub status: GoalStatus,
    #[serde(default)]
    pub current_iteration: u32,
    /// FullCycle (default) = entire loop per daemon tick; StepByStep = one iter per tick.
    #[serde(default)]
    pub execution_mode: GoalExecutionMode,
    /// 0.0–1.0 priority signal; higher = schedule sooner. Default 0.5.
    #[serde(default = "default_urgency")]
    pub urgency: f64,
    /// Estimated resource cost (relative units). Lower = cheaper. Default 1.0.
    #[serde(default = "default_cost")]
    pub estimated_cost: f64,
}

fn default_max_iterations() -> u32 {
    10
}

fn default_urgency() -> f64 {
    0.5
}

fn default_cost() -> f64 {
    1.0
}

/// Payload for MemoryType::Decision — records an act-phase choice with rationale.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionPayload {
    /// The action chosen (e.g. "symbolic_step", "llm_call").
    pub option_chosen: String,
    /// Alternatives considered but rejected.
    #[serde(default)]
    pub alternatives: Vec<String>,
    /// IDs of Temporal/Belief records that informed this decision.
    #[serde(default)]
    pub rationale: Vec<Uuid>,
    /// Ordered human-readable steps explaining the choice ("observed X → inferred Y → chose Z").
    #[serde(default)]
    pub rationale_chain: Vec<String>,
    /// Confidence in the chosen option (0.0–1.0).
    #[serde(default = "default_decision_confidence")]
    pub confidence: f64,
    /// ID of the Temporal observation that resulted from this decision (back-filled).
    #[serde(default)]
    pub outcome: Option<Uuid>,
}

fn default_decision_confidence() -> f64 {
    0.7
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
