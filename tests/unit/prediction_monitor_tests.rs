// Unit tests for PredictionMonitor (Phase-E, G-SHIFT).
// Covers the rolling-error window and SelfModel::record_prediction_error wiring.

use hipcortex::self_model::{PredictionMonitor, SelfModel};

#[test]
fn prediction_monitor_no_trigger_below_window() {
    let mut pm = PredictionMonitor::new("eq-1", 5, 0.3);
    for _ in 0..4 {
        assert!(pm.feed(0.9).is_none(), "must not trigger before window is full");
    }
}

#[test]
fn prediction_monitor_triggers_on_full_window_all_above_threshold() {
    let mut pm = PredictionMonitor::new("eq-target", 5, 0.3);
    for _ in 0..4 {
        pm.feed(0.9);
    }
    let result = pm.feed(0.9);
    assert!(result.is_some(), "must trigger when window full and all errors > threshold");
    let (node_id, weights) = result.unwrap();
    assert_eq!(node_id, "eq-target");
    assert!(!weights.is_empty());
}

#[test]
fn prediction_monitor_no_trigger_when_one_slot_low() {
    let mut pm = PredictionMonitor::new("eq-2", 5, 0.3);
    pm.feed(0.9);
    pm.feed(0.9);
    pm.feed(0.05); // below threshold
    pm.feed(0.9);
    let result = pm.feed(0.9);
    assert!(result.is_none(), "must not trigger when one slot is below threshold");
}

#[test]
fn prediction_monitor_resets_after_trigger() {
    let mut pm = PredictionMonitor::new("eq-3", 2, 0.3);
    pm.feed(0.9); // window: [0.9] — not full yet
    assert!(pm.feed(0.9).is_some(), "window full => trigger");
    // after reset, only 1 error in window — should not trigger again
    assert!(pm.feed(0.9).is_none(), "must not re-trigger immediately after reset");
}

#[test]
fn self_model_record_prediction_error_returns_none_before_window() {
    let sm = SelfModel::new(); // default: window=5, threshold=0.3
    for _ in 0..4 {
        assert!(
            sm.record_prediction_error(0.9).is_none(),
            "must return None before window fills"
        );
    }
}

#[test]
fn self_model_record_prediction_error_returns_some_on_persistent_drift() {
    let sm = SelfModel::new();
    for _ in 0..4 {
        sm.record_prediction_error(0.9);
    }
    let result = sm.record_prediction_error(0.9);
    assert!(result.is_some(), "must signal drift after full window of high errors");
    let (node_id, weights) = result.unwrap();
    assert_eq!(node_id, "world-model-default");
    assert!(!weights.is_empty());
}

// ── Phase-3b OLS drift isolation (AC-5a through AC-5d) ────────────────────────

#[test]
fn ac5a_feed_with_obs_stores_pairs_for_ols() {
    // AC-5a: feed_with_obs accumulates (x, y) pairs
    let mut pm = PredictionMonitor::new("eq-ols", 5, 0.3);
    pm.feed_with_obs(0.1, vec![1.0, 0.0], vec![0.5]);
    pm.feed_with_obs(0.1, vec![2.0, 0.0], vec![1.0]);
    let weights = pm.fit_ols();
    assert!(weights.is_some(), "AC-5a: fit_ols must return Some after >= 2 obs pairs");
}

#[test]
fn ac5b_fit_ols_returns_none_before_two_pairs() {
    // AC-5b: fit_ols returns None when fewer than 2 pairs collected
    let pm = PredictionMonitor::new("eq-ols", 5, 0.3);
    assert!(pm.fit_ols().is_none(), "AC-5b: fit_ols must return None before 2 pairs");
}

#[test]
fn ac5c_ols_weights_approximate_linear_relationship() {
    // AC-5c: OLS weights should match w ≈ y/x for a simple y = 2x relationship
    let mut pm = PredictionMonitor::new("eq-ols", 10, 0.3);
    for i in 1..=5 {
        let x = i as f64;
        pm.feed_with_obs(0.1, vec![x], vec![2.0 * x]);
    }
    let weights = pm.fit_ols().expect("AC-5c: fit_ols must return Some");
    assert!(!weights.is_empty(), "AC-5c: weights must not be empty");
    let w0 = weights[0];
    assert!(
        (w0 - 2.0).abs() < 0.01,
        "AC-5c: OLS weight should ≈ 2.0 for y=2x; got {w0}"
    );
}

#[test]
fn ac5d_self_model_record_with_obs_and_drift_weights() {
    // AC-5d: SelfModel wiring — record_prediction_error_with_obs + prediction_drift_weights
    let sm = SelfModel::new();
    sm.record_prediction_error_with_obs(0.1, vec![1.0], vec![1.0]);
    sm.record_prediction_error_with_obs(0.1, vec![2.0], vec![2.0]);
    let weights = sm.prediction_drift_weights();
    assert!(weights.is_some(), "AC-5d: prediction_drift_weights must return Some after 2 obs");
}
