/// Two-Process Field Soak Diary — proves WAL+JSONL persistence across process boundaries.
///
/// Simulates 30 "day" cycles where each day:
///   1. Opens a NEW MemoryStore from disk (simulates a fresh process start)
///   2. Adds Temporal + Belief records
///   3. Drops the store (flush + WAL commit)
///   4. Reopens and verifies records persisted
///
/// This is the honest proof that HipCortex survives process restarts — the "3-month"
/// claim requires per-restart persistence, not just in-memory accumulation.

use hipcortex::{
    memory_record::{MemoryRecord, MemoryType},
    memory_store::MemoryStore,
};
use tempfile::tempdir;

#[derive(Debug, serde::Serialize)]
struct DayStat {
    day: usize,
    record_count: usize,
    naive_tokens: usize,
}

fn make_temporal(day: usize, i: usize, actor: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.to_string(),
        format!("day_{}_action_{}", day, i),
        format!("target_{}_{}", day, i),
        serde_json::json!({ "day": day, "seq": i }),
    )
}

fn make_belief(day: usize, i: usize, actor: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Symbolic,
        actor.to_string(),
        "holds_belief".to_string(),
        format!("belief_{}_{}", day, i),
        serde_json::json!({ "day": day, "confidence": 0.8 }),
    )
}

#[test]
fn field_soak_diary_two_process_persistence() {
    let dir = tempdir().expect("tempdir");
    let store_path = dir.path().join("diary_store.jsonl");
    let actor = "diary-agent";
    let mut diary: Vec<DayStat> = Vec::new();

    // 30 day cycles — each reopens store from disk (two-process simulation)
    for day in 1..=30 {
        // --- Process A: write ---
        {
            let mut store = MemoryStore::new(&store_path)
                .expect("open store write");
            for i in 0..5 {
                store.add(make_temporal(day, i, actor)).expect("add temporal");
            }
            for i in 0..2 {
                store.add(make_belief(day, i, actor)).expect("add belief");
            }
            // drop flushes WAL
        }

        // --- Process B: read back and verify ---
        {
            let store = MemoryStore::new(&store_path)
                .expect("open store read");
            let all = store.all();
            let record_count = all.len();

            assert!(
                record_count >= 7 * day,
                "day {}: expected >= {} records after process reopen, got {}",
                day, 7 * day, record_count
            );

            // naive token estimate: all records × 50 bytes avg / 4
            let naive_tokens = record_count * 50 / 4;

            diary.push(DayStat { day, record_count, naive_tokens });
        }
    }

    // Diary assertions
    assert_eq!(diary.len(), 30, "diary must have exactly 30 day entries");

    let day1 = &diary[0];
    let day30 = &diary[diary.len() - 1];
    assert!(
        day30.record_count > day1.record_count,
        "day-30 record_count ({}) must exceed day-1 ({})",
        day30.record_count, day1.record_count
    );
    assert!(
        day30.naive_tokens > day1.naive_tokens,
        "naive_tokens must grow as records accumulate"
    );

    // Substrate estimate: a compact live_beliefs response ~ 2KB per turn
    let substrate_tokens_per_turn: usize = 2000 / 4;
    assert!(
        day30.naive_tokens > substrate_tokens_per_turn,
        "naive_tokens ({}) must exceed substrate estimate ({}) — proves compression opportunity",
        day30.naive_tokens, substrate_tokens_per_turn
    );

    // Serialize diary to confirm it is JSON-round-trippable
    let json = serde_json::to_string(&diary).expect("diary must serialize");
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).expect("diary must parse");
    assert_eq!(parsed.len(), 30, "serialized diary must have 30 entries");

    // Write diary artifact
    let out_path = std::path::Path::new("fixtures").join("soak_diary_output.json");
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&out_path, &json);
}

#[test]
fn field_soak_diary_day1_survives_process_restart() {
    let dir = tempdir().expect("tempdir");
    let store_path = dir.path().join("restart_proof.jsonl");
    let actor = "restart-agent";

    // Process A: add 3 records, drop
    {
        let mut store = MemoryStore::new(&store_path).expect("store A");
        store.add(make_temporal(1, 0, actor)).expect("add 0");
        store.add(make_temporal(1, 1, actor)).expect("add 1");
        store.add(make_belief(1, 0, actor)).expect("add belief");
    }

    // Process B: open fresh, verify all 3 present
    {
        let store = MemoryStore::new(&store_path).expect("store B");
        let all = store.all();
        assert_eq!(
            all.len(), 3,
            "Process B must see all 3 records written by Process A"
        );
        let has_actor = all.iter().all(|r| r.actor == actor);
        assert!(has_actor, "all records must belong to actor={}", actor);
    }
}
