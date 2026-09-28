//! CognitiveStateProvider / CognitiveStateSink — thin boundary traits.
//!
//! Chain-of-thought: KARM must never lock MemoryStore or WorldModelEnhanced
//! directly. These traits encapsulate the contract: read via Provider,
//! write via Sink. CognitiveHandle<B> implements both.
//!
//! v3.18.0 adds defaulted methods only, so a v3.17 implementor still compiles:
//! its page is built from `provider_transitions_since` with no retention horizon,
//! and it serves no SCM.

use crate::action_intent::{ActionIntent, ActionReceipt};
use crate::cognitive_state::{CognitiveError, CognitiveSnapshot};
use crate::scm_view::ScmView;
use crate::transition_view::{TransitionPage, TransitionView};

/// Read contract for KARM: snapshot + settled transition history.
pub trait CognitiveStateProvider {
    fn provider_snapshot(&self, actor: &str) -> Result<CognitiveSnapshot, CognitiveError>;
    fn provider_transitions_since(&self, actor: &str, since_tx: u64) -> Vec<TransitionView>;

    /// One page of the transition feed; see [`TransitionPage`].
    fn provider_transitions_page(&self, actor: &str, since_tx: u64) -> TransitionPage {
        TransitionPage::new(self.provider_transitions_since(actor, since_tx), since_tx, 0)
    }

    /// Read model of the causal SCM, if the provider has one.
    fn provider_scm(&self) -> Option<ScmView> {
        None
    }
}

/// Write contract for KARM: submit Intent; accept Receipt.
pub trait CognitiveStateSink {
    fn sink_apply_intent(&self, intent: ActionIntent, actor: &str) -> Result<u64, CognitiveError>;
    fn sink_apply_receipt(&self, receipt: ActionReceipt, actor: &str) -> Result<u64, CognitiveError>;
}
