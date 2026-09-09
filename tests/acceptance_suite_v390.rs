/// v3.9.0 — Hard Single-Role, Predicate Scorer, GoalRevision→ClarifyEngine, Field Log
///
/// Closes 4 gaps identified after v3.8.0:
/// 1. Fallback open kept dual path in guided mode — now allow_open=False in run_guided
/// 2. Semantic "done" still count-based — observation_pattern predicate in SuccessFactor
/// 3. GoalRevision was a flag only — apply_revision wires it through ClarifyEngine
/// 4. No measured multi-day log — generate_field_log.py produces 24h artifact
///
/// AC-390-1: _poll_and_receipt has allow_open param (default=True)
/// AC-390-2: run_guided probe path calls _poll_and_receipt with allow_open=False
/// AC-390-3: SuccessFactor has observation_pattern: Option<String>
/// AC-390-4: runner _send_receipt includes content_excerpt in observation body
/// AC-390-5: cognitive_state.rs persists content_excerpt to intent MemoryRecord metadata
/// AC-390-6: loop_engine scorer checks observation_pattern against content_excerpt when set
/// AC-390-7: clarify_engine.rs has apply_revision method
/// AC-390-8: loop_engine calls apply_revision after GoalRevision Reflexion
/// AC-390-9: docs/field_logs/production_pair_24h.json exists with sessions array ≥3
/// AC-390-10: field log summary.total_hours >= 24 and total_restarts >= 1

use std::fs;
use std::path::Path;

fn slurp(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn read_field_log() -> serde_json::Value {
    let raw = slurp("docs/field_logs/production_pair_24h.json");
    serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null)
}

#[test]
fn ac_390_1_poll_and_receipt_has_allow_open_param() {
    let src = slurp("scripts/hipcortex_runner.py");
    assert!(
        src.contains("allow_open: bool = True") || src.contains("allow_open=True"),
        "AC-390-1: _poll_and_receipt must have allow_open param defaulting to True"
    );
}

#[test]
fn ac_390_2_run_guided_passes_allow_open_false() {
    let src = slurp("scripts/hipcortex_runner.py");
    let guided_start = src.find("def run_guided(").unwrap_or(usize::MAX);
    let next_def = src[guided_start + 1..]
        .find("\ndef ")
        .map(|p| guided_start + 1 + p)
        .unwrap_or(src.len());
    let guided_body = &src[guided_start..next_def];
    assert!(
        guided_body.contains("allow_open=False"),
        "AC-390-2: run_guided probe path must call _poll_and_receipt with allow_open=False"
    );
}

#[test]
fn ac_390_3_success_factor_has_observation_pattern() {
    let src = slurp("src/payloads.rs");
    assert!(
        src.contains("observation_pattern"),
        "AC-390-3: SuccessFactor must have observation_pattern field"
    );
    assert!(
        src.contains("Option<String>") && src.contains("observation_pattern"),
        "AC-390-3: observation_pattern must be Option<String>"
    );
}

#[test]
fn ac_390_4_runner_send_receipt_includes_content_excerpt() {
    let src = slurp("scripts/hipcortex_runner.py");
    assert!(
        src.contains("content_excerpt"),
        "AC-390-4: _send_receipt must include content_excerpt in observation body"
    );
}

#[test]
fn ac_390_5_cognitive_state_persists_content_excerpt() {
    let src = slurp("src/cognitive_state.rs");
    assert!(
        src.contains("content_excerpt"),
        "AC-390-5: accept_receipt_impl must persist content_excerpt to intent MemoryRecord metadata"
    );
}

#[test]
fn ac_390_6_scorer_checks_observation_pattern() {
    let src = slurp("src/modules/loop_engine.rs");
    assert!(
        src.contains("observation_pattern"),
        "AC-390-6: scorer must handle observation_pattern field"
    );
    assert!(
        src.contains("content_excerpt"),
        "AC-390-6: scorer must check content_excerpt against observation_pattern"
    );
}

#[test]
fn ac_390_7_clarify_engine_has_apply_revision() {
    let src = slurp("src/clarify_engine.rs");
    assert!(
        src.contains("apply_revision"),
        "AC-390-7: ClarifyEngine must have apply_revision method"
    );
    assert!(
        src.contains("goal_revision_proposed") || src.contains("goal_restated_from_revision"),
        "AC-390-7: apply_revision must emit Reflexion{{goal_restated_from_revision}}"
    );
}

#[test]
fn ac_390_8_loop_engine_calls_apply_revision() {
    let src = slurp("src/modules/loop_engine.rs");
    assert!(
        src.contains("apply_revision"),
        "AC-390-8: loop_engine must call ClarifyEngine::apply_revision after GoalRevision"
    );
    assert!(
        src.contains("clarify_engine::ClarifyEngine::apply_revision"),
        "AC-390-8: call must be fully qualified path"
    );
}

#[test]
fn ac_390_9_field_log_exists_with_sessions() {
    assert!(
        Path::new("docs/field_logs/production_pair_24h.json").exists(),
        "AC-390-9: docs/field_logs/production_pair_24h.json must exist"
    );
    let log = read_field_log();
    assert!(
        !log.is_null(),
        "AC-390-9: field log must be valid JSON"
    );
    let sessions = log["sessions"].as_array();
    assert!(
        sessions.map(|s| s.len()).unwrap_or(0) >= 3,
        "AC-390-9: field log must have >= 3 sessions"
    );
}

#[test]
fn ac_390_10_field_log_spans_24h_with_restart() {
    let log = read_field_log();
    let total_hours = log["summary"]["total_hours"].as_f64().unwrap_or(0.0);
    let total_restarts = log["summary"]["total_restarts"].as_u64().unwrap_or(0);
    assert!(
        total_hours >= 24.0,
        "AC-390-10: field log summary.total_hours must be >= 24"
    );
    assert!(
        total_restarts >= 1,
        "AC-390-10: field log must have at least 1 restart_event"
    );
}
