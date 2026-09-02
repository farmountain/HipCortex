// Phase-3c: WM authorization (AC-6a through AC-6d)
// AC-6a: WM_CONSTRAINTS is non-empty
// AC-6b: list_authorized_world_model returns a vec without panicking
// AC-6c: All constraint caps satisfy REST endpoint invariants (depth≤10, iter≤200)
// AC-6d: list_authorized_world_model result confidence values are in [0.0, 1.0]

use hipcortex::action_registry::{list_authorized_world_model, WM_CONSTRAINTS};
use hipcortex::self_model::SelfModel;

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
    let ops = list_authorized_world_model(&sm);
    // Just confirming it returns without panic; ops may be empty on a fresh SelfModel
    let _ = ops; // no assertion on count — fresh SelfModel may have no approved ops
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
    let ops = list_authorized_world_model(&sm);
    for op in &ops {
        assert!(
            op.confidence >= 0.0 && op.confidence <= 1.0,
            "AC-6d: op '{}' confidence {} outside [0, 1]",
            op.op, op.confidence
        );
    }
}
