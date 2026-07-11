use hipcortex::world_model_enhanced::constraint::{Constraint, ConstraintEngine, ConstraintSeverity};

#[test]
fn test_constraint_boundary_evaluation() {
    let mut engine = ConstraintEngine::new();
    engine.add_constraint(Constraint {
        constraint_id: "OOM_HARD".to_string(),
        target_metric: "memory_mb".to_string(),
        operator: ">=".to_string(),
        threshold: 4096.0,
        severity: ConstraintSeverity::HardTermination,
    });
    engine.add_constraint(Constraint {
        constraint_id: "LATENCY_SOFT".to_string(),
        target_metric: "latency_ms".to_string(),
        operator: ">".to_string(),
        threshold: 50.0,
        severity: ConstraintSeverity::SoftPenalty(15.0),
    });

    assert_eq!(engine.evaluate("memory_mb", 5000.0), Some(ConstraintSeverity::HardTermination));
    assert_eq!(engine.evaluate("latency_ms", 120.0), Some(ConstraintSeverity::SoftPenalty(15.0)));
    assert_eq!(engine.evaluate("memory_mb", 1024.0), None);
}
