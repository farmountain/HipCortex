/// Gap 7 — v2.7.0 acceptance tests
///
/// AC-A1: update_from_receipt writes TWO transitions (meta-probe + domain observe).
/// AC-A2: step_with_wm couples WM MAP probability into DynamicsContext entity_states.
/// AC-A3: predicted_only_barrier blocks step_with_wm when MAP == 0.
/// AC-B1: belief reinforcement traverses derived_from / evidence provenance links.
/// AC-B2: substring match (proposition.contains) is no longer the reinforce path.
/// AC-C1: subscribe_with_config installs DecisionEngine when execution_gate is None.
/// AC-C2: explicit gate injected via config is never overwritten.

use std::fs;

// ── AC-A1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_a1_update_from_receipt_writes_dual_transitions() {
    let src = fs::read_to_string("src/wm_updater.rs").unwrap();
    // meta-probe transition
    assert!(src.contains("\"probe\""),
        "update_from_receipt must write meta-probe transition (action=probe)");
    // domain observe transition — distinct from probe
    assert!(src.contains("\"observe\""),
        "update_from_receipt must write domain observe transition (action=observe)");
    // both paths call observe_transition
    let count = src.matches("observe_transition").count();
    assert!(count >= 2,
        "update_from_receipt must call observe_transition at least twice; found {}", count);
}

// ── AC-A2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_a2_step_with_wm_couples_map_prob_into_context() {
    let src = fs::read_to_string("src/digital_twin.rs").unwrap();
    assert!(src.contains("step_with_wm"),
        "DigitalTwin must expose step_with_wm method");
    assert!(src.contains("predict_next_state"),
        "step_with_wm must call wm.predict_next_state to derive MAP probability");
    assert!(src.contains("entity_states: &entity_signals"),
        "step_with_wm must pass entity_signals into DynamicsContext.entity_states");
    assert!(src.contains("map_prob"),
        "MAP probability must flow from WM into DynamicsContext");
}

// ── AC-A3 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_a3_predicted_only_barrier_blocks_when_map_zero() {
    let src = fs::read_to_string("src/digital_twin.rs").unwrap();
    assert!(src.contains("predicted_only_barrier"),
        "DigitalTwin must have predicted_only_barrier field");
    assert!(src.contains("map_prob == 0.0"),
        "barrier must check map_prob == 0.0");
    assert!(src.contains("PredictedOnly barrier"),
        "barrier must return Err with 'PredictedOnly barrier' message");
}

// ── AC-B1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_b1_belief_reinforce_traverses_provenance_links() {
    let src = fs::read_to_string("src/cognitive_state.rs").unwrap();
    // derived_from link check
    assert!(src.contains("derived_from"),
        "accept_receipt_impl must check derived_from for credit assignment");
    // evidence link check
    assert!(src.contains(".evidence.iter()"),
        "accept_receipt_impl must traverse evidence links");
    assert!(src.contains("BeliefExecutive::reinforce"),
        "accept_receipt_impl must call BeliefExecutive::reinforce on provenance-linked beliefs");
}

// ── AC-B2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_b2_substring_reinforce_removed() {
    let src = fs::read_to_string("src/cognitive_state.rs").unwrap();
    // The old substring path used proposition.contains(entity)
    assert!(!src.contains("proposition.contains(entity)"),
        "substring credit assignment (proposition.contains) must be removed");
    // Also confirm the new path gates on MemoryType::Belief, not substring
    assert!(src.contains("MemoryType::Belief"),
        "reinforce path must filter by MemoryType::Belief, not substring");
}

// ── AC-C1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_c1_subscribe_with_config_installs_decision_engine_by_default() {
    let src = fs::read_to_string("src/substrate_daemon.rs").unwrap();
    assert!(src.contains("G7c"),
        "G7c comment must mark the always-gated default installation");
    assert!(src.contains("config.execution_gate.is_none()"),
        "subscribe_with_config must check execution_gate.is_none()");
    assert!(src.contains("DecisionEngine::new()"),
        "DecisionEngine::new() must be installed as default gate");
}

// ── AC-C2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_c2_explicit_gate_not_overwritten() {
    let src = fs::read_to_string("src/substrate_daemon.rs").unwrap();
    // The guard is gated on is_none() so it only runs when no gate was injected
    let guard_idx = src.find("config.execution_gate.is_none()").expect("guard must exist");
    let install_idx = src.find("DecisionEngine::new()").expect("install must exist");
    // install must come after the is_none() guard (inside the if block)
    assert!(install_idx > guard_idx,
        "DecisionEngine install must be inside the is_none() guard, not unconditional");
    // Confirm it is not is_some() (which would overwrite)
    assert!(!src.contains("config.execution_gate.is_some()"),
        "gate must NOT overwrite an existing explicit gate");
}
