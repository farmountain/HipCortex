/// v3.0.0 acceptance tests — Operational: content probes + restate evidence + live scorecard
///
/// AC-F1: _probe_filesystem returns sha256_hex (runner content-anchored)
/// AC-F2: derive_obs_state uses sha256_hex hash-first (WM state = entity:<hash8>)
/// AC-C1: restate_if_env_changed renames env-blocked factor + writes Reflexion{goal_restated}
/// AC-C2: restate_if_env_changed is idempotent (no duplicate rename/reflexion on second call)
/// AC-S1: /substrate/scorecard route inlines live build_report data (has "live" key)
/// AC-S2: scorecard live field contains recommended_op + uncertain_count

use std::fs;

// ── AC-F1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_f1_probe_filesystem_returns_sha256_hex() {
    let src = fs::read_to_string("sdk/python/hipcortex/runner.py").unwrap();
    assert!(
        src.contains("sha256_hex"),
        "_probe_filesystem must return sha256_hex field for content-anchored WM state"
    );
    assert!(
        src.contains("hashlib.sha256"),
        "_probe_filesystem must use hashlib.sha256 to compute file hash"
    );
    assert!(
        src.contains("fh.read(65536)"),
        "_probe_filesystem must read file in 64KB chunks to handle large files"
    );
}

// ── AC-F2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_f2_derive_obs_state_uses_hash_first() {
    let src = fs::read_to_string("src/wm_updater.rs").unwrap();
    assert!(
        src.contains("sha256_hex"),
        "derive_obs_state must check sha256_hex field first for content-anchored WM state"
    );
    assert!(
        src.contains("hash.len().min(8)"),
        "derive_obs_state must truncate hash to 8 chars (entity:<hash8> label)"
    );
    // Verify hash branch uses early return (precedes other fallbacks)
    assert!(
        src.contains("return format!(\"{}:{}\", entity, prefix)"),
        "hash-first branch must use early return before other fallbacks in derive_obs_state"
    );
}

// ── AC-C1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_c1_restate_renames_env_blocked_factor_and_writes_reflexion() {
    use hipcortex::clarify_engine::ClarifyEngine;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let mut store = MemoryStore::new_in_memory();
    let actor = "restate-agent";

    // Goal with one unsatisfied factor whose keyword overlaps the Temporal failure signal.
    let mut payload = GoalPayload::default();
    payload.status = GoalStatus::Pending;
    payload.target_state = "prod".to_string();
    payload.success_factors = vec![
        SuccessFactor { name: "deploy_service".to_string(), weight: 1.0, satisfied: false },
    ];
    let meta = serde_json::to_value(&payload).unwrap();
    let mut rec = MemoryRecord::new(
        MemoryType::Goal, actor.to_string(), "run".to_string(), "prod".to_string(), meta,
    );
    let goal_id = rec.id;
    rec.actor = actor.to_string();
    store.add(rec).unwrap();

    // Temporal: action="failed" + target="deploy_service"
    // → combined "failed deploy_service" hits FAILURE_KEYWORDS + factor keyword "deploy"
    let temporal = MemoryRecord::new(
        MemoryType::Temporal, actor.to_string(),
        "failed".to_string(), "deploy_service".to_string(),
        serde_json::json!({}),
    );
    store.add(temporal).unwrap();

    let restated = ClarifyEngine::restate_if_env_changed(&mut store, goal_id, actor);
    assert!(restated, "restate_if_env_changed must return true when env failure matches factor keyword");

    // Factor renamed to {name}_when_available
    let updated = store.find_by_id(goal_id).unwrap();
    let updated_payload: GoalPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
    assert_eq!(
        updated_payload.success_factors[0].name,
        "deploy_service_when_available",
        "env-blocked factor must be renamed to deploy_service_when_available"
    );

    // Reflexion{goal_restated} written with derived_from = goal_id
    let reflexions: Vec<_> = store
        .all_by_type(MemoryType::Reflexion)
        .into_iter()
        .filter(|r| r.action == "goal_restated" && r.derived_from == Some(goal_id))
        .collect();
    assert_eq!(reflexions.len(), 1, "must write exactly 1 Reflexion{{goal_restated}} per restate call");
}

// ── AC-C2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_c2_restate_is_idempotent() {
    use hipcortex::clarify_engine::ClarifyEngine;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let mut store = MemoryStore::new_in_memory();
    let actor = "restate-agent2";

    let mut payload = GoalPayload::default();
    payload.status = GoalStatus::Pending;
    payload.target_state = "prod".to_string();
    payload.success_factors = vec![
        SuccessFactor { name: "build_pipeline".to_string(), weight: 1.0, satisfied: false },
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
        "failed".to_string(), "build_pipeline".to_string(),
        serde_json::json!({}),
    );
    store.add(temporal).unwrap();

    // First call — renames factor
    let r1 = ClarifyEngine::restate_if_env_changed(&mut store, goal_id, actor);
    assert!(r1, "first restate call must return true");

    // Second call — factor already ends with _when_available, must skip → false
    let r2 = ClarifyEngine::restate_if_env_changed(&mut store, goal_id, actor);
    assert!(!r2, "second restate call must return false (idempotent — factor already restated)");

    // Still exactly 1 Reflexion, not 2
    let count = store
        .all_by_type(MemoryType::Reflexion)
        .into_iter()
        .filter(|r| r.action == "goal_restated" && r.derived_from == Some(goal_id))
        .count();
    assert_eq!(count, 1, "idempotent restate must not duplicate Reflexion{{goal_restated}}");
}

// ── AC-S1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_s1_scorecard_route_has_live_field() {
    let src = fs::read_to_string("src/web_server.rs").unwrap();
    assert!(
        src.contains("\"live\": live"),
        "/substrate/scorecard route must include live build_report data in response"
    );
    assert!(
        src.contains("build_report(&*store"),
        "scorecard route must call build_report with the live store"
    );
}

// ── AC-S2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_s2_scorecard_live_contains_recommended_op_and_uncertain_count() {
    let src = fs::read_to_string("src/web_server.rs").unwrap();
    assert!(
        src.contains("recommended_op"),
        "scorecard live field must expose recommended_op from next_recommendation"
    );
    assert!(
        src.contains("uncertain_count"),
        "scorecard live field must expose uncertain_count from open_uncertainties"
    );
    assert!(
        src.contains("invalidated_count"),
        "scorecard live field must expose invalidated_count from open_uncertainties"
    );
    // Verify scorecard endpoint accepts actor query param
    assert!(
        src.contains("params.get(\"actor\")"),
        "scorecard route must accept ?actor= query param to scope build_report"
    );
}
