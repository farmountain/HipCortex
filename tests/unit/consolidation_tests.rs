use hipcortex::archive_store::ArchiveStore;
use hipcortex::consolidation::{consolidate, compute_pressure, ConsolidationConfig};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::backends::petgraph_backend::PetgraphBackend;
use hipcortex::tx_log::TxLog;

fn make_record(actor: &str, tags: &[&str]) -> MemoryRecord {
    let mut r = MemoryRecord::new(
        MemoryType::Temporal,
        actor.to_string(),
        "did".to_string(),
        "x".to_string(),
        serde_json::json!({}),
    );
    r.tags = tags.iter().map(|s| s.to_string()).collect();
    r
}

#[test]
fn pressure_zero_on_empty_store() {
    let store = MemoryStore::new_in_memory();
    let config = ConsolidationConfig::default();
    assert_eq!(compute_pressure(&store, &config), 0.0);
}

#[test]
fn pressure_proportional_to_count() {
    let mut store = MemoryStore::new_in_memory();
    let config = ConsolidationConfig { capacity_limit: 10, ..Default::default() };
    for _ in 0..5 {
        store.add(make_record("a", &["t"])).unwrap();
    }
    let p = compute_pressure(&store, &config);
    assert!((p - 0.5).abs() < 1e-5, "expected 0.5, got {p}");
}

#[test]
fn consolidate_groups_by_actor_and_tags() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let mut store = MemoryStore::new_in_memory();
    let mut archive = ArchiveStore::new(dir.path().join("archive.jsonl"));
    let mut graph = SymbolicStore::from_backend(PetgraphBackend::new());
    let config = ConsolidationConfig { min_group_size: 3, ..Default::default() };

    // Group A: 3 records — should consolidate.
    for _ in 0..3 {
        store.add(make_record("alice", &["work", "rust"])).unwrap();
    }
    // Group B: 2 records — below threshold, must NOT consolidate.
    for _ in 0..2 {
        store.add(make_record("bob", &["misc"])).unwrap();
    }

    let report = consolidate(&mut store, &mut archive, &mut graph, &log, &config).unwrap();

    assert_eq!(report.groups_consolidated, 1, "only group A should consolidate");
    assert_eq!(report.records_archived, 3, "3 originals archived");
    assert_eq!(report.summary_records_created, 1);
    // Hot store: 1 summary + 2 bob records.
    assert_eq!(store.all().len(), 3, "hot store should have 1 summary + 2 bob records");
}

#[test]
fn consolidate_moves_originals_to_archive() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let mut store = MemoryStore::new_in_memory();
    let mut archive = ArchiveStore::new(dir.path().join("archive.jsonl"));
    let mut graph = SymbolicStore::from_backend(PetgraphBackend::new());
    let config = ConsolidationConfig { min_group_size: 2, ..Default::default() };

    let r1 = make_record("agent", &["task"]);
    let r2 = make_record("agent", &["task"]);
    let id1 = r1.id;
    let id2 = r2.id;
    store.add(r1).unwrap();
    store.add(r2).unwrap();

    let report = consolidate(&mut store, &mut archive, &mut graph, &log, &config).unwrap();

    assert_eq!(report.records_archived, 2);
    assert!(report.archived_ids.contains(&id1));
    assert!(report.archived_ids.contains(&id2));

    // Originals gone from hot store.
    assert!(store.find_by_id(id1).is_none(), "id1 should not be in hot store");
    assert!(store.find_by_id(id2).is_none(), "id2 should not be in hot store");

    // Originals present in cold store.
    let cold = archive.load_all().unwrap();
    assert!(cold.iter().any(|r| r.id == id1), "id1 should be in archive");
    assert!(cold.iter().any(|r| r.id == id2), "id2 should be in archive");
}
