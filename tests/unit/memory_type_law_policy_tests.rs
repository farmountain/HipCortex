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
