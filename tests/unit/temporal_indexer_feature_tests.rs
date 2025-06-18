use hipcortex::decay::{decay_exponential, decay_linear, DecayType};
use hipcortex::markov::MarkovChain;
use hipcortex::poisson::PoissonBurst;
use std::time::{Duration, SystemTime};

#[test]
fn decay_functions_work() {
    let half = decay_exponential(1.0, Duration::from_secs(10), Duration::from_secs(10));
    assert!((half - 0.5).abs() < 0.01);
    let lin = decay_linear(1.0, Duration::from_secs(5), Duration::from_secs(10));
    assert!((lin - 0.5).abs() < 0.01);
}

#[test]
fn markov_predicts_next() {
    let mut m = MarkovChain::new(4);
    m.record_transition(&"a", &"b");
    m.record_transition(&"a", &"b");
    m.record_transition(&"a", &"c");
    assert_eq!(m.predict_next(&"a"), Some("b"));
}

#[test]
fn poisson_detects_burst() {
    let mut p = PoissonBurst::new(0.5);
    let now = SystemTime::now();
    p.record_event_at(now - Duration::from_secs(2));
    p.record_event_at(now - Duration::from_millis(100));
    p.record_event_at(now);
    assert!(p.is_bursty());
}
