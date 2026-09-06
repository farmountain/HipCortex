//! WorldModelUpdater — closes the feedback loop by feeding Temporal observations
//! back into WorldModelEnhanced transition probabilities after each ReactEngine iteration.

use crate::memory_record::MemoryRecord;
use crate::world_model_enhanced::WorldModelEnhanced;

/// Feed a Temporal observation into the world model's Dirichlet-Multinomial transition model.
/// Extracts (from_state, action, to_state) from the record and calls observe_transition.
/// to_state precedence: metadata["to_state"] → metadata["status"] → "{target}_observed"
pub fn update_from_temporal(obs: &MemoryRecord, wm: &mut WorldModelEnhanced) {
    let action = obs
        .metadata
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("symbolic_step")
        .to_string();

    let from_state = obs.target.clone();
    let to_state = obs
        .metadata
        .get("to_state")
        .and_then(|v| v.as_str())
        .or_else(|| obs.metadata.get("status").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}_observed", from_state));

    let _ = wm.observe_transition(from_state, action, to_state);
}

/// Feed an AcceptReceipt outcome into the WM transition model.
/// Writes two transitions when ok=true:
///   (1) meta-probe: entity → probe → entity_{ok|failed}   (success-rate counter)
///   (2) domain:     entity → observe → entity:<obs_state>  (real P(s'|s,a) for the work)
/// The domain transition uses receipt.observation JSON to derive the actual observed state,
/// making this a genuine world-model update rather than a binary counter.
pub fn update_from_receipt(entity: &str, ok: bool, observation: &serde_json::Value, wm: &mut WorldModelEnhanced) {
    // (1) meta-probe transition — always written
    let probe_to = if ok { format!("{}_ok", entity) } else { format!("{}_failed", entity) };
    let _ = wm.observe_transition(entity.to_string(), "probe".to_string(), probe_to);

    // (2) domain transition — only on success with a usable observation
    if ok {
        let obs_state = observation
            .as_str()
            .map(|s| {
                let s = s.trim();
                if s.len() > 40 { format!("{}_observed", entity) } else { format!("{}:{}", entity, s) }
            })
            .or_else(|| {
                observation.get("status").and_then(|v| v.as_str())
                    .map(|s| format!("{}:{}", entity, s))
            })
            .unwrap_or_else(|| format!("{}_observed", entity));
        let _ = wm.observe_transition(entity.to_string(), "observe".to_string(), obs_state);
    }
}
