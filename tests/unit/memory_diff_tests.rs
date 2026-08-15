use hipcortex::memory_diff::{compute_diff, StateDiff};
use hipcortex::memory_record::{MemoryRecord, MemoryType};

#[test]
fn test_compute_diff_detects_confidence_change() {
    let mut r1 = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "b".into(),
        "c".into(),
        serde_json::json!({}),
    );
    r1.confidence = 0.9;

    let mut r2 = r1.clone();
    r2.confidence = 0.4;
    r2.version = 1;
    r2.status = "quarantine".to_string();

    let diff = compute_diff(&r1, &r2);
    assert_eq!(diff.record_id, r1.id);
    assert_eq!(diff.from_version, 0);
    assert_eq!(diff.to_version, 1);
    assert!(
        (diff.confidence_delta - (-0.5_f32)).abs() < 0.001,
        "wrong confidence delta: {}",
        diff.confidence_delta
    );
    assert!(diff.status_change.is_some(), "status change not detected");
    let (from_s, to_s) = diff.status_change.unwrap();
    assert_eq!(from_s, "active");
    assert_eq!(to_s, "quarantine");
}

#[test]
fn test_compute_diff_no_changes() {
    let r1 = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "b".into(),
        "c".into(),
        serde_json::json!({}),
    );
    let r2 = r1.clone();
    let diff = compute_diff(&r1, &r2);
    assert!(diff.field_changes.is_empty());
    assert!(diff.status_change.is_none());
    assert_eq!(diff.confidence_delta, 0.0);
}
