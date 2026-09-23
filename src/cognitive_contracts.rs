//! CognitiveStateProvider / CognitiveStateSink — thin boundary traits.
//!
//! Chain-of-thought: KARM must never lock MemoryStore or WorldModelEnhanced
//! directly. These traits encapsulate the contract: read via Provider,
//! write via Sink. CognitiveHandle<B> implements both.

use crate::action_intent::{ActionIntent, ActionReceipt};
use crate::cognitive_state::{CognitiveError, CognitiveSnapshot};
use crate::transition_view::TransitionView;

/// Read contract for KARM: snapshot + settled transition history.
pub trait CognitiveStateProvider {
    fn provider_snapshot(&self, actor: &str) -> Result<CognitiveSnapshot, CognitiveError>;
    fn provider_transitions_since(&self, actor: &str, since_tx: u64) -> Vec<TransitionView>;
}

/// Write contract for KARM: submit Intent; accept Receipt.
pub trait CognitiveStateSink {
    fn sink_apply_intent(&self, intent: ActionIntent, actor: &str) -> Result<u64, CognitiveError>;
    fn sink_apply_receipt(&self, receipt: ActionReceipt, actor: &str) -> Result<u64, CognitiveError>;
}
