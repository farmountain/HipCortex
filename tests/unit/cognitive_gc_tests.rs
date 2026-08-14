use hipcortex::cognitive_gc::{CognitiveGC, GcAction};
use uuid::Uuid;

#[test]
fn test_gc_preserves_referenced_record() {
    let mut gc = CognitiveGC::new();
    let record_id = Uuid::new_v4();
    let goal_id = Uuid::new_v4();
    gc.register_reference(record_id, goal_id);
    assert_eq!(gc.gc_action(record_id), GcAction::Archive);
}

#[test]
fn test_gc_deletes_unreferenced_record() {
    let mut gc = CognitiveGC::new();
    let record_id = Uuid::new_v4();
    assert_eq!(gc.gc_action(record_id), GcAction::Delete);
}

#[test]
fn test_gc_deregister_makes_orphan() {
    let mut gc = CognitiveGC::new();
    let obs_id = Uuid::new_v4();
    let goal_id = Uuid::new_v4();
    gc.register_reference(obs_id, goal_id);
    gc.deregister_referencing(goal_id);
    assert_eq!(gc.gc_action(obs_id), GcAction::Delete);
}
