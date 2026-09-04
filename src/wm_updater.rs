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
/// from_state=entity, action="probe", to_state="{entity}_ok" or "{entity}_failed".
/// This lets the WM learn probe success rates per entity over time.
pub fn update_from_receipt(entity: &str, ok: bool, wm: &mut WorldModelEnhanced) {
    let to_state = if ok {
        format!("{}_ok", entity)
    } else {
        format!("{}_failed", entity)
    };
    let _ = wm.observe_transition(entity.to_string(), "probe".to_string(), to_state);
}
