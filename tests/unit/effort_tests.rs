use hipcortex::effort::{ConfidenceRegulator, EffortEvaluator};

#[test]
fn effort_and_confidence() {
    let mut eval = EffortEvaluator::new();
    let mut conf = ConfidenceRegulator::new();
    eval.record();
    conf.decay(0.1);
    assert_eq!(eval.effort(), 1);
    assert!(conf.confidence() < 1.0);
}
