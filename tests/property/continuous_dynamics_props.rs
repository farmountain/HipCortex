// Property tests for ContinuousDynamics mathematical invariants:
// 1. sigma_norm always >= 0
// 2. sigma_norm monotonically non-decreasing across steps
// 3. halted flag never resets to false after being set

use hipcortex::continuous_dynamics::{ContinuousDynamics, DynamicsContext, KalmanVectorField};
use proptest::prelude::*;

proptest! {
    #[test]
    fn sigma_norm_always_nonnegative(
        diag in prop::collection::vec(0.0f64..5.0, 1..=4),
        dt in 0.01f64..0.5,
        steps in 1usize..10,
    ) {
        let vf = KalmanVectorField::with_diag(diag.clone());
        let mut dyn_ = ContinuousDynamics::new(Box::new(vf), dt, 1e9);
        let ctx = DynamicsContext { entity_states: &[], resource_vec: &[], tx_cursor: 0 };
        let mut state = vec![1.0; diag.len()];
        for s in 0..steps {
            if let Ok(ns) = dyn_.step(s as f64 * dt, &state, &ctx) {
                state = ns;
            }
            prop_assert!(dyn_.sigma_norm() >= 0.0, "sigma_norm negative at step {s}");
        }
    }
}

proptest! {
    #[test]
    fn sigma_norm_monotonically_nondecreasing(
        diag in prop::collection::vec(0.0f64..2.0, 1..=3),
        dt in 0.01f64..0.3,
        steps in 2usize..8,
    ) {
        let vf = KalmanVectorField::with_diag(diag.clone());
        let mut dyn_ = ContinuousDynamics::new(Box::new(vf), dt, 1e9);
        let ctx = DynamicsContext { entity_states: &[], resource_vec: &[], tx_cursor: 0 };
        let mut state = vec![1.0; diag.len()];
        let mut prev = dyn_.sigma_norm();
        for s in 0..steps {
            if let Ok(ns) = dyn_.step(s as f64 * dt, &state, &ctx) {
                state = ns;
            }
            let curr = dyn_.sigma_norm();
            prop_assert!(
                curr >= prev - 1e-12,
                "sigma_norm decreased: {prev:.6} -> {curr:.6} at step {s}"
            );
            prev = curr;
        }
    }
}

proptest! {
    #[test]
    fn halted_flag_never_clears_spontaneously(
        diag in prop::collection::vec(5.0f64..10.0, 1..=2),
        dt in 0.1f64..0.5,
    ) {
        let vf = KalmanVectorField::with_diag(diag.clone());
        let mut dyn_ = ContinuousDynamics::new(Box::new(vf), dt, 0.001);
        let ctx = DynamicsContext { entity_states: &[], resource_vec: &[], tx_cursor: 0 };
        let state = vec![1.0; diag.len()];
        for s in 0..20 {
            dyn_.step(s as f64 * dt, &state, &ctx).ok();
            if dyn_.is_halted() { break; }
        }
        if dyn_.is_halted() {
            for s in 0..5 {
                dyn_.step(s as f64 * dt, &state, &ctx).ok();
                prop_assert!(dyn_.is_halted(), "halted flag cleared spontaneously");
            }
        }
    }
}
