use hipcortex::world_model_enhanced::causal::{LinearSE, StructuralEquation};

#[test]
fn test_linear_se_evaluate() {
    let se = LinearSE { weights: vec![2.0, 3.0] };
    let result = se.evaluate(&[1.0, 2.0], 0.5);
    assert!((result - 8.5).abs() < 1e-9);
}

#[test]
fn test_linear_se_invert_for_u() {
    let se = LinearSE { weights: vec![2.0, 3.0] };
    let u = se.invert_for_u(&[1.0, 2.0], 8.5);
    assert!((u - 0.5).abs() < 1e-9);
}
