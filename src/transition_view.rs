//! TransitionView and PredictionError — derived read models for KARM.
//!
//! Chain-of-thought: KARM needs to reconstruct what transitions happened
//! since a given cursor. Both models are derived from existing data
//! (open_intents Vec + CalibrationTracker) — no second store.
//!
//! v3.18.0 makes the cursor real:
//! 1. A settle (receipt or expiry) stamps the intent with the transaction that
//!    settled it (`ActionIntent::settled_tx`); only stamped intents are transitions.
//! 2. Stamps are allocated under the `open_intents` lock, so a reader sees a prefix
//!    of the stamp sequence and paging by `settled_tx > since` is gap-free.
//! 3. Retention keeps the newest `cap` settled intents; the newest evicted stamp is
//!    the horizon. A cursor behind the horizon may have missed evicted transitions,
//!    so its page is flagged `truncated` and the consumer re-bootstraps from a snapshot.

use crate::action_intent::{ActionIntent, IntentStatus};
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
    /// true when status == Received and the receipt reported success
    pub ok: bool,
    /// "Received" | "Expired" — the settled state of the intent
    pub status: String,
    pub timestamp: DateTime<Utc>,
    /// Transaction that settled the intent — the feed's cursor unit.
    #[serde(default)]
    pub settled_tx: u64,
}

impl TransitionView {
    /// `None` for an intent that has not settled.
    pub fn from_intent(intent: &ActionIntent) -> Option<Self> {
        let settled_tx = intent.settled_tx?;
        Some(Self {
            intent_id: intent.id,
            action: intent.op.clone(),
            target: intent.target_entity.clone().unwrap_or_default(),
            actor: intent.actor.clone(),
            // Settles recorded before 3.18 carry no receipt_ok; they were successes.
            ok: intent.status == IntentStatus::Received && intent.receipt_ok.unwrap_or(true),
            status: format!("{:?}", intent.status),
            timestamp: intent.created_tx,
            settled_tx,
        })
    }
}

/// One page of the transition feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionPage {
    /// Settled transitions with `settled_tx > since`, ascending by `settled_tx`.
    pub transitions: Vec<TransitionView>,
    /// Pass as `since` on the next call: `max(since, last settled_tx)`.
    pub next_cursor: u64,
    /// Newest stamp evicted by retention (the TxLog cursor at startup, before any).
    pub horizon_tx: u64,
    /// `since < horizon_tx`: transitions after `since` may have been evicted, so the
    /// consumer must re-bootstrap from `snapshot()`.
    pub truncated: bool,
}

impl TransitionPage {
    pub fn new(mut transitions: Vec<TransitionView>, since: u64, horizon_tx: u64) -> Self {
        transitions.sort_by_key(|t| t.settled_tx);
        let next_cursor = transitions.last().map_or(since, |t| t.settled_tx.max(since));
        Self {
            transitions,
            next_cursor,
            horizon_tx,
            truncated: since < horizon_tx,
        }
    }
}

/// Drops the oldest-stamped settled intents beyond `cap`; unstamped intents are kept.
/// Returns the newest evicted stamp, or `None` when nothing was evicted.
///
/// Stamps are unique (each settle allocates its own transaction), so exactly
/// `settled - cap` intents are dropped.
pub fn evict_settled(intents: &mut Vec<ActionIntent>, cap: usize) -> Option<u64> {
    let mut stamps: Vec<u64> = intents.iter().filter_map(|i| i.settled_tx).collect();
    if stamps.len() <= cap {
        return None;
    }
    let excess = stamps.len() - cap;
    let (_, cutoff, _) = stamps.select_nth_unstable(excess - 1);
    let cutoff = *cutoff;
    intents.retain(|i| !matches!(i.settled_tx, Some(s) if s <= cutoff));
    Some(cutoff)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionError {
    pub global_ewma: f32,
    /// true when prediction_error_ewma > 0.3 (KARM rotation signal)
    pub uncertain: bool,
    pub timestamp: DateTime<Utc>,
}
