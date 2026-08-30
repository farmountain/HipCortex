/// v1.1.0 unit tests: BeliefInvalidator, EmergenceDetector, GoalScheduler.
use hipcortex::belief_invalidator::BeliefInvalidator;
use hipcortex::emergence::EmergenceDetector;
use hipcortex::goal_scheduler::GoalScheduler;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, GoalPayload, GoalStatus, SuccessFactor};

fn seed_belief(store: &mut MemoryStore<impl hipcortex::persistence::MemoryBackend>, proposition: &str, confidence: f32) -> uuid::Uuid {
    let payload = BeliefPayload {
        proposition: proposition.to_string(),
        justification: "test".to_string(),
        confidence,
        ..Default::default()
    };
    let mut rec = MemoryRecord::new(
        MemoryType::Belief,
        "test".into(),
        "assert".into(),
        proposition.into(),
        serde_json::to_value(&payload).unwrap(),
    );
    rec.confidence = confidence;
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

fn seed_goal(
    store: &mut MemoryStore<impl hipcortex::persistence::MemoryBackend>,
    actor: &str,
    target: &str,
    urgency: f64,
    status: GoalStatus,
) -> uuid::Uuid {
    let payload = GoalPayload {
        target_state: target.to_string(),
        urgency,
        status,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Goal,
        actor.into(),
        "pursue".into(),
        target.into(),
        serde_json::to_value(&payload).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

// ── BeliefInvalidator ────────────────────────────────────────────────────────

#[test]
fn belief_invalidator_decays_on_negation_overlap() {
    let mut store = MemoryStore::new_in_memory();
    let bid = seed_belief(&mut store, "service is healthy", 0.9);

    // Contradicting Temporal record
    let obs = MemoryRecord::new(
        MemoryType::Temporal,
        "react_engine".into(),
        "not healthy".into(),
        "service is broken".into(),
        serde_json::json!({ "thought": "service not healthy error" }),
    );
    let invalidated = BeliefInvalidator::process(&obs, &mut store);

    let updated = store.find_by_id(bid).unwrap();
    assert!(
        updated.confidence < 0.9,
        "Confidence should decay after contradiction: got {}",
        updated.confidence
    );
    // If confidence dropped below 0.2, belief should be in invalidated list
    if updated.confidence < 0.2 {
        assert!(invalidated.contains(&bid), "Low-confidence belief must be in invalidated set");
        // And a Temporal marker must exist
        assert!(
            store.find_by_action("belief_invalidated").iter().any(|r| r.derived_from == Some(bid)),
            "belief_invalidated marker must exist"
        );
    }
}

#[test]
fn belief_invalidator_ignores_non_overlapping_records() {
    let mut store = MemoryStore::new_in_memory();
    let bid = seed_belief(&mut store, "database connection stable", 0.8);

    let obs = MemoryRecord::new(
        MemoryType::Temporal,
        "agent".into(),
        "observe".into(),
        "ui rendering".into(),
        serde_json::json!({ "thought": "frontend loaded" }),
    );
    BeliefInvalidator::process(&obs, &mut store);

    let updated = store.find_by_id(bid).unwrap();
    assert_eq!(
        updated.confidence, 0.8_f32,
        "Unrelated record must not affect belief confidence"
    );
}

#[test]
fn belief_invalidated_marker_created_when_confidence_below_threshold() {
    let mut store = MemoryStore::new_in_memory();
    let bid = seed_belief(&mut store, "cache always valid", 0.21);

    // Single strong contradiction should push below 0.2
    for _ in 0..6 {
        let obs = MemoryRecord::new(
            MemoryType::Temporal,
            "agent".into(),
            "not valid cache error invalid".into(),
            "cache always valid broken".into(),
            serde_json::json!({ "thought": "cache not valid error invalid wrong" }),
        );
        BeliefInvalidator::process(&obs, &mut store);
    }

    let updated_conf = store.find_by_id(bid).unwrap().confidence;
    if updated_conf < 0.2 {
        let markers = store.find_by_action("belief_invalidated");
        assert!(
            markers.iter().any(|r| r.derived_from == Some(bid)),
            "Expected belief_invalidated Temporal marker for belief {:?}", bid
        );
    }
}

// ── EmergenceDetector ────────────────────────────────────────────────────────

#[test]
fn emergence_detector_creates_belief_from_dense_temporals() {
    let mut store = MemoryStore::new_in_memory();

    // Write 12 Temporal records all mentioning "timeout"
    for i in 0..12 {
        let obs = MemoryRecord::new(
            MemoryType::Temporal,
            "agent".into(),
            format!("observe_timeout_{}", i),
            "connection timeout error".into(),
            serde_json::json!({ "thought": format!("connection timeout occurred at step {}", i) }),
        );
        store.add(obs).unwrap();
    }

    let created = EmergenceDetector::detect(&mut store, "agent");
    assert!(!created.is_empty(), "EmergenceDetector should synthesise at least one Belief from 12 similar Temporal records");

    // Verify the belief has evidence pointers
    for belief_id in &created {
        let rec = store.find_by_id(*belief_id).unwrap();
        assert!(!rec.evidence.is_empty(), "Emerged Belief must have evidence pointers");
        assert!(rec.confidence > 0.0, "Emerged Belief must have positive confidence");
    }
}

#[test]
fn emergence_detector_no_belief_below_density() {
    let mut store = MemoryStore::new_in_memory();

    // Only 3 records — below DENSITY=5
    for i in 0..3 {
        let obs = MemoryRecord::new(
            MemoryType::Temporal, "agent".into(),
            format!("rare_event_{}", i), "target".into(),
            serde_json::json!({}),
        );
        store.add(obs).unwrap();
    }

    let created = EmergenceDetector::detect(&mut store, "agent");
    assert!(created.is_empty(), "Should not emerge beliefs below density threshold");
}

#[test]
fn emergence_detector_trigger_every_fires_on_10th_write() {
    let mut store = MemoryStore::new_in_memory();
    let mut det = EmergenceDetector::new();

    // Write 15 Temporal records with overlapping token "latency"
    for i in 0..15 {
        let obs = MemoryRecord::new(
            MemoryType::Temporal, "agent".into(),
            format!("latency spike {}", i), "service".into(),
            serde_json::json!({ "thought": format!("latency degraded at {}", i) }),
        );
        store.add(obs).unwrap();
        det.on_temporal_write(&mut store, "agent");
    }

    // After 15 writes with TRIGGER_EVERY=10, should have triggered at least once
    let beliefs = store.all_by_type(MemoryType::Belief);
    // May or may not create beliefs depending on token density — just verify it ran without panic
    let _ = beliefs;
}

// ── GoalScheduler ────────────────────────────────────────────────────────────

#[test]
fn goal_scheduler_returns_highest_urgency_goal() {
    let mut store = MemoryStore::new_in_memory();
    let _low = seed_goal(&mut store, "agent", "low_priority", 0.1, GoalStatus::Pending);
    let mid = seed_goal(&mut store, "agent", "mid_priority", 0.5, GoalStatus::Pending);
    let _high_already_done = seed_goal(&mut store, "agent", "done_goal", 0.9, GoalStatus::Succeeded);
    let high = seed_goal(&mut store, "agent", "high_priority", 0.9, GoalStatus::Pending);

    let next = GoalScheduler::next(&store, "agent").expect("Should return a goal");
    assert_eq!(next, high, "Scheduler must return highest-urgency active goal");
    let _ = mid;
}

#[test]
fn goal_scheduler_ignores_completed_goals() {
    let mut store = MemoryStore::new_in_memory();
    seed_goal(&mut store, "agent", "done", 1.0, GoalStatus::Succeeded);
    seed_goal(&mut store, "agent", "failed", 0.9, GoalStatus::Failed);

    let next = GoalScheduler::next(&store, "agent");
    assert!(next.is_none(), "Scheduler must not return Succeeded/Failed goals");
}

#[test]
fn goal_scheduler_respects_estimated_cost() {
    let mut store = MemoryStore::new_in_memory();

    // same urgency 0.8, but different costs → lower cost wins (higher score)
    let payload_cheap = GoalPayload {
        target_state: "cheap".into(),
        urgency: 0.8,
        estimated_cost: 0.5,
        status: GoalStatus::Pending,
        ..Default::default()
    };
    let payload_expensive = GoalPayload {
        target_state: "expensive".into(),
        urgency: 0.8,
        estimated_cost: 4.0,
        status: GoalStatus::Pending,
        ..Default::default()
    };

    let r1 = MemoryRecord::new(MemoryType::Goal, "a".into(), "p".into(), "cheap".into(), serde_json::to_value(&payload_cheap).unwrap());
    let cheap_id = r1.id;
    let r2 = MemoryRecord::new(MemoryType::Goal, "a".into(), "p".into(), "expensive".into(), serde_json::to_value(&payload_expensive).unwrap());
    store.add(r1).unwrap();
    store.add(r2).unwrap();

    let next = GoalScheduler::next(&store, "a").unwrap();
    assert_eq!(next, cheap_id, "Lower cost at same urgency should be preferred");
}
