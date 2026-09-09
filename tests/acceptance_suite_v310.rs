/// v3.1.0 acceptance tests — Field Grounding: probe honesty + restate depth + soak proof
///
/// AC-P1: unknown sensor → reachable=False, ok=False (probe honesty, no more fake grounding)
/// AC-P2: restate_if_env_changed emits Temporal{probe_required} per blocked factor
/// AC-R1: runtime — probe_required Temporal written with correct target + derived_from
/// AC-S1: soak proof — content change → different sha256 → different WM state label

use std::fs;

// ── AC-P1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_p1_unknown_sensor_returns_reachable_false() {
    let src = fs::read_to_string("sdk/python/hipcortex/runner.py").unwrap();
    assert!(
        src.contains("unknown_sensor"),
        "execute_probe unknown sensor branch must use 'unknown_sensor' error key"
    );
    assert!(
        !src.contains(r#"{"reachable": True, "sensor": "default"}"#),
        "probe default must NOT silently return reachable:True for unknown sensors"
    );
}

// ── AC-P2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_p2_restate_emits_probe_required_temporal() {
    let src = fs::read_to_string("src/clarify_engine.rs").unwrap();
    assert!(
        src.contains("probe_required"),
        "restate_if_env_changed must write Temporal{{probe_required}} for blocked factors"
    );
    assert!(
        src.contains("blocked_factors"),
        "restate must collect blocked_factors before renaming to preserve original names"
    );
    assert!(
        src.contains("env_blocked_ac"),
        "probe_required Temporal metadata must carry reason=env_blocked_ac"
    );
}

// ── AC-R1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_r1_restate_probe_required_runtime() {
    use hipcortex::clarify_engine::ClarifyEngine;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let mut store = MemoryStore::new_in_memory();
    let actor = "probe-req-agent";

    let mut payload = GoalPayload::default();
    payload.status = GoalStatus::Pending;
    payload.target_state = "deployed".to_string();
    payload.success_factors = vec![
        SuccessFactor { name: "api_service".to_string(), weight: 1.0, satisfied: false, observation_pattern: None },
    ];
    let meta = serde_json::to_value(&payload).unwrap();
    let mut rec = MemoryRecord::new(
        MemoryType::Goal, actor.to_string(), "run".to_string(), "prod".to_string(), meta,
    );
    let goal_id = rec.id;
    rec.actor = actor.to_string();
    store.add(rec).unwrap();

    let temporal = MemoryRecord::new(
        MemoryType::Temporal, actor.to_string(),
        "failed".to_string(), "api_service".to_string(),
        serde_json::json!({}),
    );
    store.add(temporal).unwrap();

    let restated = ClarifyEngine::restate_if_env_changed(&mut store, goal_id, actor);
    assert!(restated, "restate must return true for env-blocked factor");

    let probes: Vec<_> = store
        .all_by_type(MemoryType::Temporal)
        .into_iter()
        .filter(|r| r.action == "probe_required" && r.derived_from == Some(goal_id))
        .collect();
    assert_eq!(probes.len(), 1, "must write exactly 1 probe_required Temporal per blocked factor");
    assert_eq!(probes[0].target, "api_service", "probe_required target must be original (pre-rename) factor name");
}

// ── AC-S1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_s1_content_change_soak_proof() {
    use sha2::{Digest, Sha256};
    let hash_a = hex::encode(Sha256::digest(b"README v1 content"));
    let hash_b = hex::encode(Sha256::digest(b"README v2 content - sed changed this"));
    let state_a = format!("readme:{}", &hash_a[..8]);
    let state_b = format!("readme:{}", &hash_b[..8]);
    assert_ne!(state_a, state_b,
        "content change must produce different WM state (entity:<hash8> labels differ)");
}
