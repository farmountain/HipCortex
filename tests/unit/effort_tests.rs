use hipcortex::effort::{ConfidenceRegulator, EffortEvaluator};

#[test]
fn cost_weighted_effort() {
    let mut eval = EffortEvaluator::new();
    eval.record(2);
    eval.record(3);
    assert_eq!(eval.effort(), 5);
}

#[test]
fn confidence_decay_models() {
    let mut reg = ConfidenceRegulator::new();
    reg.decay(0.2);
    assert!((reg.confidence() - 0.8).abs() < 1e-6);
    reg.decay_exponential(0.5); // 0.8 * 0.5
    assert!((reg.confidence() - 0.4).abs() < 1e-6);
    reg.decay_log(1.0); // 0.4 * 1/(1+1) = 0.2
    assert!((reg.confidence() - 0.2).abs() < 1e-6);
}

#[test]
fn confidence_never_negative() {
    let mut reg = ConfidenceRegulator::new();
    reg.decay(2.0);
    assert_eq!(reg.confidence(), 0.0);
    reg.decay(1.0);
    reg.decay_exponential(0.5);
    reg.decay_log(10.0);
    assert_eq!(reg.confidence(), 0.0);
}

#[test]
fn collapse_score_bounds_and_threshold() {
    let mut eval = EffortEvaluator::new();
    let conf = 1.0;
    let score = eval.collapse_score(conf);
    assert!((score - 0.0).abs() < 1e-6);
    assert!(!eval.is_collapse_imminent(conf, 0.5));

    eval.record(10);
    let conf = 0.2;
    let score = eval.collapse_score(conf);
    assert!(score >= 0.0 && score <= 1.0);
    assert!(eval.is_collapse_imminent(conf, 0.1));
}
