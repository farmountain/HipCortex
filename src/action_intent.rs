//! Intent / Receipt seam (v2.3.0) — HipCortex's only env API.
//!
//! Chain-of-thought:
//!   HipCortex never calls tools. It writes ActionIntent tickets and waits.
//!   The host runner reads Open intents, executes them, and posts ActionReceipts.
//!   AcceptReceipt in CognitiveDelta is the only path that turns a receipt into
//!   a Temporal observation + WM contact update. No code path bypasses this gate.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Contact tracking ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ContactKind {
    /// Entity mean filled in by Kalman / twin only — no host receipt.
    #[default]
    PredictedOnly,
    /// Host delivered a receipt; real sensor touched this entity.
    Observed,
    /// Host attempted probe but reported ok=false.
    ProbeFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum GroundingStatus {
    /// No real observation ever received.
    #[default]
    Virgin,
    /// 1–3 observations — beginning to form a picture.
    Sketch,
    /// ≥4 observations — entity is reliably tracked.
    Mapped,
    /// Was Mapped/Sketch but last_contact > STALE_THRESHOLD_S ago.
    Stale,
    /// OOD anomaly flagged on this entity.
    Anomalous,
}

pub const STALE_THRESHOLD_S: i64 = 3600; // 1 hour without contact → Stale
pub const MAPPED_OBS_THRESHOLD: u32 = 4; // n_observations ≥ this → Mapped

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityContactRecord {
    #[serde(default)]
    pub last_contact_tx: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_contact_kind: ContactKind,
    #[serde(default)]
    pub n_observations: u32,
    #[serde(default)]
    pub probe_count: u32,
    #[serde(default)]
    pub grounding_status: GroundingStatus,
}

impl EntityContactRecord {
    /// Seconds since last real contact. None if never contacted.
    pub fn staleness_s(&self) -> Option<i64> {
        self.last_contact_tx.map(|ts| (Utc::now() - ts).num_seconds())
    }

    /// Epistemic uncertainty: 1/√(n+1). Higher = less known.
    pub fn epistemic(&self) -> f32 {
        1.0 / (self.n_observations as f32 + 1.0).sqrt()
    }

    /// Information-gain probe score: epistemic × deficit × probe_penalty.
    /// Entities at or above MAPPED_OBS_THRESHOLD return 0.0 — already grounded, skip.
    /// Higher score = higher priority probe target (directional toward coverage).
    pub fn ig_score(&self) -> f32 {
        let deficit = (1.0 - self.n_observations as f32 / MAPPED_OBS_THRESHOLD as f32).max(0.0);
        if deficit == 0.0 {
            return 0.0;
        }
        let probe_penalty = 1.0 / (1.0 + self.probe_count as f32 * 0.2);
        self.epistemic() * deficit * probe_penalty
    }

    /// Apply a new observation receipt, updating status fields.
    pub fn apply_observation(&mut self) {
        self.last_contact_tx = Some(Utc::now());
        self.last_contact_kind = ContactKind::Observed;
        self.n_observations += 1;
        self.grounding_status = if self.n_observations >= MAPPED_OBS_THRESHOLD {
            GroundingStatus::Mapped
        } else {
            GroundingStatus::Sketch
        };
    }

    pub fn apply_probe_failed(&mut self) {
        self.last_contact_tx = Some(Utc::now());
        self.last_contact_kind = ContactKind::ProbeFailed;
        self.probe_count += 1;
    }

    /// Recheck staleness; call periodically or on snapshot load.
    pub fn refresh_staleness(&mut self) {
        if let Some(staleness) = self.staleness_s() {
            if staleness > STALE_THRESHOLD_S
                && matches!(self.grounding_status, GroundingStatus::Mapped | GroundingStatus::Sketch)
            {
                self.grounding_status = GroundingStatus::Stale;
            }
        }
    }
}

// ─── Intent ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntentKind {
    /// Gather information about a specific entity (reversible, cheap).
    Probe,
    /// Make a change in the environment (only allowed after grounding exits).
    Instrumental,
    /// Ask the host/user for a clarifying observation.
    ClarifySense,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntentStatus {
    Open,
    InFlight,
    Received,
    Expired,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionIntent {
    pub id: Uuid,
    #[serde(default)]
    pub goal_id: Option<Uuid>,
    pub actor: String,
    pub kind: IntentKind,
    /// Opaque operation name, e.g. "probe_entity:filesystem" or "write_file".
    pub op: String,
    #[serde(default)]
    pub args: serde_json::Value,
    /// Which WM entity this probe/action targets (for contact tracking).
    #[serde(default)]
    pub target_entity: Option<String>,
    /// How long the host has to fulfil this intent before it expires (ms).
    pub deadline_ms: u64,
    pub deadline_tx: DateTime<Utc>,
    pub status: IntentStatus,
    pub created_tx: DateTime<Utc>,
}

impl ActionIntent {
    pub fn new_probe(
        actor: String,
        entity: String,
        goal_id: Option<Uuid>,
        deadline_ms: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            goal_id,
            actor,
            kind: IntentKind::Probe,
            op: format!("probe_entity:{entity}"),
            args: serde_json::json!({ "entity": entity }),
            target_entity: Some(entity),
            deadline_ms,
            deadline_tx: now + Duration::milliseconds(deadline_ms as i64),
            status: IntentStatus::Open,
            created_tx: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        matches!(self.status, IntentStatus::Open | IntentStatus::InFlight)
            && self.deadline_tx < Utc::now()
    }
}

// ─── Receipt ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionReceipt {
    /// Matches ActionIntent.id.
    pub intent_id: Uuid,
    /// Whether the host operation succeeded.
    pub ok: bool,
    /// Raw payload the host sensor produced.
    pub observation: serde_json::Value,
    /// Which MCP server / skill produced this observation.
    pub sensor_path: String,
    pub ts: DateTime<Utc>,
}

// ─── ActuatorRegistry ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorEntry {
    /// Unique logical name (matches sensor_path in receipts).
    pub name: String,
    /// Transport hint for the runner ("mcp:playwright", "mcp:filesystem", …).
    pub transport_hint: String,
    #[serde(default)]
    pub last_heartbeat: Option<DateTime<Utc>>,
    /// Last error string from a failed receipt, if any.
    #[serde(default)]
    pub last_error: Option<String>,
    /// Default probe op for this actuator, e.g. "probe_entity:*".
    #[serde(default)]
    pub probe_op: Option<String>,
}

impl ActuatorEntry {
    pub fn is_healthy(&self) -> bool {
        self.last_error.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActuatorRegistry {
    pub actuators: Vec<ActuatorEntry>,
}

impl ActuatorRegistry {
    pub fn register(&mut self, entry: ActuatorEntry) {
        if !self.actuators.iter().any(|a| a.name == entry.name) {
            self.actuators.push(entry);
        }
    }

    pub fn find(&self, name: &str) -> Option<&ActuatorEntry> {
        self.actuators.iter().find(|a| a.name == name)
    }

    pub fn any_healthy(&self) -> bool {
        self.actuators.iter().any(|a| a.is_healthy())
    }

    /// Update heartbeat from a received receipt.
    pub fn apply_receipt(&mut self, sensor_path: &str, ts: DateTime<Utc>, ok: bool) {
        if let Some(a) = self
            .actuators
            .iter_mut()
            .find(|a| a.name == sensor_path || a.transport_hint.contains(sensor_path))
        {
            a.last_heartbeat = Some(ts);
            if ok {
                a.last_error = None;
            } else {
                a.last_error = Some("last receipt failed".into());
            }
        }
    }
}
