use hipcortex::memory_record::MemoryType;

#[test]
fn law_memory_type_serde_round_trip() {
    let t = MemoryType::Law;
    let s = serde_json::to_string(&t).unwrap();
    assert_eq!(s, "\"Law\"", "wire format must be 'Law' — grpc/web string-matching depends on it");
    let back: MemoryType = serde_json::from_str(&s).unwrap();
    assert_eq!(back, MemoryType::Law);
}

#[test]
fn policy_memory_type_serde_round_trip() {
    let t = MemoryType::Policy;
    let s = serde_json::to_string(&t).unwrap();
    assert_eq!(s, "\"Policy\"", "wire format must be 'Policy' — grpc/web string-matching depends on it");
    let back: MemoryType = serde_json::from_str(&s).unwrap();
    assert_eq!(back, MemoryType::Policy);
}

#[test]
fn law_is_not_belief_or_goal() {
    // Structural: Laws are distinct from semantically adjacent types.
    // Guards against accidental enum reordering that could cause serde
    // to deserialize the wrong discriminant in bincode/postcard contexts.
    let law_json = serde_json::to_string(&MemoryType::Law).unwrap();
    let belief_json = serde_json::to_string(&MemoryType::Belief).unwrap();
    let goal_json = serde_json::to_string(&MemoryType::Goal).unwrap();
    assert_ne!(law_json, belief_json);
    assert_ne!(law_json, goal_json);
    let policy_json = serde_json::to_string(&MemoryType::Policy).unwrap();
    assert_ne!(policy_json, belief_json);
    assert_ne!(policy_json, goal_json);
    assert_ne!(policy_json, law_json);
}

use hipcortex::payloads::{LawPayload, PolicyPayload};

#[test]
fn law_payload_serde_round_trip() {
    let p = LawPayload {
        equation: "velocity ~ apply_force(mass)".into(),
        causal_variables: vec!["mass".into(), "friction".into()],
        effect_variable: "velocity".into(),
        evidence_ids: vec![uuid::Uuid::new_v4()],
        mdl_score: 1.5,
        domain: Some("physics".into()),
        holds_under_intervention: true,
    };
    let v = serde_json::to_value(&p).unwrap();
    let back: LawPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back.equation, p.equation);
    assert!((back.mdl_score - 1.5).abs() < 1e-9);
    assert!(back.holds_under_intervention);
}

#[test]
fn law_payload_defaults_work() {
    // Minimal JSON — only required field is equation
    let json = r#"{"equation":"x ~ f(y)"}"#;
    let p: LawPayload = serde_json::from_str(json).unwrap();
    assert!(p.causal_variables.is_empty());
    assert_eq!(p.effect_variable, "unknown");
    assert!(p.holds_under_intervention); // default true
    assert!((p.mdl_score).abs() < 1e-9); // default 0.0
}

#[test]
fn policy_payload_serde_round_trip() {
    let entity = uuid::Uuid::new_v4();
    let p = PolicyPayload {
        entity_id: entity,
        trigger_condition: "state.velocity > 2.0".into(),
        action_fn: "apply_brake(0.5)".into(),
        priority: 10,
        active: true,
        law_id: None,
    };
    let v = serde_json::to_value(&p).unwrap();
    let back: PolicyPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back.entity_id, entity);
    assert_eq!(back.priority, 10);
    assert!(back.active);
}

#[test]
fn policy_payload_defaults_active_true() {
    // active defaults to true; law_id defaults to None
    let entity = uuid::Uuid::new_v4();
    let json = format!(
        r#"{{"entity_id":"{}","trigger_condition":"x","action_fn":"y","priority":1}}"#,
        entity
    );
    let p: PolicyPayload = serde_json::from_str(&json).unwrap();
    assert!(p.active);
    assert!(p.law_id.is_none());
}
