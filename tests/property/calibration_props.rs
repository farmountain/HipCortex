// Property and unit tests for UncertaintyEstimator calibration invariants.
//
// ECE is a quality target (< 0.1), not a universal invariant for random inputs.
// Tests focus on structural invariants (ECE in [0,1]) and constructive
// properties (perfect calibration -> ECE = 0).

use hipcortex::world_model_enhanced::UncertaintyEstimator;
use proptest::prelude::*;

// ============================================================================
// Unit test 1: Empty estimator -> ECE = 0.0 (not NaN, not panic)
// ============================================================================

#[test]
fn empty_estimator_ece_is_zero() {
    let estimator = UncertaintyEstimator::new();
    let metrics = estimator.get_metrics();
    assert_eq!(metrics.ece, 0.0, "empty estimator must return ECE = 0.0");
    assert!(!metrics.ece.is_nan(), "ECE must not be NaN on empty input");
}

// ============================================================================
// Unit test 2: Perfect calibration -> ECE ~= 0.0
//
// Build 10 buckets x 100 samples each.
// Bucket i midpoint confidence = (2i + 1) / 20 -> 0.05, 0.15, ..., 0.95
// Set exactly round(100 x confidence) samples correct per bucket.
// -> fraction_correct == confidence per bucket -> ECE = 0.
// ============================================================================

#[test]
fn perfect_calibration_achieves_zero_ece() {
    let mut estimator = UncertaintyEstimator::with_bins(10);
    for bucket_idx in 0..10usize {
        let confidence = (2 * bucket_idx + 1) as f64 / 20.0;
        let n_samples = 100usize;
        let n_correct = (n_samples as f64 * confidence).round() as usize;
        for i in 0..n_samples {
            estimator.record_outcome(confidence, i < n_correct);
        }
    }
    let metrics = estimator.get_metrics();
    assert!(
        metrics.ece < 1e-9,
        "perfect calibration must yield ECE ~= 0, got {:.6}",
        metrics.ece
    );
}

// ============================================================================
// Proptest 1: ECE always in [0, 1] for any input
// ============================================================================

proptest! {
    #[test]
    fn ece_always_in_unit_interval(
        samples in prop::collection::vec(
            (0.0f64..=1.0, prop::bool::ANY),
            1..200
        )
    ) {
        let mut estimator = UncertaintyEstimator::new();
        for (prob, correct) in &samples {
            estimator.record_outcome(*prob, *correct);
        }
        let metrics = estimator.get_metrics();
        prop_assert!(metrics.ece >= 0.0, "ECE < 0: {:.6}", metrics.ece);
        prop_assert!(metrics.ece <= 1.0, "ECE > 1: {:.6}", metrics.ece);
        prop_assert!(!metrics.ece.is_nan(), "ECE is NaN");
    }
}

// ============================================================================
// Proptest 2: ECE <= MCE for any input
// ECE is a weighted average of per-bucket errors; MCE is the max.
// Weighted average <= max is always true.
// ============================================================================

proptest! {
    #[test]
    fn ece_bounded_by_mce(
        samples in prop::collection::vec(
            (0.0f64..=1.0, prop::bool::ANY),
            10..200
        )
    ) {
        let mut estimator = UncertaintyEstimator::new();
        for (prob, correct) in &samples {
            estimator.record_outcome(*prob, *correct);
        }
        let metrics = estimator.get_metrics();
        prop_assert!(
            metrics.ece <= metrics.mce + f64::EPSILON,
            "ECE {:.6} > MCE {:.6} -- violated weighted-average <= max invariant",
            metrics.ece,
            metrics.mce
        );
    }
}

// ============================================================================
// Proptest 3: is_well_calibrated() consistent with get_metrics().ece < 0.1
// The gate method must agree with the raw metric on every input.
// ============================================================================

proptest! {
    #[test]
    fn is_well_calibrated_iff_ece_below_threshold(
        samples in prop::collection::vec(
            (0.0f64..=1.0, prop::bool::ANY),
            1..200
        )
    ) {
        let mut estimator = UncertaintyEstimator::new();
        for (prob, correct) in &samples {
            estimator.record_outcome(*prob, *correct);
        }
        let metrics = estimator.get_metrics();
        let gate = estimator.is_well_calibrated();
        prop_assert_eq!(
            gate,
            metrics.ece < 0.1,
            "is_well_calibrated() ({}) inconsistent with ECE {:.6} < 0.1",
            gate,
            metrics.ece
        );
    }
}
