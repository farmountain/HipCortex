/// v3.4.0 — Field Soak + Per-Actor Wall Discipline
///
/// AC-FS1: scripts/field_soak_scenario.py exists
/// AC-FS2: script uses subprocess (real two-process proof, not in-memory)
/// AC-FS3: script captures after_restart state in diary
/// AC-FS4: docs/field_soak_example.json exists (sample or live run output)
/// AC-WD1: server.py uses per-actor set _live_beliefs_seen_actors (not global bool)
/// AC-WD2: dispatch_tool checks actor not in _live_beliefs_seen_actors
use std::fs;
use std::path::Path;

#[test]
fn ac_fs1_field_soak_script_exists() {
    assert!(
        Path::new("scripts/field_soak_scenario.py").exists(),
        "scripts/field_soak_scenario.py must exist"
    );
}

#[test]
fn ac_fs2_field_soak_script_two_process() {
    let src = fs::read_to_string("scripts/field_soak_scenario.py").unwrap();
    assert!(
        src.contains("subprocess"),
        "script must use subprocess to start real server process"
    );
    assert!(
        src.contains("start_server"),
        "script must have --start-server flag"
    );
    assert!(
        src.contains("_start_server"),
        "script must define _start_server helper"
    );
    assert!(
        src.contains("/memory/add"),
        "script must POST to /memory/add (correct REST endpoint)"
    );
    assert!(
        src.contains("record_type"),
        "script must use record_type field (not memory_type)"
    );
}

#[test]
fn ac_fs3_field_soak_captures_restart() {
    let src = fs::read_to_string("scripts/field_soak_scenario.py").unwrap();
    assert!(
        src.contains("after_restart"),
        "diary must capture post-restart scorecard state"
    );
    assert!(
        src.contains("records_survive_restart"),
        "diary assertions must include records_survive_restart"
    );
}

#[test]
fn ac_fs4_sample_diary_exists() {
    assert!(
        Path::new("docs/field_soak_example.json").exists(),
        "docs/field_soak_example.json must exist (sample or live run output)"
    );
    let content = fs::read_to_string("docs/field_soak_example.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&content)
        .expect("docs/field_soak_example.json must be valid JSON");
    assert_eq!(v["actor"], "soak-1", "diary actor must be soak-1");
    assert!(v.get("after_restart").is_some(), "diary must have after_restart key");
    assert!(v.get("assertions").is_some(), "diary must have assertions key");
}

#[test]
fn ac_wd1_per_actor_discipline_set() {
    let src = fs::read_to_string("sdk/mcp/server.py").unwrap();
    assert!(
        src.contains("_live_beliefs_seen_actors"),
        "server.py must use per-actor set _live_beliefs_seen_actors"
    );
    assert!(
        !src.contains("_live_beliefs_seen = False"),
        "old global bool _live_beliefs_seen must be removed"
    );
}

#[test]
fn ac_wd2_per_actor_discipline_check() {
    let src = fs::read_to_string("sdk/mcp/server.py").unwrap();
    assert!(
        src.contains("actor not in _live_beliefs_seen_actors"),
        "dispatch_tool must check per-actor discipline with actor not in _live_beliefs_seen_actors"
    );
    assert!(
        src.contains("_live_beliefs_seen_actors.add(actor)"),
        "dispatch_tool must register actor on get_live_beliefs/reflect"
    );
}
