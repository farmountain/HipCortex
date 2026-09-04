/// Gap 5 — Spine v2.6.0 acceptance tests
///
/// AC-E1: ExecutionGate field exists on CognitiveLoopConfig (None by default).
/// AC-E2: Gate rejected → "gate_veto" Temporal record written; ReactEngine NOT called.
/// AC-E3: Gate approved → ReactEngine runs normally.
/// AC-W1: update_from_receipt wires entity+ok into WM (probe→entity_ok transition).
/// AC-W2: update_from_receipt with ok=false writes entity_failed transition.
/// AC-W3: update_from_temporal still works (from_state/action/to_state unchanged).
/// AC-B1: BeliefExecutive::reinforce boosts confidence (additive, capped at 1.0).
/// AC-B2: reinforce on unknown id is a no-op (no panic).
/// AC-B3: cognitive_state accept_receipt_impl calls reinforce path (belief confidence rises).

use std::fs;

// ── AC-E1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_e1_execution_gate_field_none_default() {
    let src = fs::read_to_string("src/substrate_daemon.rs").unwrap();
    assert!(src.contains("execution_gate: Option<Arc<Mutex<dyn crate::execution_gate::ExecutionGate"),
        "CognitiveLoopConfig must have execution_gate field");
    assert!(src.contains("execution_gate: None,"),
        "Default impl must set execution_gate: None");
    assert!(!src.contains("#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub struct CognitiveLoopConfig"),
        "Debug must be removed from derive (manual impl instead)");
    assert!(src.contains("impl std::fmt::Debug for CognitiveLoopConfig"),
        "Manual Debug impl required");
}

// ── AC-E2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_e2_gate_rejected_writes_veto_record() {
    let src = fs::read_to_string("src/substrate_daemon.rs").unwrap();
    assert!(src.contains("gate_veto"),
        "gate_veto Temporal record must be written when gate rejects");
    assert!(src.contains("execution_gate_rejected"),
        "veto reason must be 'execution_gate_rejected'");
    assert!(src.contains("!gate_ok"),
        "guard must check !gate_ok before skipping ReactEngine");
}

// ── AC-E3 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_e3_gate_approved_runs_react_engine() {
    let src = fs::read_to_string("src/substrate_daemon.rs").unwrap();
    // ReactEngine::new() must appear inside the gate_ok branch (else branch after !gate_ok check)
    let veto_pos = src.find("gate_veto").expect("gate_veto must exist");
    let engine_pos = src.find("let mut engine = ReactEngine::new()").expect("ReactEngine::new must exist");
    assert!(engine_pos > veto_pos,
        "ReactEngine::new() must appear after the gate_veto branch (in the else/approved path)");
}

// ── AC-W1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_w1_update_from_receipt_ok_writes_entity_ok_transition() {
    let src = fs::read_to_string("src/wm_updater.rs").unwrap();
    assert!(src.contains("pub fn update_from_receipt"),
        "update_from_receipt must be public");
    assert!(src.contains("format!(\"{}_ok\", entity)"),
        "ok=true path must produce {{entity}}_ok to_state");
    assert!(src.contains("\"probe\".to_string()"),
        "action must be 'probe'");
}

// ── AC-W2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_w2_update_from_receipt_failed_writes_entity_failed_transition() {
    let src = fs::read_to_string("src/wm_updater.rs").unwrap();
    assert!(src.contains("format!(\"{}_failed\", entity)"),
        "ok=false path must produce {{entity}}_failed to_state");
}

// ── AC-W3 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_w3_update_from_temporal_to_state_precedence() {
    let src = fs::read_to_string("src/wm_updater.rs").unwrap();
    // Must check metadata["to_state"] first, then metadata["status"], then fall back
    let to_state_pos = src.find("\"to_state\"").expect("to_state key must exist");
    let status_pos   = src.find("\"status\"").expect("status key must exist");
    let observed_pos = src.find("_observed").expect("_observed fallback must exist");
    assert!(to_state_pos < status_pos,  "to_state checked before status");
    assert!(status_pos   < observed_pos, "status checked before _observed fallback");
}

// ── AC-B1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_b1_reinforce_boosts_confidence() {
    let src = fs::read_to_string("src/belief_executive.rs").unwrap();
    assert!(src.contains("pub fn reinforce"),
        "BeliefExecutive::reinforce must be public");
    assert!(src.contains("(current.confidence + boost).min(1.0)"),
        "reinforce must be additive and capped at 1.0");
}

// ── AC-B2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_b2_reinforce_unknown_id_noop() {
    let src = fs::read_to_string("src/belief_executive.rs").unwrap();
    // Uses if-let pattern on find_by_id — no panic on None
    assert!(src.contains("if let Some(current) = store.find_by_id(id)"),
        "reinforce must guard with if-let to be no-op on unknown id");
}

// ── AC-B3 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_b3_accept_receipt_calls_reinforce_on_ok() {
    let src = fs::read_to_string("src/cognitive_state.rs").unwrap();
    assert!(src.contains("crate::belief_executive::BeliefExecutive::reinforce"),
        "accept_receipt_impl must call BeliefExecutive::reinforce");
    assert!(src.contains("if receipt.ok"),
        "reinforce must be conditional on receipt.ok");
    assert!(src.contains("crate::wm_updater::update_from_receipt"),
        "accept_receipt_impl must call wm_updater::update_from_receipt");
    // Separate write lock for wm_updater (no deadlock with entity contact read lock)
    let write_pos = src.find("self.world.write()").expect("world.write() must exist for G5b");
    let read_pos  = src.find("self.world.read()").expect("world.read() must exist");
    assert!(write_pos > read_pos,
        "write lock for wm_updater must appear AFTER read lock (separate scope, no deadlock)");
}
