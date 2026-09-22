//! MDL-aware GC for Laws (v3.16.0).
//! Laws carry an MDL (Minimum Description Length) sparsity score. High-MDL laws
//! (>= MDL_KEEP_THRESHOLD) are always kept regardless of references. Below the
//! threshold the standard reference-based policy applies: Archive if referenced,
//! Delete if orphaned.

use hipcortex::cognitive_gc::{CognitiveGC, GcAction, MDL_KEEP_THRESHOLD};
use uuid::Uuid;

/// AC-S1: an orphaned law below the MDL threshold is deleted.
#[test]
fn low_mdl_orphaned_law_is_deleted() {
    let gc = CognitiveGC::new();
    let law_id = Uuid::new_v4();
    // no references registered → orphaned
    let action = gc.gc_action_for_law(law_id, 0.1);
    assert_eq!(
        action,
        GcAction::Delete,
        "orphaned law below MDL threshold must be deleted"
    );
}

/// AC-S1 (referenced branch): a referenced law below the MDL threshold is archived.
#[test]
fn low_mdl_referenced_law_is_archived() {
    let mut gc = CognitiveGC::new();
    let law_id = Uuid::new_v4();
    let goal_id = Uuid::new_v4();
    // register a reference so in-degree > 0
    gc.register_reference(law_id, goal_id);
    let action = gc.gc_action_for_law(law_id, 0.1);
    assert_eq!(
        action,
        GcAction::Archive,
        "referenced law below MDL threshold must be archived"
    );
}

/// AC-S2: a high-MDL law is kept regardless of references (none here).
#[test]
fn high_mdl_law_kept_regardless_of_references() {
    let gc = CognitiveGC::new();
    let law_id = Uuid::new_v4();
    // no references — but high mdl_score overrides
    let action = gc.gc_action_for_law(law_id, 0.9);
    assert_eq!(
        action,
        GcAction::Keep,
        "law above MDL threshold must be kept"
    );
}

/// AC-S2 (edge, referenced): high-MDL law kept even when referenced.
#[test]
fn high_mdl_referenced_law_still_kept() {
    let mut gc = CognitiveGC::new();
    let law_id = Uuid::new_v4();
    let goal_id = Uuid::new_v4();
    gc.register_reference(law_id, goal_id);
    let action = gc.gc_action_for_law(law_id, 0.9);
    assert_eq!(
        action,
        GcAction::Keep,
        "high-MDL law must be kept even when referenced"
    );
}

/// Edge: exactly at the threshold is Kept (>= semantics).
#[test]
fn borderline_mdl_at_threshold_is_kept() {
    let gc = CognitiveGC::new();
    let law_id = Uuid::new_v4();
    let action = gc.gc_action_for_law(law_id, MDL_KEEP_THRESHOLD);
    assert_eq!(
        action,
        GcAction::Keep,
        "law exactly at threshold must be kept (>=)"
    );
}
