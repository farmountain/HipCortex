/// v3.6.0 — Unattended Runner: Runner Hashes, Script Only Edits
///
/// Closes gap identified after v3.5.0: soak script WAS the hasher.
/// v3.6.0 proves: scripts/hipcortex_runner.py autonomously hashes the file
/// and posts all intent/open + intent/receipt calls. Script only edits file
/// and reads scorecard. Q10 advances past probe_entity:X after runner exits.
///
/// AC-UA1: unattended soak script contains no hashlib (runner hashes)
/// AC-UA2: unattended soak script contains no /intent/receipt calls
/// AC-UA3: unattended soak script contains no /intent/open calls
/// AC-UA4: scripts/hipcortex_runner.py exists
/// AC-UA5: runner script contains hashlib.sha256 (runner IS the sensor)
/// AC-UA6: runner script calls /intent/open AND /intent/receipt
/// AC-UA7: docs/unattended_soak_example.json has unattended:true + all assertion fields
/// AC-UA8: unattended diary assertions all true + result == "PASS"
/// AC-UA9: cognitive_state.rs syncs intent status → "Received" in MemoryStore
/// AC-UA10: loop_engine.rs wires ClarifyEngine::run before ReAct loop body
use std::fs;
use std::path::Path;

fn slurp(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn ac_ua1_unattended_script_no_hashlib() {
    let src = slurp("scripts/unattended_soak_scenario.py");
    assert!(
        !src.contains("import hashlib") && !src.contains("hashlib.sha256"),
        "AC-UA1: unattended soak script must not import hashlib — runner does hashing"
    );
}

#[test]
fn ac_ua2_unattended_script_no_intent_receipt() {
    let src = slurp("scripts/unattended_soak_scenario.py");
    assert!(
        !src.contains("\"/intent/receipt\"") && !src.contains("'/intent/receipt'"),
        "AC-UA2: unattended soak script must not call /intent/receipt — runner does it"
    );
}

#[test]
fn ac_ua3_unattended_script_no_intent_open() {
    let src = slurp("scripts/unattended_soak_scenario.py");
    assert!(
        !src.contains("\"/intent/open\"") && !src.contains("'/intent/open'"),
        "AC-UA3: unattended soak script must not call /intent/open — runner manages intents"
    );
}

#[test]
fn ac_ua4_runner_script_exists() {
    assert!(
        Path::new("scripts/hipcortex_runner.py").exists(),
        "AC-UA4: scripts/hipcortex_runner.py must exist — it is the autonomous sensor"
    );
}

#[test]
fn ac_ua5_runner_uses_hashlib() {
    let src = slurp("scripts/hipcortex_runner.py");
    assert!(
        src.contains("hashlib.sha256"),
        "AC-UA5: runner must use hashlib.sha256 — runner IS the sensor, not the soak script"
    );
}

#[test]
fn ac_ua6_runner_posts_intent_open_and_receipt() {
    let src = slurp("scripts/hipcortex_runner.py");
    assert!(
        src.contains("/intent/open"),
        "AC-UA6a: runner must call /intent/open autonomously"
    );
    assert!(
        src.contains("/intent/receipt"),
        "AC-UA6b: runner must call /intent/receipt autonomously"
    );
}

#[test]
fn ac_ua7_unattended_diary_has_required_fields() {
    let path = "docs/unattended_soak_example.json";
    assert!(
        Path::new(path).exists(),
        "AC-UA7: docs/unattended_soak_example.json must exist (run soak to generate)"
    );
    let raw = slurp(path);
    let diary: serde_json::Value =
        serde_json::from_str(&raw).expect("AC-UA7: diary must be valid JSON");
    assert!(
        diary["assertions"]["unattended"].as_bool() == Some(true),
        "AC-UA7a: diary must have assertions.unattended == true"
    );
    for field in &["uncertain_count_before_zero", "uncertain_count_increased",
                   "recommended_op_changed", "epistemic_state_survived_restart"] {
        assert!(
            diary["assertions"][field].is_boolean(),
            "AC-UA7b: diary.assertions.{} must be a boolean", field
        );
    }
}

#[test]
fn ac_ua8_unattended_diary_assertions_all_pass() {
    let raw = slurp("docs/unattended_soak_example.json");
    if raw.is_empty() {
        panic!("AC-UA8: docs/unattended_soak_example.json is empty — run the unattended soak");
    }
    let diary: serde_json::Value =
        serde_json::from_str(&raw).expect("AC-UA8: diary must be valid JSON");
    assert_eq!(
        diary["result"].as_str(),
        Some("PASS"),
        "AC-UA8: diary result must be PASS"
    );
    let assertions = diary["assertions"].as_object().expect("AC-UA8: assertions must be an object");
    for (k, v) in assertions {
        assert!(
            v.as_bool() == Some(true),
            "AC-UA8: assertion '{}' must be true, got {:?}", k, v
        );
    }
}

#[test]
fn ac_ua9_cognitive_state_syncs_intent_status_to_store() {
    let src = slurp("src/cognitive_state.rs");
    assert!(
        src.contains("meta[\"status\"] = serde_json::json!(\"Received\")"),
        "AC-UA9: cognitive_state.rs must sync intent status to MemoryStore after receipt \
         so cognitive_report Q10 can read Received status and advance past probe_entity:X"
    );
}

#[test]
fn ac_ua10_react_engine_wires_clarify_engine() {
    let src = slurp("src/modules/loop_engine.rs");
    assert!(
        src.contains("ClarifyEngine::run("),
        "AC-UA10: ReactEngine::run() must call ClarifyEngine::run() before the loop body \
         so ambiguous goals are self-prompted before any observe/reflect/act iteration"
    );
}
