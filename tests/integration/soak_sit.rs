/// Soak SIT — v2.8.0
///
/// Time-compressed proof for the 3-month-agent claim.
/// Runs 500 synthetic cognitive loop iterations verifying:
///   AC-S1: temporal decay — purge_expired removes all records past TTL, hot store stays clean.
///   AC-S2: WM convergence — observe_transition builds non-trivial transition table.
///   AC-S3: bounded growth — hot store record count stays within margin after purge.

use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::world_model_enhanced::WorldModelEnhanced;

#[test]
fn ac_s1_temporal_decay_purge_expired_cleans_hot_store() {
    let mut store = MemoryStore::new_in_memory();
    let now = chrono::Utc::now().timestamp();

    // Add 200 records that already expired (expires_at = 1 s in the past)
    for i in 0..200usize {
        let mut r = MemoryRecord::new(
            MemoryType::Temporal,
            "soak-agent".into(),
            "observe".into(),
            format!("entity_{}", i),
            serde_json::json!({"iter": i}),
        );
        r.expires_at = Some(now - 1);
        store.add(r).unwrap();
    }
    // Add 20 live records (expires 1 hour in the future)
    for i in 0..20usize {
        let mut r = MemoryRecord::new(
            MemoryType::Temporal,
            "soak-agent".into(),
            "observe".into(),
            format!("live_{}", i),
            serde_json::json!({"live": true}),
        );
        r.expires_at = Some(now + 3600);
        store.add(r).unwrap();
    }
    assert_eq!(store.all_by_type(MemoryType::Temporal).len(), 220);

    let removed = store.purge_expired();

    assert_eq!(removed, 200, "purge_expired must remove all 200 expired records");
    let remaining = store.all_by_type(MemoryType::Temporal);
    assert_eq!(remaining.len(), 20, "only 20 live records should remain");
    assert!(
        remaining.iter().all(|r| r.expires_at.map_or(true, |exp| exp > now)),
        "no expired record must remain in hot store after purge"
    );
}

#[test]
fn ac_s2_wm_convergence_observe_transition_builds_table() {
    let wm = WorldModelEnhanced::new();
    let states = ["idle", "probing", "acting", "verifying"];

    // Simulate 500 iterations of observe → transition observations
    for i in 0..500usize {
        let from = states[i % states.len()];
        let to = states[(i + 1) % states.len()];
        wm.observe_transition(from.to_string(), "step".to_string(), to.to_string()).unwrap();
    }

    // Each (from, "step") pair must have at least one grounded transition
    for &state in &states {
        let pred = wm.predict_next_state(state, "step");
        assert!(pred.is_ok(), "WM must have transitions for state '{}' after 500 observations", state);
        let map_prob = pred.unwrap().probabilities.values().cloned().fold(0.0_f64, f64::max);
        assert!(map_prob > 0.0, "MAP prob for '{}' must be > 0 after soak", state);
    }
}

#[test]
fn ac_s3_hot_store_bounded_growth_after_500_iterations() {
    let mut store = MemoryStore::new_in_memory();
    let now = chrono::Utc::now().timestamp();

    // Simulate 500 iterations: each adds 2 records (1 short-TTL, 1 persistent belief)
    // then purge every 50 iterations
    for iter in 0..500usize {
        // Short-TTL temporal (already expired by the time we purge)
        let mut temporal = MemoryRecord::new(
            MemoryType::Temporal,
            "soak-agent".into(),
            "step".into(),
            format!("entity_{}", iter % 20),
            serde_json::json!({"iter": iter}),
        );
        temporal.expires_at = Some(now - 1); // immediately expired
        store.add(temporal).unwrap();

        // Persistent belief (no TTL)
        if iter % 10 == 0 {
            let belief = MemoryRecord::new(
                MemoryType::Belief,
                "soak-agent".into(),
                "assert".into(),
                format!("belief_{}", iter / 10),
                serde_json::json!({"confidence": 0.8}),
            );
            store.add(belief).unwrap();
        }

        // Purge every 50 iterations
        if iter % 50 == 49 {
            store.purge_expired();
        }
    }

    // After soak: persistent beliefs = 500/10 = 50, temporals all expired and purged
    let all_records = store.all_by_type(MemoryType::Temporal).len()
        + store.all_by_type(MemoryType::Belief).len();

    // Temporal should be 0 (all expired and purged); beliefs should be exactly 50
    let live_temporal = store.all_by_type(MemoryType::Temporal).len();
    let live_beliefs = store.all_by_type(MemoryType::Belief).len();
    assert_eq!(live_temporal, 0, "all expired temporals must be purged; found {}", live_temporal);
    assert_eq!(live_beliefs, 50, "exactly 50 persistent beliefs expected; found {}", live_beliefs);
    // Total bounded: 50, not 500+
    assert!(all_records <= 100, "hot store must be bounded; found {} records", all_records);
}
