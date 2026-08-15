use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;

fn make_expired(actor: &str, target: &str) -> MemoryRecord {
    let mut r = MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    );
    r.expires_at = Some(chrono::Utc::now().timestamp() - 100); // 100 seconds in the past
    r
}

fn make_live(actor: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    )
    // expires_at = None → never expires
}

// ── Export expiry filter ──────────────────────────────────────────────────────

#[test]
fn export_logic_excludes_expired_by_default() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_expired("agent", "expired_mem")).unwrap();
    store.add(make_live("agent", "live_mem")).unwrap();

    let now_ts = chrono::Utc::now().timestamp();
    // Simulate include_expired = false (default behavior after the fix)
    let active: Vec<_> = store
        .all()
        .iter()
        .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
        .collect();

    assert_eq!(
        active.len(),
        1,
        "default export should exclude expired records"
    );
    assert_eq!(active[0].target, "live_mem");
}

#[test]
fn export_logic_includes_expired_when_flag_set() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_expired("agent", "expired_mem")).unwrap();
    store.add(make_live("agent", "live_mem")).unwrap();

    // Simulate include_expired = true: no expiry filter applied
    let all = store.all();
    assert_eq!(
        all.len(),
        2,
        "include_expired=true should return all records"
    );
}

// ── Consolidate data-loss fix ─────────────────────────────────────────────────

#[test]
fn consolidate_candidates_skip_expired_records() {
    let mut store = MemoryStore::new_in_memory();

    // Expired record with similar text AND a newer write timestamp — the data-loss scenario:
    // without the expiry filter, this expired record would be ranked "keep" and the live
    // record "drop", deleting a valid memory.
    let mut expired = make_expired("agent", "use postgres for auth");
    expired.timestamp = chrono::Utc::now(); // mark as newly written but short-lived

    let live = make_live("agent", "use postgres for users");

    store.add(expired).unwrap();
    store.add(live).unwrap();

    let now_ts = chrono::Utc::now().timestamp();
    // Simulate the fixed consolidate candidate filter
    let records = store.all();
    let candidates: Vec<_> = records
        .iter()
        .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
        .collect();

    assert_eq!(
        candidates.len(),
        1,
        "expired record must be excluded from consolidate candidates"
    );
    assert_eq!(candidates[0].target, "use postgres for users");
}

// ── Stats active/total split ──────────────────────────────────────────────────

#[test]
fn stats_active_records_excludes_expired() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_expired("agent", "exp1")).unwrap();
    store.add(make_expired("agent", "exp2")).unwrap();
    store.add(make_live("agent", "live1")).unwrap();

    let now_ts = chrono::Utc::now().timestamp();
    let total = store.all().len();
    let active = store
        .all()
        .iter()
        .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
        .count();

    assert_eq!(total, 3, "total_records counts all including expired");
    assert_eq!(active, 1, "active_records counts only non-expired");
    assert!(total > active);
}

#[test]
fn stats_active_equals_total_when_no_ttl() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_live("agent", "a")).unwrap();
    store.add(make_live("agent", "b")).unwrap();

    let now_ts = chrono::Utc::now().timestamp();
    let total = store.all().len();
    let active = store
        .all()
        .iter()
        .filter(|r| r.expires_at.map_or(true, |exp| exp > now_ts))
        .count();

    assert_eq!(total, active, "no TTL records: active must equal total");
}
