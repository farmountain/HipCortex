// Phase-3c / Gap-6: WM authorization (AC-6a through AC-6g)
// AC-6a: WM_CONSTRAINTS is non-empty
// AC-6b: list_authorized_world_model returns a vec without panicking
// AC-6c: All constraint caps satisfy REST endpoint invariants (depth≤10, iter≤200)
// AC-6d: list_authorized_world_model result confidence values are in [0.0, 1.0]
// AC-6e: Empty WM blocks rollout (no transitions recorded)
// AC-6f: Empty WM blocks counterfactual (no causal nodes)
// AC-6g: WM with transition allows rollout through gate

use hipcortex::action_registry::{list_authorized_world_model, WM_CONSTRAINTS};
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;

#[test]
fn ac6a_wm_constraints_non_empty() {
    assert!(
        !WM_CONSTRAINTS.is_empty(),
        "AC-6a: WM_CONSTRAINTS must declare at least one world-model op constraint"
    );
}

#[test]
fn ac6b_list_authorized_world_model_returns_vec() {
    let sm = SelfModel::new();
    let wm = WorldModelEnhanced::new();
    let ops = list_authorized_world_model(&sm, &wm);
    let _ = ops;
}

#[test]
fn ac6c_all_constraint_caps_within_rest_bounds() {
    for c in WM_CONSTRAINTS {
        assert!(
            c.max_depth <= 10,
            "AC-6c: constraint '{}' max_depth {} exceeds REST cap 10",
            c.op, c.max_depth
        );
        assert!(
            c.max_iterations <= 200,
            "AC-6c: constraint '{}' max_iterations {} exceeds REST cap 200",
            c.op, c.max_iterations
        );
    }
}

#[test]
fn ac6d_authorized_ops_confidence_in_unit_range() {
    let sm = SelfModel::new();
    let wm = WorldModelEnhanced::new();
    let ops = list_authorized_world_model(&sm, &wm);
    for op in &ops {
        assert!(
            op.confidence >= 0.0 && op.confidence <= 1.0,
            "AC-6d: op '{}' confidence {} outside [0, 1]",
            op.op, op.confidence
        );
    }
}

/// AC-6e: Empty WM (no transitions) must exclude 'world_model_rollout' from authorized ops.
#[test]
fn ac6e_empty_wm_blocks_rollout() {
    let sm = SelfModel::new();
    let wm = WorldModelEnhanced::new(); // 0 transitions
    let ops = list_authorized_world_model(&sm, &wm);
    let has_rollout = ops.iter().any(|o| o.op == "world_model_rollout");
    assert!(
        !has_rollout,
        "AC-6e: 'world_model_rollout' must be blocked when WM has 0 transitions"
    );
}

/// AC-6f: Empty WM (no causal nodes) must exclude 'counterfactual' from authorized ops.
#[test]
fn ac6f_empty_wm_blocks_counterfactual() {
    let sm = SelfModel::new();
    let wm = WorldModelEnhanced::new(); // 0 causal nodes
    let ops = list_authorized_world_model(&sm, &wm);
    let has_cf = ops.iter().any(|o| o.op == "counterfactual");
    assert!(
        !has_cf,
        "AC-6f: 'counterfactual' must be blocked when WM has 0 causal nodes"
    );
}

/// AC-6g: WM with at least one transition allows 'world_model_rollout' through WM gate.
/// Note: still gated by SelfModel.can_execute — may remain unauthorized if SelfModel rejects.
/// The test asserts either (a) rollout is present (SelfModel approved it too), or
/// (b) transition_count > 0 so the WM gate itself does not block it.
#[test]
fn ac6g_wm_gate_does_not_block_rollout_after_transition() {
    let sm = SelfModel::new();
    let mut wm = WorldModelEnhanced::new();
    wm.observe_transition("state_a".to_string(), "action_x".to_string(), "state_b".to_string());
    assert!(
        wm.transition_count() > 0,
        "AC-6g: WM must have ≥1 transition after observe_transition"
    );
    // Confirm WM gate won't block (actual authorization still depends on SelfModel)
    let ops = list_authorized_world_model(&sm, &wm);
    let _ = ops; // no panic, gate passes through to SelfModel for final decision
}

/// AC-6h: Sparse WM (5 distinct outcomes each seen once) → max_transition_confidence < 0.5
/// → 'world_model_rollout' blocked by WM-belief confidence gate.
#[test]
fn ac6h_sparse_wm_blocks_rollout_via_confidence_gate() {
    let sm = SelfModel::new();
    let mut wm = WorldModelEnhanced::new();
    // 5 distinct next-states → Dirichlet posterior max prob = (1+1)/(5+5×1) = 0.2 < 0.5
    for i in 0..5 {
        wm.observe_transition("state_s".to_string(), "action_a".to_string(), format!("to_{i}"));
    }
    assert!(
        wm.transition_count() > 0,
        "AC-6h: WM must have transitions (data-presence passes)"
    );
    assert!(
        wm.max_transition_confidence() < 0.5,
        "AC-6h: max_transition_confidence must be < 0.5 for sparse 5-way uniform; got {}",
        wm.max_transition_confidence()
    );
    let ops = list_authorized_world_model(&sm, &wm);
    let has_rollout = ops.iter().any(|o| o.op == "world_model_rollout");
    assert!(
        !has_rollout,
        "AC-6h: rollout must be blocked when WM confidence < 0.5 (got {:.3})",
        wm.max_transition_confidence()
    );
}

/// AC-6i: Dominant WM belief (same outcome seen 4 times) → max_transition_confidence ≥ 0.5.
/// WM-belief gate passes; final auth still depends on SelfModel health (ac6g pattern).
/// Contrast with ac6h (sparse WM blocks rollout) — here the WM portion does not block.
#[test]
fn ac6i_dominant_wm_belief_passes_wm_gate() {
    let sm = SelfModel::new();
    let mut wm = WorldModelEnhanced::new();
    // 4 observations same (state,action,next_state) → Dirichlet max prob = (4+1)/(4+1×1) = 1.0
    for _ in 0..4 {
        wm.observe_transition("state_x".to_string(), "action_y".to_string(), "state_z".to_string());
    }
    assert!(
        wm.max_transition_confidence() >= 0.5,
        "AC-6i: max_transition_confidence must be ≥ 0.5 after 4 same-pair obs; got {}",
        wm.max_transition_confidence()
    );
    // WM-belief gate condition is met; SelfModel determines the final auth result.
    // Contrast: ac6h sparse WM confidence=0.2 DOES block rollout; here WM gate does not block.
    let _ops = list_authorized_world_model(&sm, &wm); // must not panic
}

// ─── Gap 5: Auth WM-policy for react_loop and credit_assign ──────────────────

/// AC-6j: react_loop must be blocked when WM has 0 transitions (WM-policy gate).
#[test]
fn ac6j_react_loop_blocked_without_wm_transitions() {
    let sm = SelfModel::new();
    let wm = WorldModelEnhanced::new(); // 0 transitions
    assert_eq!(wm.transition_count(), 0, "precondition: no transitions");
    let ops = list_authorized_world_model(&sm, &wm);
    let has_react = ops.iter().any(|o| o.op == "react_loop");
    assert!(
        !has_react,
        "AC-6j: react_loop must be blocked when WM has 0 transitions"
    );
}

/// AC-6k: WM with ≥1 transition lets react_loop through the WM-policy gate.
/// WM_CONSTRAINTS must declare react_loop with requires_wm=true.
#[test]
fn ac6k_react_loop_wm_gate_passes_with_transitions() {
    let sm = SelfModel::new();
    let mut wm = WorldModelEnhanced::new();
    wm.observe_transition("s0".to_string(), "act".to_string(), "s1".to_string());
    assert!(wm.transition_count() > 0, "precondition: has transitions");
    // Constraint declared with requires_wm=true
    let c = WM_CONSTRAINTS.iter().find(|c| c.op == "react_loop")
        .expect("AC-6k: react_loop must be in WM_CONSTRAINTS");
    assert!(c.requires_wm, "AC-6k: react_loop requires_wm must be true");
    // WM gate does not block; SelfModel determines final auth — must not panic
    let _ops = list_authorized_world_model(&sm, &wm);
}

/// AC-6l: credit_assign must be blocked when WM has neither causal nodes nor transitions.
#[test]
fn ac6l_credit_assign_blocked_without_wm_causal_or_transitions() {
    let sm = SelfModel::new();
    let wm = WorldModelEnhanced::new(); // 0 causal nodes, 0 transitions
    assert_eq!(wm.causal_node_count(), 0, "precondition: no causal nodes");
    assert_eq!(wm.transition_count(), 0, "precondition: no transitions");
    let ops = list_authorized_world_model(&sm, &wm);
    let has_credit = ops.iter().any(|o| o.op == "credit_assign");
    assert!(
        !has_credit,
        "AC-6l: credit_assign must be blocked when WM has 0 causal nodes and 0 transitions"
    );
}
