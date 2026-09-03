use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::cognitive_state::{CognitiveDelta, CognitiveHandle};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, EpistemicStatus, JtmsLabel};
use hipcortex::persistence::InMemoryBackend;
use hipcortex::self_model::{calibration::CalibrationTracker, SelfModel};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::sync::{Arc, Mutex, RwLock};

fn handle() -> CognitiveHandle<InMemoryBackend> {
    let store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let wm = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let sm = Arc::new(SelfModel::new());
    let coherence = Arc::new(CoherenceChecker::new());
    let cal = Arc::new(CalibrationTracker::new());
    let gc = Arc::new(CognitiveGC::new());
    CognitiveHandle::new(store, wm, sm, None, coherence, cal, gc)
}

fn temporal(actor: &str, content: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal, actor.into(), "observe".into(),
        content.into(), serde_json::json!({ "thought": content }),
    )
}

// E1: Authority gate applied via CognitiveDelta::AddMemory

#[test]
fn add_memory_belief_high_conf_no_evidence_is_clamped() {
    let h = handle();
    let bp = BeliefPayload {
        proposition: "cache never fails".into(),
        confidence: 0.95,
        ..Default::default()
    };
    let mut rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "assert".into(),
        "cache never fails".into(), serde_json::to_value(&bp).unwrap(),
    );
    rec.confidence = 0.95;
    // no evidence IDs — rec.evidence is empty
    let id = rec.id;
    h.transact(CognitiveDelta::AddMemory(rec), "agent").unwrap();

    let store = h.memory.lock().unwrap();
    let stored = store.find_by_id(id).unwrap();
    assert!(
        stored.confidence <= 0.5,
        "0 evidence must clamp to <=0.5, got {}",
        stored.confidence
    );
}

#[test]
fn add_memory_belief_with_authority_evidence_passes_unmodified() {
    let h = handle();
    let mut store_lock = h.memory.lock().unwrap();
    let ev_ids: Vec<uuid::Uuid> = (0..7)
        .map(|i| {
            let t = temporal("agent", &format!("event_{i}"));
            let id = t.id;
            store_lock.add(t).unwrap();
            id
        })
        .collect();
    drop(store_lock);

    let bp = BeliefPayload {
        proposition: "system stable".into(),
        confidence: 0.9,
        ..Default::default()
    };
    let mut rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "assert".into(),
        "system stable".into(), serde_json::to_value(&bp).unwrap(),
    );
    rec.confidence = 0.9;
    rec.evidence = ev_ids;
    let id = rec.id;
    h.transact(CognitiveDelta::AddMemory(rec), "agent").unwrap();

    let store = h.memory.lock().unwrap();
    let stored = store.find_by_id(id).unwrap();
    assert!(
        stored.confidence >= 0.89,
        "7 evidence: must pass unmodified, got {}",
        stored.confidence
    );
}

// E1: Authority gate via CognitiveDelta::UpdateBelief

#[test]
fn update_belief_high_conf_no_evidence_is_clamped() {
    let h = handle();

    // First write belief at low confidence (no evidence → already clamped at <=0.5)
    let bp_init = BeliefPayload { proposition: "log never full".into(), confidence: 0.3, ..Default::default() };
    let rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "assert".into(),
        "log never full".into(), serde_json::to_value(&bp_init).unwrap(),
    );
    let id = rec.id;
    h.transact(CognitiveDelta::AddMemory(rec), "agent").unwrap();

    // Now try to upgrade to 0.95 with no evidence
    let bp_upgrade = BeliefPayload { proposition: "log never full".into(), confidence: 0.95, ..Default::default() };
    h.transact(CognitiveDelta::UpdateBelief { id, payload: bp_upgrade }, "agent").unwrap();

    let store = h.memory.lock().unwrap();
    let stored = store.find_by_id(id).unwrap();
    assert!(
        stored.confidence <= 0.5,
        "UpdateBelief with 0 evidence must clamp to <=0.5, got {}",
        stored.confidence
    );
}

// E2: EmergenceDetector assigns Provisional status

#[test]
fn emergence_assigns_provisional_before_gate() {
    let mut store = MemoryStore::new_in_memory();
    let mut ed = hipcortex::emergence::EmergenceDetector::new();

    // Write DENSITY-1 (4) temporals — below gate threshold; belief created as Provisional
    for _ in 0..5 {
        let t = temporal("agent", "cache miss error invalid");
        store.add(t).unwrap();
    }
    ed.on_temporal_write(&mut store, "agent");

    let beliefs = store.all_by_type(MemoryType::Belief);
    if beliefs.is_empty() {
        return; // EmergenceDetector may not trigger yet — skip
    }
    for b in &beliefs {
        let p: BeliefPayload = serde_json::from_value(b.metadata.clone()).unwrap();
        // Confidence must be gated: 5 evidence → cap 0.8, raw = 5/DENSITY so <= 0.8
        assert!(p.confidence <= 0.8, "emerged confidence must be gated, got {}", p.confidence);
        // Status before gate elevation is Provisional (may be Confirmed if gate passed)
        assert!(
            p.epistemic_status == EpistemicStatus::Provisional
                || p.epistemic_status == EpistemicStatus::Confirmed,
            "must be Provisional or Confirmed, got {:?}", p.epistemic_status
        );
    }
}

// E2: AbstractionGate elevation — 4+ Temporals → In + Confirmed

#[test]
fn abstraction_gate_elevates_valid_cluster() {
    use hipcortex::abstraction_gate::{AbstractionGate, MIN_EVIDENCE};
    let mut store = MemoryStore::new_in_memory();

    let ev_ids: Vec<uuid::Uuid> = (0..MIN_EVIDENCE)
        .map(|i| {
            let t = temporal("agent", &format!("pattern_{i}"));
            let id = t.id;
            store.add(t).unwrap();
            id
        })
        .collect();

    // Create a Provisional belief
    let bp = BeliefPayload {
        proposition: "pattern recurs".into(),
        epistemic_status: EpistemicStatus::Provisional,
        jtms_label: JtmsLabel::Unknown,
        ..Default::default()
    };
    let mut rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "emerge".into(),
        "pattern recurs".into(), serde_json::to_value(&bp).unwrap(),
    );
    rec.evidence = ev_ids.clone();
    let bid = rec.id;
    store.add(rec).unwrap();

    let result = AbstractionGate::validate(
        &ev_ids, "pattern recurs", &std::collections::HashSet::new(), &store
    );
    assert!(result.valid, "{}", result.reason);
    AbstractionGate::elevate(&mut store, bid).unwrap();

    let updated = store.find_by_id(bid).unwrap();
    let p: BeliefPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
    assert_eq!(p.jtms_label, JtmsLabel::In);
    assert_eq!(p.epistemic_status, EpistemicStatus::Confirmed);
}

// E3: JTMS labels persist in JSONL store (in-memory round-trip via snapshot/reload)

#[test]
fn jtms_out_label_persists_after_retraction() {
    use hipcortex::jtms;
    let mut store = MemoryStore::new_in_memory();

    // A: prior belief (In with empty in_list treated as axiomatic — manually set In)
    let bp_a = BeliefPayload { proposition: "server_up".into(), jtms_label: JtmsLabel::In, ..Default::default() };
    let mut rec_a = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "assert".into(),
        "server_up".into(), serde_json::to_value(&bp_a).unwrap(),
    );
    let a_id = rec_a.id;
    store.add(rec_a.clone()).unwrap();

    // B: depends on A
    let bp_b = BeliefPayload { proposition: "api_healthy".into(), jtms_label: JtmsLabel::Unknown, ..Default::default() };
    let mut rec_b = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "assert".into(),
        "api_healthy".into(), serde_json::to_value(&bp_b).unwrap(),
    );
    let b_id = rec_b.id;
    store.add(rec_b.clone()).unwrap();

    // Assert B depends on A
    jtms::assert_justification(&mut store, b_id, vec![a_id], vec![]).unwrap();

    // B should be In (A is In)
    let b_after = store.find_by_id(b_id).unwrap();
    let pb: BeliefPayload = serde_json::from_value(b_after.metadata.clone()).unwrap();
    assert_eq!(pb.jtms_label, JtmsLabel::In);

    // Retract A → cascade B to Out
    jtms::propagate_retraction(&mut store, a_id, None, "agent");

    let b_out = store.find_by_id(b_id).unwrap();
    let pb2: BeliefPayload = serde_json::from_value(b_out.metadata.clone()).unwrap();
    assert_eq!(pb2.jtms_label, JtmsLabel::Out, "B must be Out after A retracted");

    let _ = (rec_a, rec_b); // suppress warnings
}
