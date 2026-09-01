use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::cognitive_state::{CognitiveDelta, CognitiveHandle};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::sync::{Arc, Mutex, RwLock};

fn make_handle() -> CognitiveHandle<InMemoryBackend> {
    CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        Arc::new(RwLock::new(WorldModelEnhanced::new())),
        Arc::new(SelfModel::new()),
        None,
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    )
}

fn temporal(actor: &str, target: &str, iteration: Option<u32>) -> MemoryRecord {
    let mut r = MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "act".into(),
        target.into(),
        serde_json::json!({"action": "act"}),
    );
    r.react_iteration = iteration;
    r
}

// AC-G1a: AddMemory(Temporal) fires WMUpdater → world model has transitions
#[test]
fn test_add_temporal_fires_wm_updater() {
    let handle = make_handle();
    let r = temporal("agent", "node", Some(0));
    handle
        .transact(CognitiveDelta::AddMemory(r), "agent")
        .expect("transact failed");

    assert!(
        handle.wm_transition_count() > 0,
        "WMUpdater must feed Temporal into world model on AddMemory"
    );
}

// AC-G1b: AddMemory(Temporal) fires BeliefInvalidator — no panic on negation content
#[test]
fn test_add_temporal_fires_belief_invalidator() {
    let handle = make_handle();

    // Write a Symbolic (belief-like) record first
    let belief = MemoryRecord::new(
        MemoryType::Symbolic,
        "agent".into(),
        "assert".into(),
        "sky is blue".into(),
        serde_json::json!({"proposition": "sky is blue"}),
    );
    handle
        .transact(CognitiveDelta::AddMemory(belief), "agent")
        .expect("belief write failed");

    // Write a Temporal with negation keyword — BeliefInvalidator must run without error
    let neg = MemoryRecord::new(
        MemoryType::Temporal,
        "agent".into(),
        "observe".into(),
        "sky is not blue".into(),
        serde_json::json!({"action": "observe"}),
    );
    handle
        .transact(CognitiveDelta::AddMemory(neg), "agent")
        .expect("temporal write with negation must not error");

    // BeliefInvalidator ran — verify store is still healthy
    let store = handle.memory.lock().expect("memory lock");
    assert!(store.all().len() >= 2, "both records must remain in store");
}

// AC-G1c: AddMemory(Temporal) fires EmergenceDetector — 10 writes trigger one scan
#[test]
fn test_emergence_via_add_memory() {
    let handle = make_handle();

    // TRIGGER_EVERY = 10; 10 Temporal writes trigger EmergenceDetector::detect
    for i in 0..10u32 {
        let r = temporal("agent", "target", Some(i));
        handle
            .transact(CognitiveDelta::AddMemory(r), "agent")
            .unwrap_or_else(|e| panic!("write {i} failed: {e:?}"));
    }

    // EmergenceDetector ran without panicking and store is intact
    let store = handle.memory.lock().expect("memory lock");
    assert!(
        store.all().len() >= 10,
        "all 10 temporal records must be stored after emergence scan"
    );
}

// AC-G2a: calibration_score drops below 1.0 after live entropy signal
#[test]
fn test_calibration_score_live_signal() {
    let handle = make_handle();

    // Write 1: single to_state outcome → entropy 0.0 → score stays 1.0
    let r1 = temporal("agent", "node", Some(0));
    handle
        .transact(CognitiveDelta::AddMemory(r1), "agent")
        .expect("write 1 failed");

    // Write 2: different to_state (iter=1) → 2 outcomes → entropy > 0 → score < 1.0
    let r2 = temporal("agent", "node", Some(1));
    handle
        .transact(CognitiveDelta::AddMemory(r2), "agent")
        .expect("write 2 failed");

    let score = handle.health().calibration_score;
    assert!(
        score < 1.0,
        "calibration_score must drop below 1.0 after live entropy; got {}",
        score
    );
}

// AC-G2c: CoherenceChecker with wired topo detects causal violations
#[test]
fn test_causal_violations_detected_with_wired_topo() {
    use hipcortex::topological_memory::{CausalTopoGraph, EdgeType};
    use std::collections::HashMap;

    let coherence = CoherenceChecker::new();

    // Build a topo with cyclic edges A→B and B→A (causal contradiction)
    let mut topo = CausalTopoGraph::new();
    let _ = topo.add_node("A".into(), [0.0f32; 128], HashMap::new());
    let _ = topo.add_node("B".into(), [0.0f32; 128], HashMap::new());
    topo.add_edge("A".into(), "B".into(), EdgeType::Causal, 0.9, 0.9)
        .expect("add A→B failed");
    // B→A creates the contradiction that detect_contradiction catches
    let _ = topo.add_edge("B".into(), "A".into(), EdgeType::Causal, 0.9, 0.9);

    coherence.set_consistency_topo(topo);

    let reports = coherence
        .check_consistency()
        .expect("check_consistency failed");
    assert!(
        !reports.is_empty(),
        "expected causal violation reports with cyclic topo; got empty"
    );
}
