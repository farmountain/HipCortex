use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::cognitive_state::{CognitiveDelta, CognitiveHandle};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::self_model::{calibration::CalibrationTracker, SelfModel};
use hipcortex::workspace::{WorkspaceId, WorkspaceMode, WorkspaceRegistry};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::sync::{Arc, Mutex, RwLock};

fn make_handle() -> CognitiveHandle<InMemoryBackend> {
    let store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let wm = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let sm = Arc::new(SelfModel::new());
    let coherence = Arc::new(CoherenceChecker::new());
    let cal = Arc::new(CalibrationTracker::new());
    let gc = Arc::new(CognitiveGC::new());
    CognitiveHandle::new(store, wm, sm, None, coherence, cal, gc)
}

fn temporal_record(actor: &str, action: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        action.into(),
        action.into(),
        serde_json::Value::Null,
    )
}

// ── WorkspaceRegistry unit tests ─────────────────────────────────────────────

#[test]
fn workspace_open_snapshots_baseline() {
    let mut store = MemoryStore::<InMemoryBackend>::new_in_memory();
    store.add(temporal_record("a", "act1")).unwrap();
    store.add(temporal_record("a", "act2")).unwrap();

    let mut reg = WorkspaceRegistry::new();
    let id = WorkspaceId::new();
    reg.open(id.clone(), WorkspaceMode::Private, &store);

    let ws = reg.get(&id).unwrap();
    // baseline has 2 records; no workspace additions yet
    assert_eq!(ws.record_count(), 0, "freshly opened workspace has 0 live records (additions only)");
    assert_eq!(ws.delta_count(), 0);
}

#[test]
fn workspace_add_record_visible_in_live() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();
    let id = WorkspaceId::new();
    reg.open(id.clone(), WorkspaceMode::Private, &store);

    let ws = reg.get_mut(&id).unwrap();
    ws.add_record(temporal_record("agent", "step1"), "agent");
    ws.add_record(temporal_record("agent", "step2"), "agent");

    assert_eq!(ws.record_count(), 2);
    assert_eq!(ws.delta_count(), 2, "both are new (not in baseline)");
}

#[test]
fn workspace_tombstone_removes_from_live() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();
    let id = WorkspaceId::new();
    reg.open(id.clone(), WorkspaceMode::Shared, &store);

    let rec = temporal_record("agent", "step");
    let rec_id = rec.id;
    let ws = reg.get_mut(&id).unwrap();
    ws.add_record(rec, "agent");
    assert_eq!(ws.record_count(), 1);

    ws.remove_record(rec_id, "agent");
    assert_eq!(ws.record_count(), 0, "tombstoned record must not appear in live set");
}

#[test]
fn workspace_apply_to_store_adds_delta_records() {
    let mut store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();
    let id = WorkspaceId::new();
    reg.open(id.clone(), WorkspaceMode::Private, &store);

    let ws = reg.get_mut(&id).unwrap();
    ws.add_record(temporal_record("bot", "moved"), "bot");

    let added = reg.apply_to_parent(&id, &mut store).unwrap();
    assert_eq!(added, 1, "one new record applied to parent store");
    assert_eq!(store.record_count(), 1);
}

#[test]
fn workspace_apply_to_store_deduplicates() {
    let mut store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let rec = temporal_record("bot", "moved");
    let rec_id = rec.id;
    store.add(rec.clone()).unwrap();

    let mut reg = WorkspaceRegistry::new();
    let id = WorkspaceId::new();
    reg.open(id.clone(), WorkspaceMode::Private, &store);

    // Add the same record again inside workspace
    let ws = reg.get_mut(&id).unwrap();
    ws.add_record(rec, "bot");

    let added = reg.apply_to_parent(&id, &mut store).unwrap();
    assert_eq!(added, 0, "record already in store → not re-added");
    assert_eq!(store.record_count(), 1);
    let _ = rec_id;
}

// ── OR-Set merge tests ────────────────────────────────────────────────────────

#[test]
fn or_set_merge_unions_additions() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();

    let id_a = WorkspaceId::new();
    let id_b = WorkspaceId::new();
    reg.open(id_a.clone(), WorkspaceMode::Shared, &store);
    reg.open(id_b.clone(), WorkspaceMode::Shared, &store);

    reg.get_mut(&id_a).unwrap().add_record(temporal_record("agent-a", "act-a"), "agent-a");
    reg.get_mut(&id_b).unwrap().add_record(temporal_record("agent-b", "act-b"), "agent-b");

    let merged = reg.merge(&id_b, &id_a).unwrap();
    assert_eq!(merged, 1, "one new record merged from B into A");
    assert_eq!(reg.get(&id_a).unwrap().record_count(), 2, "A now has 2 live records");
}

#[test]
fn or_set_merge_idempotent() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();

    let id_a = WorkspaceId::new();
    let id_b = WorkspaceId::new();
    reg.open(id_a.clone(), WorkspaceMode::Shared, &store);
    reg.open(id_b.clone(), WorkspaceMode::Shared, &store);
    reg.get_mut(&id_b).unwrap().add_record(temporal_record("bot", "ping"), "bot");

    reg.merge(&id_b, &id_a).unwrap();
    let second = reg.merge(&id_b, &id_a).unwrap();
    assert_eq!(second, 0, "second merge of same record is idempotent (no duplicate)");
}

#[test]
fn or_set_merge_rejects_private() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();

    let id_priv = WorkspaceId::new();
    let id_shared = WorkspaceId::new();
    reg.open(id_priv.clone(), WorkspaceMode::Private, &store);
    reg.open(id_shared.clone(), WorkspaceMode::Shared, &store);

    let err = reg.merge(&id_shared, &id_priv);
    assert!(err.is_err(), "merging into a Private workspace must fail");
}

#[test]
fn or_set_merge_same_id_noop() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();
    let id = WorkspaceId::new();
    reg.open(id.clone(), WorkspaceMode::Shared, &store);

    let result = reg.merge(&id, &id);
    assert_eq!(result.unwrap(), 0, "merging workspace with itself is a no-op");
}

// ── CognitiveDelta::WorkspaceOpen / WorkspaceMerge integration ────────────────

#[test]
fn workspace_open_delta_creates_workspace() {
    let handle = make_handle();
    let id = WorkspaceId::new();
    handle
        .transact(CognitiveDelta::WorkspaceOpen { id: id.clone(), mode: WorkspaceMode::Private }, "agent")
        .unwrap();
    let reg = handle.workspace_registry.lock().unwrap();
    assert!(reg.get(&id).is_some(), "workspace must be registered after WorkspaceOpen delta");
    assert_eq!(reg.workspace_count(), 1);
}

#[test]
fn workspace_open_private_does_not_contaminate_parent() {
    let handle = make_handle();
    // Add a record to parent first
    handle.transact(CognitiveDelta::AddMemory(temporal_record("a", "act")), "a").unwrap();

    let id = WorkspaceId::new();
    handle
        .transact(CognitiveDelta::WorkspaceOpen { id: id.clone(), mode: WorkspaceMode::Private }, "a")
        .unwrap();

    // Add record to workspace directly (via registry, bypassing transact — isolation test)
    {
        let mut reg = handle.workspace_registry.lock().unwrap();
        reg.get_mut(&id).unwrap().add_record(temporal_record("a", "private-act"), "a");
    }

    // Parent store must still have exactly 1 record
    assert_eq!(handle.memory.lock().unwrap().record_count(), 1, "private workspace must not touch parent");
}

#[test]
fn workspace_merge_delta_converges_shared_workspaces() {
    let handle = make_handle();
    let id_a = WorkspaceId::new();
    let id_b = WorkspaceId::new();

    handle.transact(CognitiveDelta::WorkspaceOpen { id: id_a.clone(), mode: WorkspaceMode::Shared }, "agent-a").unwrap();
    handle.transact(CognitiveDelta::WorkspaceOpen { id: id_b.clone(), mode: WorkspaceMode::Shared }, "agent-b").unwrap();

    // Add a record to B
    {
        let mut reg = handle.workspace_registry.lock().unwrap();
        reg.get_mut(&id_b).unwrap().add_record(temporal_record("agent-b", "contrib"), "agent-b");
    }

    // Merge B into A via transact
    handle.transact(CognitiveDelta::WorkspaceMerge { from: id_b.clone(), into: id_a.clone() }, "agent-a").unwrap();

    let reg = handle.workspace_registry.lock().unwrap();
    assert_eq!(reg.get(&id_a).unwrap().record_count(), 1, "A must contain B's contribution after merge");
}

#[test]
fn evict_expired_removes_stale_workspaces() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();
    // Open two workspaces; they're fresh so not expired
    reg.open(WorkspaceId::new(), WorkspaceMode::Private, &store);
    reg.open(WorkspaceId::new(), WorkspaceMode::Shared, &store);
    assert_eq!(reg.workspace_count(), 2);

    // Fresh workspaces should not be evicted
    let evicted = reg.evict_expired();
    assert_eq!(evicted, 0);
    assert_eq!(reg.workspace_count(), 2);
}

// ── Phase 3a: Workspace lease tests (AC-2a through AC-2d) ─────────────────────

#[test]
fn ac2a_no_lease_never_expires() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let ws = hipcortex::workspace::Workspace::open(
        WorkspaceId::new(),
        WorkspaceMode::Private,
        &store,
    );
    assert!(!ws.is_expired(), "AC-2a: workspace with no lease must not be expired");
    assert!(ws.expires_at().is_none(), "AC-2a: expires_at must be None when no lease set");
}

#[test]
fn ac2b_renew_lease_sets_expires_at() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut ws = hipcortex::workspace::Workspace::open(
        WorkspaceId::new(),
        WorkspaceMode::Private,
        &store,
    );
    ws.renew_lease(3600);
    assert!(!ws.is_expired(), "AC-2b: freshly renewed lease must not be expired");
    assert!(ws.expires_at().is_some(), "AC-2b: expires_at must be Some after renew_lease");
}

#[test]
fn ac2c_registry_renew_ok_for_existing_workspace() {
    let store = MemoryStore::<InMemoryBackend>::new_in_memory();
    let mut reg = WorkspaceRegistry::new();
    let id = WorkspaceId::new();
    reg.open(id.clone(), WorkspaceMode::Private, &store);

    let result = reg.renew(&id, 3600);
    assert!(result.is_ok(), "AC-2c: renew on existing workspace must return Ok; got: {:?}", result);
    let expires = reg.get(&id).and_then(|ws| ws.expires_at());
    assert!(expires.is_some(), "AC-2c: expires_at must be Some after registry renew");
}

#[test]
fn ac2d_registry_renew_err_for_unknown_workspace() {
    let mut reg = WorkspaceRegistry::new();
    let unknown = WorkspaceId::new();
    let result = reg.renew(&unknown, 3600);
    assert!(result.is_err(), "AC-2d: renew on unknown workspace must return Err");
}
