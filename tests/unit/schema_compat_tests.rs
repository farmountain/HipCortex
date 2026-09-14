/// GAP-6 schema compat — old JSON missing new fields must still deserialize.

use hipcortex::memory_record::{MemoryRecord, MemoryType};

#[test]
fn memory_record_old_json_without_new_fields_deserializes() {
    // Minimal JSON as it would have been written before v0.5 added tags, priority, status, etc.
    let old = r#"{
        "id": "00000000-0000-0000-0000-000000000001",
        "record_type": "Temporal",
        "timestamp": "2024-01-01T00:00:00Z",
        "actor": "agent",
        "action": "did",
        "target": "thing"
    }"#;
    let r: MemoryRecord = serde_json::from_str(old).expect("old MemoryRecord JSON must deserialize");
    assert_eq!(r.actor, "agent");
    assert_eq!(r.tags, Vec::<String>::new(), "tags must default to empty vec");
    assert_eq!(r.priority, "normal", "priority must default to 'normal'");
    assert_eq!(r.status, "active", "status must default to 'active'");
    assert!((r.confidence - 1.0).abs() < 1e-6, "confidence must default to 1.0");
    assert_eq!(r.version, 0, "version must default to 0");
    assert!(r.evidence.is_empty());
    assert!(r.derived_from.is_none());
    assert!(r.react_iteration.is_none());
}

#[test]
fn memory_record_save_has_all_required_fields() {
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "actor".into(),
        "action".into(),
        "target".into(),
        serde_json::json!({}),
    );
    let v: serde_json::Value = serde_json::to_value(&r).unwrap();
    for field in &["id", "record_type", "timestamp", "actor", "action", "target", "confidence", "priority", "status"] {
        assert!(v.get(field).is_some(), "field '{}' missing from serialized MemoryRecord", field);
    }
}

#[test]
fn worldmodel_json_version_stamp_present() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("hc-schema-compat-test-{}.json", uuid::Uuid::new_v4()));
    let wm = hipcortex::world_model_enhanced::WorldModelEnhanced::new();
    wm.save(&path).expect("save must succeed");
    let contents = std::fs::read_to_string(&path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert!(v.get("version").is_some(), "worldmodel.json must carry a 'version' field");
    assert!(v["version"].as_u64().unwrap_or(0) >= 1, "version must be >= 1");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn worldmodel_load_from_json_without_version_succeeds() {
    // Simulates loading a worldmodel.json written by an old build (no version key, no entities).
    let dir = std::env::temp_dir();
    let path = dir.join(format!("hc-schema-compat-old-{}.json", uuid::Uuid::new_v4()));
    let old = r#"{
        "transition_counts": {},
        "transition_totals": {},
        "smoothing": 1.0,
        "causal_edges": []
    }"#;
    std::fs::write(&path, old).unwrap();
    hipcortex::world_model_enhanced::WorldModelEnhanced::load(&path)
        .expect("old worldmodel.json without version must load successfully");
    let _ = std::fs::remove_file(&path);
}
