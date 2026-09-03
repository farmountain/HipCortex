use hipcortex::epistemic_authority::EpistemicAuthority;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, JtmsLabel};

fn in_memory() -> MemoryStore<hipcortex::persistence::InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn belief(actor: &str, label: JtmsLabel) -> MemoryRecord {
    let p = BeliefPayload {
        proposition: format!("prop_{}", uuid::Uuid::new_v4()),
        confidence: 0.7,
        jtms_label: label,
        ..Default::default()
    };
    MemoryRecord::new(
        MemoryType::Belief,
        actor.into(),
        "assert".into(),
        "prop".into(),
        serde_json::to_value(&p).unwrap(),
    )
}

// ── gate_belief_write ─────────────────────────────────────────────────────────

#[test]
fn gate_clamps_zero_evidence() {
    let result = EpistemicAuthority::gate_belief_write(0.9, 0);
    assert!(result <= 0.5, "0 evidence must cap at 0.5, got {result}");
}

#[test]
fn gate_clamps_one_evidence() {
    let result = EpistemicAuthority::gate_belief_write(0.9, 1);
    assert!(result <= 0.65, "1 evidence must cap at 0.65, got {result}");
}

#[test]
fn gate_clamps_three_evidence() {
    let result = EpistemicAuthority::gate_belief_write(0.9, 3);
    assert!(result <= 0.8, "3 evidence must cap at 0.80, got {result}");
}

#[test]
fn gate_passes_authority_evidence() {
    let result = EpistemicAuthority::gate_belief_write(0.9, 7);
    assert!((result - 0.9).abs() < f32::EPSILON, "7 evidence: no clamp, got {result}");
}

#[test]
fn gate_low_confidence_unchanged() {
    let result = EpistemicAuthority::gate_belief_write(0.4, 0);
    assert!((result - 0.4).abs() < f32::EPSILON, "0.4 <= 0.5 cap: unchanged, got {result}");
}

// ── actor_track_record ────────────────────────────────────────────────────────

#[test]
fn track_record_no_history_returns_one() {
    let store = in_memory();
    let r = EpistemicAuthority::actor_track_record(&store, "unknown_actor");
    assert!((r - 1.0).abs() < f32::EPSILON);
}

#[test]
fn track_record_all_in_returns_one() {
    let mut store = in_memory();
    store.add(belief("alice", JtmsLabel::In)).unwrap();
    store.add(belief("alice", JtmsLabel::In)).unwrap();
    let r = EpistemicAuthority::actor_track_record(&store, "alice");
    assert!((r - 1.0).abs() < f32::EPSILON);
}

#[test]
fn track_record_mixed() {
    let mut store = in_memory();
    store.add(belief("bob", JtmsLabel::In)).unwrap();
    store.add(belief("bob", JtmsLabel::In)).unwrap();
    store.add(belief("bob", JtmsLabel::Out)).unwrap();
    // 2 In + 1 Out = 2/3
    let r = EpistemicAuthority::actor_track_record(&store, "bob");
    assert!((r - 2.0 / 3.0).abs() < 0.01, "got {r}");
}

#[test]
fn track_record_excludes_unknown_label() {
    let mut store = in_memory();
    // Unknown labels are excluded from In+Out total
    store.add(belief("carol", JtmsLabel::Unknown)).unwrap();
    let r = EpistemicAuthority::actor_track_record(&store, "carol");
    // No In+Out → 1.0 (no history)
    assert!((r - 1.0).abs() < f32::EPSILON, "got {r}");
}
