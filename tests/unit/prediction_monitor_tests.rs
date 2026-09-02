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
