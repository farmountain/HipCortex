use hipcortex::belief_executive::BeliefExecutive;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, JtmsLabel};

fn belief(store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>, conf: f32) -> uuid::Uuid {
    let bp = BeliefPayload { proposition: "test".into(), confidence: conf, ..Default::default() };
    let mut rec = MemoryRecord::new(
        MemoryType::Belief, "a".into(), "assert".into(), "test".into(),
        serde_json::to_value(&bp).unwrap(),
    );
    rec.confidence = conf;
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

#[test]
fn decay_above_threshold_no_jtms_cascade() {
    let mut store = MemoryStore::new_in_memory();
    let id = belief(&mut store, 0.8);
    let retracted = BeliefExecutive::decay(&mut store, id, 0.3);
    assert!(retracted.is_empty(), "above threshold must not cascade JTMS");
    let rec = store.find_by_id(id).unwrap();
    let p: BeliefPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    // JtmsLabel stays Unknown (not retracted)
    assert_ne!(p.jtms_label, JtmsLabel::Out, "must stay In/Unknown above threshold");
    assert!((rec.confidence - 0.3).abs() < 0.01, "confidence must be updated");
}

#[test]
fn decay_below_threshold_cascades_jtms_to_out() {
    let mut store = MemoryStore::new_in_memory();
    let id = belief(&mut store, 0.8);
    let retracted = BeliefExecutive::decay(&mut store, id, 0.05);
    assert!(!retracted.is_empty(), "below threshold must cascade JTMS");
    assert!(retracted.contains(&id));
    let rec = store.find_by_id(id).unwrap();
    let p: BeliefPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    assert_eq!(p.jtms_label, JtmsLabel::Out, "must be Out after cascade");
}

#[test]
fn retract_clamps_confidence_to_zero() {
    let mut store = MemoryStore::new_in_memory();
    let id = belief(&mut store, 0.9);
    let retracted = BeliefExecutive::retract(&mut store, id, None, "test");
    assert!(retracted.contains(&id));
    let rec = store.find_by_id(id).unwrap();
    assert!(rec.confidence < 0.01, "retract must clamp confidence to 0");
    let p: BeliefPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    assert_eq!(p.jtms_label, JtmsLabel::Out);
}

#[test]
fn decay_and_jtms_agree_no_split_state() {
    // Verify that after decay below threshold, confidence and JtmsLabel are both
    // in the "invalid" state — no split where one says valid and other says not.
    let mut store = MemoryStore::new_in_memory();
    let id = belief(&mut store, 0.9);
    BeliefExecutive::decay(&mut store, id, 0.1);
    let rec = store.find_by_id(id).unwrap();
    let p: BeliefPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    let conf_says_invalid = rec.confidence < 0.2;
    let label_says_invalid = p.jtms_label == JtmsLabel::Out;
    assert_eq!(
        conf_says_invalid, label_says_invalid,
        "confidence and JtmsLabel must agree: conf={} label={:?}",
        rec.confidence, p.jtms_label
    );
}
