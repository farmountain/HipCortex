use hipcortex::effort::{ConfidenceRegulator, EffortEvaluator};

#[test]
fn collapse_score_integration_flow() {
    let mut eval = EffortEvaluator::new();
    let mut reg = ConfidenceRegulator::new();

    // Record multiple reasoning steps with varied cost.
    eval.record(2);
    reg.decay_exponential(0.1);
    eval.record(3);
    reg.decay_log(0.5);

    let score = eval.collapse_score(reg.confidence());
    assert!(score > 0.0 && score <= 1.0);
    assert!(eval.is_collapse_imminent(reg.confidence(), 0.1));
}
