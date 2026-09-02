// SIT: workspace persistence across restart (AC-5, G-WS).
//
// Verifies:
// 1. A workspace saved to disk reloads with same ID + OR-Set contents.
// 2. load_all restores multiple workspaces from a directory.
// 3. OR-Set additions survive a save/load cycle without data loss.

use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::workspace::{Workspace, WorkspaceId, WorkspaceMode};
use tempfile::tempdir;

fn dummy_record(actor: &str, label: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.to_string(),
        "test".to_string(),
        label.to_string(),
        serde_json::json!({}),
    )
}

#[test]
fn workspace_saves_and_reloads() {
    let dir = tempdir().expect("tempdir");
    let store = MemoryStore::new_in_memory();

    let ws_id = WorkspaceId::new();
    let mut ws = Workspace::open(ws_id.clone(), WorkspaceMode::Private, &store);

    let rec = dummy_record("agent-1", "test-record");
    let rec_id = rec.id;
    ws.add_record(rec, "agent-1");

    ws.save(dir.path()).expect("save must succeed");

    // Simulate restart — reload from disk.
    let ws2 = Workspace::load(
        &dir.path().join(format!("workspace_{}.jsonl", ws_id.0)),
    )
    .expect("load must succeed");

    assert_eq!(ws2.id.0, ws_id.0, "reloaded workspace must have same ID");
    let live = ws2.live_records();
    assert!(
        live.iter().any(|r| r.id == rec_id),
        "reloaded workspace must contain the added record",
    );
}

#[test]
fn load_all_restores_multiple_workspaces() {
    let dir = tempdir().expect("tempdir");

    for i in 0..3 {
        let store = MemoryStore::new_in_memory();
        let mut ws = Workspace::open(WorkspaceId::new(), WorkspaceMode::Shared, &store);
        ws.add_record(dummy_record("agent", &format!("rec-{i}")), "agent");
        ws.save(dir.path()).expect("save");
    }

    let loaded = Workspace::load_all(dir.path());
    assert_eq!(loaded.len(), 3, "must load all 3 workspaces");
}

#[test]
fn workspace_or_set_survives_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let store = MemoryStore::new_in_memory();

    let ws_id = WorkspaceId::new();
    let mut ws = Workspace::open(ws_id.clone(), WorkspaceMode::Shared, &store);

    let r1 = dummy_record("agent-1", "record-one");
    let r2 = dummy_record("agent-1", "record-two");
    let r1_id = r1.id;
    let r2_id = r2.id;

    ws.add_record(r1, "agent-1");
    ws.add_record(r2, "agent-1");

    ws.save(dir.path()).expect("save");
    let ws2 = Workspace::load(
        &dir.path().join(format!("workspace_{}.jsonl", ws_id.0)),
    )
    .expect("load");

    let ids: std::collections::HashSet<uuid::Uuid> =
        ws2.live_records().iter().map(|r| r.id).collect();

    assert!(ids.contains(&r1_id), "OR-Set must contain r1 after reload");
    assert!(ids.contains(&r2_id), "OR-Set must contain r2 after reload");
    assert_eq!(ids.len(), 2, "no spurious records after reload");
}
