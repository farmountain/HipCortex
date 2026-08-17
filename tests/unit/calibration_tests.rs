use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, EpistemicStatus};
use hipcortex::persistence::InMemoryBackend;
use hipcortex::self_model::calibration::CalibrationTracker;

fn fresh_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn belief_record(confidence: f32) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Belief,
        "agent".to_string(),
        "assert".to_string(),
        "B1".to_string(),
        serde_json::to_value(BeliefPayload {
            proposition: "test belief".to_string(),
            justification: String::new(),
            contradicts: vec![],
            confidence,
            epistemic_status: EpistemicStatus::Observed,
            causal_source_ids: vec![],
            half_life_ms: 0,
            tx_origin: None,
            ..Default::default()
        })
        .unwrap(),
    )
}

#[test]
fn ewma_formula_alpha_0_1_single_error() {
    let tracker = CalibrationTracker::new();
    // ewma_new = 0.1 * 1.0 + 0.9 * 0.0 = 0.1
    tracker.record_prediction_error(1.0);
    let s = tracker.snapshot();
    assert!(
        (s.prediction_error_ewma - 0.1).abs() < 1e-5,
        "expected 0.1, got {}",
        s.prediction_error_ewma
    );
    // calibration_score = 1.0 - 0.1 = 0.9
    assert!(
        (s.calibration_score - 0.9).abs() < 1e-5,
        "expected 0.9, got {}",
        s.calibration_score
    );
}

#[test]
fn ewma_converges_to_one_after_many_errors() {
    let tracker = CalibrationTracker::new();
    for _ in 0..100 {
        tracker.record_prediction_error(1.0);
    }
    let s = tracker.snapshot();
    assert!(s.prediction_error_ewma > 0.99, "ewma should approach 1.0");
    assert!(
        s.calibration_score < 0.01,
        "calibration_score should approach 0.0"
    );
}

#[test]
fn entropy_zero_on_empty_store() {
    let store = fresh_store();
    let tracker = CalibrationTracker::new();
    tracker.update_from_store(&store, 0.0, 0);
    assert_eq!(tracker.snapshot().epistemic_entropy, 0.0);
}

#[test]
fn entropy_positive_with_belief_records() {
    let mut store = fresh_store();
    store.add(belief_record(0.8)).unwrap();
    let tracker = CalibrationTracker::new();
    tracker.update_from_store(&store, 0.0, 0);
    let s = tracker.snapshot();
    assert!(
        s.epistemic_entropy > 0.0,
        "entropy should be positive with one 0.8-confidence belief, got {}",
        s.epistemic_entropy
    );
}

#[test]
fn healthy_true_by_default() {
    let tracker = CalibrationTracker::new();
    assert!(tracker.snapshot().healthy);
}

#[test]
fn healthy_false_when_calibration_score_below_0_70() {
    let tracker = CalibrationTracker::new();
    for _ in 0..100 {
        tracker.record_prediction_error(1.0);
    }
    assert!(
        !tracker.snapshot().healthy,
        "unhealthy when calibration_score < 0.70"
    );
}

#[test]
fn healthy_false_when_pressure_above_0_90() {
    let store = fresh_store();
    let tracker = CalibrationTracker::new();
    tracker.update_from_store(&store, 0.95, 0);
    assert!(
        !tracker.snapshot().healthy,
        "unhealthy when consolidation_pressure = 0.95 > 0.90"
    );
}

#[test]
fn update_sets_current_tx() {
    let store = fresh_store();
    let tracker = CalibrationTracker::new();
    tracker.update_from_store(&store, 0.0, 77);
    assert_eq!(tracker.snapshot().current_tx, 77);
}
