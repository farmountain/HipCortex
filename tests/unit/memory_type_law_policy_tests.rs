use hipcortex::memory_record::MemoryType;

#[test]
fn law_memory_type_serde_round_trip() {
    let t = MemoryType::Law;
    let s = serde_json::to_string(&t).unwrap();
    let back: MemoryType = serde_json::from_str(&s).unwrap();
    assert_eq!(back, MemoryType::Law);
}

#[test]
fn policy_memory_type_serde_round_trip() {
    let t = MemoryType::Policy;
    let s = serde_json::to_string(&t).unwrap();
    let back: MemoryType = serde_json::from_str(&s).unwrap();
    assert_eq!(back, MemoryType::Policy);
}

#[test]
fn law_and_policy_are_distinct_types() {
    assert_ne!(MemoryType::Law, MemoryType::Policy);
    assert_ne!(MemoryType::Law, MemoryType::Belief);
    assert_ne!(MemoryType::Policy, MemoryType::Goal);
}
