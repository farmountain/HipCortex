//! WorldModelUpdater — closes the feedback loop by feeding Temporal observations
//! back into WorldModelEnhanced transition probabilities after each ReactEngine iteration.

use crate::memory_record::MemoryRecord;
use crate::world_model_enhanced::WorldModelEnhanced;

/// Feed a Temporal observation into the world model's Dirichlet-Multinomial transition model.
/// Extracts (from_state, action, to_state) from the record and calls observe_transition.
pub fn update_from_temporal(obs: &MemoryRecord, wm: &mut WorldModelEnhanced) {
    let action = obs
        .metadata
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("symbolic_step")
        .to_string();

    let from_state = obs.target.clone();
    let iteration = obs.react_iteration.unwrap_or(0);
    // to_state encodes the observed outcome at this iteration
    let to_state = format!("{}_iter_{}", from_state, iteration);

    let _ = wm.observe_transition(from_state, action, to_state);
}
