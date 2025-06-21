use hipcortex::effort::{ConfidenceRegulator, EffortEvaluator};

#[test]
fn user_tracks_effort_and_confidence() {
    let mut eval = EffortEvaluator::new();
    let mut reg = ConfidenceRegulator::new();

    // Simulate a user reasoning session with incremental decay.
    for cost in [1, 2, 3] {
        eval.record(cost);
        reg.decay(0.15);
    }

    let score = eval.collapse_score(reg.confidence());
    assert!(score >= 0.0 && score <= 1.0);
    assert!(eval.is_collapse_imminent(reg.confidence(), 0.3));
}
