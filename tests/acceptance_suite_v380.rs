/// v3.8.0 — Production-Grade Goal Lifecycle
///
/// Closes 4 gaps identified after v3.7.0:
/// 1. Completion heuristic thin: was_surprising=true now required (semantic match)
/// 2. Duration / continuous service: production_pair_setup.py + production_deployment.md
/// 3. Runner dual-role: _poll_and_receipt replaces open+receipt in run_guided probe path
/// 4. Long-horizon goal revision: drift detection (consecutive_low_score >= 3 → Reflexion)
///
/// AC-GS1: cognitive_state.rs syncs was_surprising to intent MemoryRecord (not just status)
/// AC-GS2: loop_engine.rs scorer filters was_surprising == true
/// AC-GS3: scripts/production_pair_setup.py exists with systemd/nssm config generation
/// AC-GS4: docs/production_deployment.md exists with IDE-closed production pair pattern
/// AC-GS5: diary continuous_service == true
/// AC-GS6: hipcortex_runner.py contains _poll_and_receipt (single-role: runner receipts only)
/// AC-GS7: run_guided probe path calls _poll_and_receipt NOT _open_intent directly
/// AC-GS8: GoalPayload has consecutive_low_score field (drift tracking)
/// AC-GS9: loop_engine.rs contains goal_revision_proposed (drift trigger output)
/// AC-GS10: drift detection resets counter after firing (consecutive_low_score = 0, bounded exit)

use std::fs;
use std::path::Path;

fn slurp(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn read_diary() -> serde_json::Value {
    let raw = slurp("docs/longrun_soak_example.json");
    serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null)
}

#[test]
fn ac_gs1_cognitive_state_syncs_was_surprising_to_intent_record() {
    let src = slurp("src/cognitive_state.rs");
    assert!(
        src.contains("was_surprising") && src.contains("meta[\"was_surprising\"]"),
        "AC-GS1: accept_receipt_impl must write was_surprising to intent MemoryRecord metadata"
    );
}

#[test]
fn ac_gs2_scorer_filters_was_surprising() {
    let src = slurp("src/modules/loop_engine.rs");
    assert!(
        src.contains("was_surprising"),
        "AC-GS2: score_success_factors_from_intents must filter was_surprising == true"
    );
    assert!(
        src.contains("score_success_factors_from_intents"),
        "AC-GS2: score_success_factors_from_intents must exist in loop_engine.rs"
    );
}

#[test]
fn ac_gs3_production_pair_setup_exists() {
    assert!(
        Path::new("scripts/production_pair_setup.py").exists(),
        "AC-GS3: scripts/production_pair_setup.py must exist"
    );
    let src = slurp("scripts/production_pair_setup.py");
    assert!(
        src.contains("systemd") || src.contains("nssm"),
        "AC-GS3: production_pair_setup.py must contain systemd or nssm service config generation"
    );
    assert!(
        src.contains("hipcortex-server") && src.contains("hipcortex-runner"),
        "AC-GS3: must generate configs for both server and runner services"
    );
}

#[test]
fn ac_gs4_production_deployment_doc_exists() {
    assert!(
        Path::new("docs/production_deployment.md").exists(),
        "AC-GS4: docs/production_deployment.md must exist"
    );
    let doc = slurp("docs/production_deployment.md");
    assert!(
        doc.contains("IDE-closed") || doc.contains("continuous service"),
        "AC-GS4: production_deployment.md must document the IDE-closed production pair pattern"
    );
    assert!(
        doc.contains("WAL") || doc.contains("restart"),
        "AC-GS4: must document WAL persistence / restart proof"
    );
}

#[test]
fn ac_gs5_diary_continuous_service_true() {
    let diary = read_diary();
    assert!(
        !diary.is_null(),
        "AC-GS5: docs/longrun_soak_example.json must exist and be valid JSON"
    );
    assert!(
        diary["continuous_service"].as_bool().unwrap_or(false),
        "AC-GS5: diary.continuous_service must be true"
    );
}

#[test]
fn ac_gs6_runner_has_poll_and_receipt() {
    let src = slurp("scripts/hipcortex_runner.py");
    assert!(
        src.contains("_poll_and_receipt"),
        "AC-GS6: hipcortex_runner.py must contain _poll_and_receipt function (single-role: runner receipts)"
    );
    assert!(
        src.contains("GET /intent/open") || src.contains("\"/intent/open\""),
        "AC-GS6: _poll_and_receipt must poll /intent/open for daemon-opened intents"
    );
}

#[test]
fn ac_gs7_run_guided_probe_uses_poll_and_receipt_not_open_intent() {
    let src = slurp("scripts/hipcortex_runner.py");
    // Find run_guided function body
    let guided_start = src.find("def run_guided(").unwrap_or(usize::MAX);
    let next_def = src[guided_start + 1..]
        .find("\ndef ")
        .map(|p| guided_start + 1 + p)
        .unwrap_or(src.len());
    let guided_body = &src[guided_start..next_def];
    assert!(
        guided_body.contains("_poll_and_receipt"),
        "AC-GS7: run_guided must call _poll_and_receipt in probe path"
    );
    assert!(
        !guided_body.contains("_open_intent("),
        "AC-GS7: run_guided must NOT call _open_intent directly (daemon opens, runner receipts)"
    );
}

#[test]
fn ac_gs8_goal_payload_has_consecutive_low_score() {
    let src = slurp("src/payloads.rs");
    assert!(
        src.contains("consecutive_low_score"),
        "AC-GS8: GoalPayload must have consecutive_low_score field for drift tracking"
    );
}

#[test]
fn ac_gs9_loop_engine_has_goal_revision_proposed() {
    let src = slurp("src/modules/loop_engine.rs");
    assert!(
        src.contains("goal_revision_proposed"),
        "AC-GS9: loop_engine.rs must emit goal_revision_proposed Reflexion on drift detection"
    );
}

#[test]
fn ac_gs10_drift_detection_bounded_resets_counter() {
    let src = slurp("src/modules/loop_engine.rs");
    // Counter must reset after firing to prevent infinite trigger loops
    assert!(
        src.contains("consecutive_low_score = 0"),
        "AC-GS10: drift detection must reset consecutive_low_score = 0 after firing (bounded exit)"
    );
    // ClarifyEngine MAX_CLARIFY_ROUNDS must still exist (no regression)
    let clarify_src = slurp("src/clarify_engine.rs");
    assert!(
        clarify_src.contains("MAX_CLARIFY_ROUNDS") || clarify_src.contains("max_rounds"),
        "AC-GS10: ClarifyEngine must still have bounded max rounds (no regression)"
    );
}
