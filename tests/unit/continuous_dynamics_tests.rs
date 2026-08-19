use hipcortex::continuous_dynamics::{
    ContinuousDynamics, DynamicsContext, KalmanVectorField, VectorField,
};
use uuid::Uuid;

#[test]
fn kalman_vector_field_dim_matches() {
    let vf = KalmanVectorField::new(3);
    assert_eq!(vf.dim(), 3);
}

#[test]
fn rk4_step_zero_field_leaves_state_unchanged() {
    // Zero transition matrix → dμ/dt = 0 → state unchanged
    let vf = KalmanVectorField::with_diag(vec![0.0, 0.0]);
    let mut dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 1.0);
    let ctx = DynamicsContext {
        entity_states: &[],
        resource_vec: &[],
        tx_cursor: 0,
    };
    let initial = vec![1.0, 2.0];
    let result = dyn_.step(0.0, &initial, &ctx).unwrap();
    assert!((result[0] - 1.0).abs() < 1e-9);
    assert!((result[1] - 2.0).abs() < 1e-9);
}

#[test]
fn sigma_norm_grows_with_steps() {
    let vf = KalmanVectorField::new(2);
    let mut dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let ctx = DynamicsContext {
        entity_states: &[],
        resource_vec: &[],
        tx_cursor: 0,
    };
    let s0 = dyn_.sigma_norm();
    let state = vec![1.0, 1.0];
    dyn_.step(0.0, &state, &ctx).ok();
    let s1 = dyn_.sigma_norm();
    assert!(s1 >= s0, "covariance must grow or stay with each step");
}

#[test]
fn halts_when_sigma_exceeds_max() {
    // Large diagonal → fast sigma growth; tiny max_covariance → halts quickly
    let vf = KalmanVectorField::with_diag(vec![10.0]);
    let mut dyn_ = ContinuousDynamics::new(Box::new(vf), 0.5, 0.01);
    let ctx = DynamicsContext {
        entity_states: &[],
        resource_vec: &[],
        tx_cursor: 0,
    };
    let state = vec![1.0];
    // After first step, sigma should exceed 0.01
    dyn_.step(0.0, &state, &ctx).ok();
    assert!(dyn_.is_halted());
}
