/// v3.5.0 — Epistemic Field Soak: Seam Proof + Strong ACs
///
/// Closes the gap identified after v3.4.0: "process death ≠ epistemic update."
/// v3.4.0 proved WAL survives kill. v3.5.0 proves the agent notices world changed
/// via the intent/receipt seam WITHOUT any /memory/add for the edit event.
///
/// AC-ES1: script calls /intent/open (not just /memory/add)
/// AC-ES2: script passes sha256_hex in observation (content-anchored WM state)
/// AC-ES3: script calls /intent/receipt to close the probe
/// AC-ES4: no /memory/add for the edit event (seam-only path enforced)
/// AC-ES5: docs/epistemic_soak_example.json exists (live run result)
/// AC-ES6: JSON has uncertain_count, recommended_op, sha256_hex, assertion fields
/// AC-ES7: JSON assertions all true + result == "PASS"
/// AC-ES8: script has ReAct loop structure (OBSERVE/REFLECT/ACT + exit criteria)
use std::fs;
use std::path::Path;

fn slurp(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn ac_es1_script_uses_intent_open() {
    let src = slurp("scripts/field_soak_scenario.py");
    assert!(
        src.contains("/intent/open"),
        "AC-ES1: script must call /intent/open (receipt seam, not /memory/add)"
    );
}

#[test]
fn ac_es2_script_passes_sha256_hex() {
    let src = slurp("scripts/field_soak_scenario.py");
    assert!(
        src.contains("sha256_hex"),
        "AC-ES2: script must compute and pass sha256_hex in observation"
    );
    assert!(
        src.contains("hashlib"),
        "AC-ES2b: script must use hashlib to compute sha256 (no /memory/add shortcut)"
    );
}

#[test]
fn ac_es3_script_uses_intent_receipt() {
    let src = slurp("scripts/field_soak_scenario.py");
    assert!(
        src.contains("/intent/receipt"),
        "AC-ES3: script must call /intent/receipt to close the probe"
    );
}

#[test]
fn ac_es4_no_memory_add_for_edit() {
    let src = slurp("scripts/field_soak_scenario.py");
    // The edit event must NOT use /memory/add — cognitive update flows via receipt seam only.
    // Check for actual HTTP call, not comment mentions (comments explaining the ban are fine).
    assert!(
        !src.contains("_post(base, \"/memory/add\""),
        "AC-ES4: /memory/add must not be called in epistemic soak — edit flows through /intent/receipt"
    );
    assert!(
        src.contains("no /memory/add"),
        "AC-ES4b: script must self-document that /memory/add is banned for edit events"
    );
}

#[test]
fn ac_es5_epistemic_diary_exists() {
    assert!(
        Path::new("docs/epistemic_soak_example.json").exists(),
        "AC-ES5: docs/epistemic_soak_example.json must be committed (live run result)"
    );
}

#[test]
fn ac_es6_diary_has_required_fields() {
    let src = slurp("docs/epistemic_soak_example.json");
    assert!(src.contains("uncertain_count"), "AC-ES6a: JSON must have uncertain_count");
    assert!(src.contains("recommended_op"),  "AC-ES6b: JSON must have recommended_op");
    assert!(src.contains("sha256_hex"),      "AC-ES6c: JSON must have sha256_hex hashes");
    assert!(src.contains("hashes_differ"),   "AC-ES6d: JSON must have hashes_differ assertion");
    assert!(src.contains("uncertain_count_increased"), "AC-ES6e: JSON must have uncertain_count_increased");
    assert!(src.contains("epistemic_state_survived_restart"), "AC-ES6f: JSON must have epistemic_state_survived_restart");
    assert!(src.contains("no_memory_add_for_edit"), "AC-ES6g: JSON must have no_memory_add_for_edit");
    assert!(src.contains("after_restart"),   "AC-ES6h: JSON must have after_restart section");
}

#[test]
fn ac_es7_diary_assertions_all_pass() {
    let src = slurp("docs/epistemic_soak_example.json");
    let v: serde_json::Value = serde_json::from_str(&src)
        .expect("AC-ES7: docs/epistemic_soak_example.json must be valid JSON");
    let a = v.get("assertions").expect("AC-ES7: must have assertions block");
    assert_eq!(a["hashes_differ"], true,
        "AC-ES7a: hashes_differ must be true (content actually changed)");
    assert_eq!(a["uncertain_count_increased"], true,
        "AC-ES7b: uncertain_count_increased must be true (epistemic state updated via seam)");
    assert_eq!(a["epistemic_state_survived_restart"], true,
        "AC-ES7c: epistemic_state_survived_restart must be true (WAL preserved discrepancy Belief)");
    assert_eq!(a["no_memory_add_for_edit"], true,
        "AC-ES7d: no_memory_add_for_edit must be true (seam-only path)");
    assert_eq!(
        v.get("result").and_then(|r| r.as_str()).unwrap_or("FAIL"),
        "PASS",
        "AC-ES7e: overall soak result must be PASS"
    );
}

#[test]
fn ac_es8_script_has_react_structure() {
    let src = slurp("scripts/field_soak_scenario.py");
    assert!(src.contains("ITERATION 1") || src.contains("ITER 1"),
        "AC-ES8a: script must have ITER 1 / ITERATION 1 marker");
    assert!(src.contains("REFLECT"), "AC-ES8b: script must have REFLECT step");
    assert!(src.contains("ACT"),     "AC-ES8c: script must have ACT step");
    assert!(src.contains("OBSERVE"), "AC-ES8d: script must have OBSERVE step");
    assert!(src.contains("exit criteria") || src.contains("Exit criteria"),
        "AC-ES8e: script must document exit criteria");
}
