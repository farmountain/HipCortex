use hipcortex::world_model_enhanced::entity::EntityState;
use hipcortex::world_model_enhanced::metalaw::{InvariantType, MetaLaw, MetaLawEngine};

#[test]
fn test_meta_laws_invariant_verification() {
    let mut engine = MetaLawEngine::new();
    engine.add_law(MetaLaw {
        law_id: "ENERGY_CONSERVATION".to_string(),
        invariant_type: InvariantType::Conservation { metric_index: 0 },
    });
    engine.add_law(MetaLaw {
        law_id: "ENTROPY_MONOTONIC".to_string(),
        invariant_type: InvariantType::Monotonic {
            metric_index: 1,
            increasing: true,
        },
    });
    engine.add_law(MetaLaw {
        law_id: "TEMP_BOUNDS".to_string(),
        invariant_type: InvariantType::Bounded {
            metric_index: 2,
            min: 0.0,
            max: 100.0,
        },
    });

    let pre_state = EntityState {
        properties: vec![10.0, 5.0, 50.0],
        covariance: vec![vec![0.1]],
    };
    let post_valid = EntityState {
        properties: vec![10.0, 6.0, 75.0],
        covariance: vec![vec![0.1]],
    };
    let post_invalid = EntityState {
        properties: vec![11.0, 4.0, 150.0],
        covariance: vec![vec![0.1]],
    };

    let valid_violations = engine.verify_transition(&pre_state, &post_valid);
    assert!(valid_violations.is_empty());

    let invalid_violations = engine.verify_transition(&pre_state, &post_invalid);
    assert_eq!(invalid_violations.len(), 3);
    assert!(invalid_violations.contains(&"ENERGY_CONSERVATION".to_string()));
    assert!(invalid_violations.contains(&"ENTROPY_MONOTONIC".to_string()));
    assert!(invalid_violations.contains(&"TEMP_BOUNDS".to_string()));
}
