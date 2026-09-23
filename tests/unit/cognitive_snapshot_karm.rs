use std::sync::{Arc, Mutex, RwLock};
use hipcortex::cognitive_state::{CognitiveHandle, CognitiveSnapshot};
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::InMemoryBackend;

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

#[test]
fn snapshot_new_fields_serde_default() {
    let old_json = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000001",
        "tx_cursor": 0u64,
        "actor": "test",
        "temporal": {"record_count": 0, "recent_actions": [], "temporal_span_ms": 0},
        "world": {"node_count": 0, "edge_count": 0, "dag_verified": true},
        "self_model": {"calibration_score": 1.0, "prediction_error_ewma": 0.0, "consolidation_pressure": 0.0, "epistemic_entropy": 0.0, "healthy": true},
        "goals": [],
        "skills": [],
        "beliefs": {"count": 0, "mean_confidence": 0.0, "epistemic_entropy": 0.0, "beliefs": []},
        "provenance": {"merkle_root_hex": "", "record_count": 0, "evidence_edge_count": 0}
    });
    let snap: CognitiveSnapshot = serde_json::from_value(old_json).expect("must parse without new fields");
    assert!(snap.laws.is_empty());
    assert!(snap.policies.is_empty());
    assert!(snap.failures.is_empty());
    assert!(!snap.uncertainty.uncertain);
}

#[test]
fn laws_appear_on_snapshot() {
    let handle = make_handle();
    let payload = serde_json::json!({ "equation": "p->q", "mdl_score": 0.8, "domain": "test" });
    let rec = MemoryRecord::new(
        MemoryType::Law,
        "agent".to_string(),
        "extracted_law".to_string(),
        "LawExtractor".to_string(),
        payload,
    );
    handle.memory.lock().unwrap().add(rec).unwrap();
    let snap = handle.snapshot("agent").unwrap();
    assert_eq!(snap.laws.len(), 1);
    assert_eq!(snap.laws[0].equation, "p->q");
}

#[test]
fn actor_isolation_laws() {
    let handle = make_handle();
    let rec_a = MemoryRecord::new(
        MemoryType::Law,
        "agent-a".to_string(),
        "law".to_string(),
        "test".to_string(),
        serde_json::json!({ "equation": "p->q" }),
    );
    let rec_b = MemoryRecord::new(
        MemoryType::Law,
        "agent-b".to_string(),
        "law".to_string(),
        "test".to_string(),
        serde_json::json!({ "equation": "x->y" }),
    );
    {
        let mut mem = handle.memory.lock().unwrap();
        mem.add(rec_a).unwrap();
        mem.add(rec_b).unwrap();
    }
    let snap_a = handle.snapshot("agent-a").unwrap();
    assert_eq!(snap_a.laws.len(), 1);
    assert_eq!(snap_a.laws[0].equation, "p->q");
    let snap_b = handle.snapshot("agent-b").unwrap();
    assert_eq!(snap_b.laws.len(), 1);
    assert_eq!(snap_b.laws[0].equation, "x->y");
}

#[test]
fn failures_from_failed_goals() {
    let handle = make_handle();
    let gp = serde_json::json!({ "status": "Failed", "target_state": "do_thing" });
    let rec = MemoryRecord::new(
        MemoryType::Goal,
        "agent".to_string(),
        "failed_goal".to_string(),
        "test".to_string(),
        gp,
    );
    handle.memory.lock().unwrap().add(rec).unwrap();
    let snap = handle.snapshot("agent").unwrap();
    assert_eq!(snap.failures.len(), 1);
    assert!(snap.failures[0].record_type.contains("Goal"));
}
