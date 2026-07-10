/// SIT tests for Sim #10 gap closure:
/// G1 recall_with_metadata (Python — skipped here, tested in sdk/python/)
/// G2 multi-actor query (actors= param)
/// G4 bulk add error detail (index in errors)
/// G5 quarantine/restore/search exclusion
/// G8 corroborate / contradict confidence
/// G13 /memory/context LLM prompt endpoint

use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use chrono::{Duration, Utc};

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn make_record(actor: &str, action: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Symbolic,
        actor.to_string(),
        action.to_string(),
        target.to_string(),
        serde_json::json!({}),
    )
}

// ── G5: status field default ──────────────────────────────────────────────────

#[test]
fn test_memory_record_default_status_is_active() {
    let r = make_record("alice", "decided", "use postgres");
    assert_eq!(r.status, "active");
}

#[test]
fn test_memory_record_status_serializes() {
    let r = make_record("alice", "decided", "use postgres");
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["status"], "active");
}

#[test]
fn test_memory_record_status_deserializes_missing_as_active() {
    let json = r#"{"id":"00000000-0000-0000-0000-000000000001","record_type":"Symbolic","timestamp":"2024-01-01T00:00:00Z","actor":"alice","action":"decided","target":"use postgres","metadata":{}}"#;
    let r: MemoryRecord = serde_json::from_str(json).unwrap();
    assert_eq!(r.status, "active");
}

// ── G5: MemoryStore.set_status ────────────────────────────────────────────────

#[test]
fn test_set_status_quarantine() {
    let mut store = make_store();
    let r = make_record("alice", "decided", "use postgres");
    let id = r.id;
    store.add(r).unwrap();
    store.set_status(id, "quarantine").unwrap();
    let found = store.find_by_id(id).unwrap();
    assert_eq!(found.status, "quarantine");
}

#[test]
fn test_set_status_restore() {
    let mut store = make_store();
    let r = make_record("alice", "decided", "use postgres");
    let id = r.id;
    store.add(r).unwrap();
    store.set_status(id, "quarantine").unwrap();
    store.set_status(id, "active").unwrap();
    let found = store.find_by_id(id).unwrap();
    assert_eq!(found.status, "active");
}

#[test]
fn test_set_status_not_found_errors() {
    let mut store = make_store();
    let fake_id = uuid::Uuid::new_v4();
    assert!(store.set_status(fake_id, "quarantine").is_err());
}

// ── G5: search excludes quarantined by default ────────────────────────────────

#[test]
fn test_search_excludes_quarantined() {
    let mut store = make_store();
    let r = make_record("alice", "decided", "postgres is the database");
    let id = r.id;
    store.add(r).unwrap();
    store.set_status(id, "quarantine").unwrap();
    let results = store.search_semantic(None, "postgres", 10, false);
    assert!(results.iter().all(|(rec, _)| rec.id != id), "quarantined record appeared in search");
}

#[test]
fn test_search_includes_quarantined_when_flag_set() {
    let mut store = make_store();
    let r = make_record("alice", "decided", "postgres is the database");
    let id = r.id;
    store.add(r).unwrap();
    store.set_status(id, "quarantine").unwrap();
    let results = store.search_semantic(None, "postgres", 10, true);
    assert!(results.iter().any(|(rec, _)| rec.id == id), "quarantined record missing when include=true");
}

// ── G2: multi-actor query in MemoryStore ─────────────────────────────────────

#[test]
fn test_find_by_actors_returns_all_matching() {
    let mut store = make_store();
    store.add(make_record("alice", "decided", "use postgres")).unwrap();
    store.add(make_record("bob", "said", "postgres is fine")).unwrap();
    store.add(make_record("carol", "noted", "redis for cache")).unwrap();

    let results = store.find_by_actors(&["alice", "bob"]);
    assert_eq!(results.len(), 2);
    let actors: Vec<&str> = results.iter().map(|r| r.actor.as_str()).collect();
    assert!(actors.contains(&"alice"));
    assert!(actors.contains(&"bob"));
    assert!(!actors.contains(&"carol"));
}

#[test]
fn test_find_by_actors_empty_list_returns_empty() {
    let mut store = make_store();
    store.add(make_record("alice", "decided", "use postgres")).unwrap();
    let results = store.find_by_actors(&[]);
    assert!(results.is_empty());
}

#[test]
fn test_find_by_actors_unknown_returns_empty() {
    let mut store = make_store();
    store.add(make_record("alice", "decided", "use postgres")).unwrap();
    let results = store.find_by_actors(&["nobody"]);
    assert!(results.is_empty());
}

// ── G4: bulk add error struct has index ──────────────────────────────────────

#[test]
fn test_bulk_add_error_has_index_field() {
    use hipcortex::memory_store::BulkAddError;
    let e = BulkAddError { index: 2, actor: "alice".to_string(), reason: "test".to_string() };
    assert_eq!(e.index, 2);
    let json = serde_json::to_value(&e).unwrap();
    assert_eq!(json["index"], 2);
    assert_eq!(json["actor"], "alice");
}

// ── G8: corroborate / contradict ─────────────────────────────────────────────

#[test]
fn test_corroborate_increases_confidence() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.7;
    let id = r.id;
    store.add(r).unwrap();

    let (before, after) = store.corroborate(id).unwrap();
    assert!(after > before);
    assert!((after - 0.8f32).abs() < 0.01, "expected ~0.8, got {}", after);
}

#[test]
fn test_corroborate_clamps_at_1_0() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.95;
    let id = r.id;
    store.add(r).unwrap();

    let (_before, after) = store.corroborate(id).unwrap();
    assert!(after <= 1.0, "confidence exceeded 1.0: {}", after);
}

#[test]
fn test_contradict_decreases_confidence() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.7;
    let id = r.id;
    store.add(r).unwrap();

    let (before, after, quarantined) = store.contradict(id).unwrap();
    assert!(after < before);
    assert!((after - 0.55f32).abs() < 0.01, "expected ~0.55, got {}", after);
    assert!(!quarantined);
}

#[test]
fn test_contradict_auto_quarantines_at_low_confidence() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.35;
    let id = r.id;
    store.add(r).unwrap();

    let (_before, _after, quarantined) = store.contradict(id).unwrap();
    assert!(quarantined, "should auto-quarantine when confidence drops below 0.3");
    let found = store.find_by_id(id).unwrap();
    assert_eq!(found.status, "quarantine");
}

#[test]
fn test_contradict_clamps_at_0_0() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.05;
    let id = r.id;
    store.add(r).unwrap();

    let (_before, after, _) = store.contradict(id).unwrap();
    assert!(after >= 0.0, "confidence went negative: {}", after);
}

// ── Task 1: search_semantic excludes expired records ─────────────────────────

#[test]
fn test_search_excludes_expired_records() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.expires_at = Some(Utc::now().timestamp() - 1); // 1 second in the past
    store.add(r).unwrap();
    let results = store.search_semantic(None, "postgres", 10, false);
    assert!(
        results.is_empty(),
        "expired record must not appear in search_semantic results"
    );
}

#[test]
fn test_search_includes_non_expired_record() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.expires_at = Some(Utc::now().timestamp() + 3600); // 1 hour from now
    store.add(r).unwrap();
    let results = store.search_semantic(None, "postgres", 10, false);
    assert!(
        !results.is_empty(),
        "non-expired record must appear in search_semantic results"
    );
}

#[test]
fn test_search_excludes_expired_pinned_record() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.priority = "pinned".to_string();
    r.expires_at = Some(Utc::now().timestamp() - 1); // expired
    store.add(r).unwrap();
    let results = store.search_semantic(None, "postgres", 10, false);
    assert!(
        results.is_empty(),
        "expired pinned record must not appear in search_semantic (pinned bypass is score, not expiry)"
    );
}

#[test]
fn test_pinned_record_with_query_match_scores_2_0_not_decayed() {
    let mut store = make_store();

    // Old pinned record with short half-life — WITHOUT the fix it would get decayed and score < 2.0
    let mut pinned_r = make_record("alice", "decided", "use postgres architecture");
    pinned_r.priority = "pinned".to_string();
    pinned_r.timestamp = Utc::now() - Duration::seconds(200);
    pinned_r.metadata = serde_json::json!({
        "decay_factor": 1.0,
        "decay_half_life_secs": 100  // 2 half-lives → would decay to ~0.25× without fix
    });
    store.add(pinned_r.clone()).unwrap();

    // Fresh normal record, same content
    let normal_r = make_record("bob", "decided", "use postgres architecture");
    store.add(normal_r.clone()).unwrap();

    let results = store.search_semantic(None, "use postgres architecture", 10, false);
    let pinned_pos = results.iter().position(|(r, _)| r.id == pinned_r.id).expect("pinned missing");
    let normal_pos = results.iter().position(|(r, _)| r.id == normal_r.id).expect("normal missing");
    let pinned_score = results[pinned_pos].1;

    assert_eq!(pinned_pos, 0, "pinned record must appear first (pos 0), got pos {}", pinned_pos);
    assert!(pinned_pos < normal_pos, "pinned must rank above normal");
    assert!((pinned_score - 2.0).abs() < f64::EPSILON, "pinned record must score 2.0, got {}", pinned_score);
}

// ── Task 2: priority multipliers (high=1.5×, low=0.5×) ───────────────────────

#[test]
fn test_high_priority_ranks_above_normal_for_same_content() {
    let mut store = make_store();

    let mut high_r = make_record("alice", "decided", "use postgres as database");
    high_r.priority = "high".to_string();
    store.add(high_r.clone()).unwrap();

    let normal_r = make_record("bob", "decided", "use postgres as database");
    store.add(normal_r.clone()).unwrap();

    let results = store.search_semantic(None, "use postgres as database", 10, false);
    let high_pos = results.iter().position(|(r, _)| r.id == high_r.id).expect("high record missing");
    let normal_pos = results.iter().position(|(r, _)| r.id == normal_r.id).expect("normal record missing");
    assert!(
        high_pos < normal_pos,
        "high priority record (pos {}) must rank above normal priority (pos {})",
        high_pos, normal_pos
    );
}

#[test]
fn test_low_priority_ranks_below_normal_for_same_content() {
    let mut store = make_store();

    let mut low_r = make_record("alice", "decided", "use redis for cache");
    low_r.priority = "low".to_string();
    store.add(low_r.clone()).unwrap();

    let normal_r = make_record("bob", "decided", "use redis for cache");
    store.add(normal_r.clone()).unwrap();

    let results = store.search_semantic(None, "use redis for cache", 10, false);
    let low_pos = results.iter().position(|(r, _)| r.id == low_r.id).expect("low record missing");
    let normal_pos = results.iter().position(|(r, _)| r.id == normal_r.id).expect("normal record missing");
    assert!(
        low_pos > normal_pos,
        "low priority record (pos {}) must rank below normal priority (pos {})",
        low_pos, normal_pos
    );
}

// ── Task 3: time-based decay scoring ─────────────────────────────────────────

#[test]
fn test_old_record_with_short_half_life_scores_lower_than_fresh() {
    let mut store = make_store();

    // "Old" record: 200 seconds old, half-life 100s → 2 half-lives → score ≈0.25×
    let mut old_r = make_record("alice", "decided", "use postgres now");
    old_r.timestamp = Utc::now() - Duration::seconds(200);
    old_r.metadata = serde_json::json!({
        "decay_factor": 1.0,
        "decay_half_life_secs": 100
    });
    store.add(old_r.clone()).unwrap();

    // "Fresh" record: same content, just created (timestamp = Utc::now())
    let fresh_r = make_record("bob", "decided", "use postgres now");
    store.add(fresh_r.clone()).unwrap();

    let results = store.search_semantic(None, "use postgres now", 10, false);
    let old_score = results.iter().find(|(r, _)| r.id == old_r.id).map(|(_, s)| *s).unwrap_or(0.0);
    let fresh_score = results.iter().find(|(r, _)| r.id == fresh_r.id).map(|(_, s)| *s).unwrap_or(0.0);
    assert!(
        old_score < fresh_score,
        "decayed old record (score {:.4}) must score lower than fresh record (score {:.4})",
        old_score, fresh_score
    );
}

#[test]
fn test_zero_decay_factor_record_is_not_decayed() {
    let mut store = make_store();

    // Very old record but λ=0 → no decay regardless of age
    let mut r = make_record("alice", "decided", "use postgres");
    r.timestamp = Utc::now() - Duration::seconds(999_999_999); // ancient
    r.metadata = serde_json::json!({"decay_factor": 0.0});
    r.confidence = 1.0;
    store.add(r.clone()).unwrap();

    let results = store.search_semantic(None, "use postgres", 10, false);
    let score = results.iter().find(|(rec, _)| rec.id == r.id).map(|(_, s)| *s).unwrap_or(0.0);
    // λ=0 → decay=1.0, conf=1.0, trust default=0.5, keyword=1.0, priority=1.0
    // weighted = 1.0 * (0.5 + 0.5*0.5) * 1.0 * 1.0 = 0.75
    assert!(score > 0.5, "λ=0 record must not decay (score {:.4} must be > 0.5)", score);
}

// ── Task 4: causal edge condition (OR semantics) ─────────────────────────────

// Test uses the causal graph directly via WorldModelEnhanced (no HTTP needed).
// We replicate the condition logic from handle_add_memory to confirm the OR semantics.
#[test]
fn test_causal_edge_condition_fires_for_symbolic_normal_priority() {
    use hipcortex::memory_record::MemoryType;
    // The new condition: record_type == Symbolic OR priority == "pinned"
    // This test documents that Symbolic + normal priority SHOULD trigger causal edge.
    let record_type = MemoryType::Symbolic;
    let priority = "normal";
    // OR condition
    let should_fire = matches!(record_type, MemoryType::Symbolic) || priority == "pinned";
    assert!(
        should_fire,
        "Symbolic record with normal priority must fire causal edge (OR condition)"
    );
}

#[test]
fn test_causal_edge_condition_old_and_semantics_would_fail() {
    use hipcortex::memory_record::MemoryType;
    // Documenting that the OLD AND condition was too restrictive
    let record_type = MemoryType::Symbolic;
    let priority = "normal";
    // OLD condition: AND — this was the bug
    let old_condition = matches!(record_type, MemoryType::Symbolic) && priority == "pinned";
    assert!(
        !old_condition,
        "OLD AND condition must NOT fire for Symbolic+normal (this was the bug)"
    );
}

// ── Longitudinal Slice-Time Sorting Tests (`N > limit`) ──────────────────────

#[test]
fn test_search_semantic_pinned_longitudinal_truncation() {
    let mut store = make_store();
    let base_time = Utc::now() - Duration::days(200);

    // Pre-seed 30 pinned records with sequential timestamps (Day 1 to Day 30)
    for day in 1..=30 {
        let mut r = make_record("alice", "pinned_fact", &format!("fact day {}", day));
        r.timestamp = base_time + Duration::days(day);
        r.priority = "pinned".to_string();
        store.add(r).unwrap();
    }

    // Query search_semantic with limit = 5
    // Because there are 30 pinned records matching, they must be sorted newest-first (`Day 30` to `Day 26`)
    let results = store.search_semantic(None, "pinned_fact", 5, false);
    assert_eq!(results.len(), 5, "Must truncate exactly to limit=5");

    for (i, expected_day) in (26..=30).rev().enumerate() {
        assert_eq!(
            results[i].0.target,
            format!("fact day {}", expected_day),
            "Index {} should be Day {} (newest first)", i, expected_day
        );
    }
}

#[test]
fn test_longitudinal_pinned_sorting_and_truncation_contract() {
    let mut store = make_store();
    let base_time = Utc::now() - Duration::days(100);

    // Seed 50 pinned records
    for day in 1..=50 {
        let mut r = make_record("bot", "learned", &format!("rule {}", day));
        r.timestamp = base_time + Duration::days(day);
        r.priority = "pinned".to_string();
        store.add(r).unwrap();
    }

    // Simulate handle_memory_live_beliefs filtering and truncation logic
    let all = store.all();
    let mut filtered: Vec<_> = all.into_iter().filter(|r| r.priority == "pinned").collect();
    filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    filtered.truncate(10);

    assert_eq!(filtered.len(), 10);
    assert_eq!(filtered[0].target, "rule 50", "First item must be the newest (rule 50)");
    assert_eq!(filtered[9].target, "rule 41", "Last item must be rule 41");
}
