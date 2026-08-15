use hipcortex::payloads::{BeliefPayload, EpistemicStatus};

#[test]
fn belief_backward_compat_missing_confidence_defaults_to_0_5() {
    let json = r#"{"proposition":"sky is blue","justification":"observed"}"#;
    let p: BeliefPayload = serde_json::from_str(json).unwrap();
    assert!(
        (p.confidence - 0.5).abs() < 1e-6,
        "expected confidence=0.5, got {}",
        p.confidence
    );
}

#[test]
fn belief_backward_compat_missing_fields_deserialize_cleanly() {
    let json = r#"{"proposition":"test"}"#;
    let p: BeliefPayload = serde_json::from_str(json).unwrap();
    assert_eq!(p.epistemic_status, EpistemicStatus::Hypothetical);
    assert!(p.causal_source_ids.is_empty());
    assert_eq!(p.half_life_ms, 0);
    assert_eq!(p.tx_origin, None);
}

#[test]
fn belief_full_roundtrip_preserves_all_fields() {
    use uuid::Uuid;
    let payload = BeliefPayload {
        proposition: "earth is round".to_string(),
        justification: "satellite images".to_string(),
        contradicts: vec![],
        confidence: 0.99,
        epistemic_status: EpistemicStatus::Observed,
        causal_source_ids: vec![Uuid::new_v4()],
        half_life_ms: 3_600_000,
        tx_origin: Some(42),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let back: BeliefPayload = serde_json::from_str(&json).unwrap();
    assert!((back.confidence - 0.99).abs() < 1e-6);
    assert_eq!(back.epistemic_status, EpistemicStatus::Observed);
    assert_eq!(back.half_life_ms, 3_600_000);
    assert_eq!(back.tx_origin, Some(42));
    assert_eq!(back.causal_source_ids.len(), 1);
}

#[test]
fn epistemic_status_default_is_hypothetical() {
    let s = EpistemicStatus::default();
    assert_eq!(s, EpistemicStatus::Hypothetical);
}
