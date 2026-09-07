/// v3.3.0 acceptance tests — Honest Claims: Wall Guard + Two-Process Diary + Probe Audit
///
/// AC-W1: server.py has WALL_TOKEN_BUDGET + _wall_exceeded_actors
/// AC-W2: handle_get_budget output has wall_status + wall_token_budget line
/// AC-W3: honest disclaimer about host context in get_budget output
/// AC-W4: wall_exceeded Reflexion write path exists in handle_get_live_beliefs
/// AC-D1: field_soak_diary_sit.rs exists
/// AC-D2: diary SIT runs 30 cycles (contains literal 30)
/// AC-D3: diary SIT registered in integration/mod.rs
/// AC-PA1: test_probe_honesty_runtime.py exists
/// AC-PA2: Python test covers multiple unknown sensor variants
/// AC-PA3: runner.py else-branch has early return before ok=True

use std::fs;

// ── AC-W1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_w1_wall_guard_constants_in_server_py() {
    let src = fs::read_to_string("sdk/mcp/server.py").unwrap();
    assert!(src.contains("WALL_TOKEN_BUDGET"), "WALL_TOKEN_BUDGET must be defined");
    assert!(src.contains("_wall_exceeded_actors"), "per-session dedup set must exist");
    assert!(
        src.contains("WALL_TOKEN_BUDGET: int = int(os.getenv"),
        "WALL_TOKEN_BUDGET must be env-configurable"
    );
}

// ── AC-W2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_w2_get_budget_reports_wall_status() {
    let src = fs::read_to_string("sdk/mcp/server.py").unwrap();
    assert!(
        src.contains("Wall status:"),
        "handle_get_budget must emit Wall status line"
    );
    assert!(
        src.contains("Wall token budget:"),
        "handle_get_budget must emit Wall token budget line"
    );
    assert!(
        src.contains("wall_status = \"bounded\"")
            || src.contains("wall_status=\"bounded\""),
        "wall_status must default to bounded"
    );
    assert!(
        src.contains("wall_status = \"exceeded\""),
        "wall_status must have exceeded branch"
    );
}

// ── AC-W3 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_w3_honest_disclaimer_in_get_budget() {
    let src = fs::read_to_string("sdk/mcp/server.py").unwrap();
    assert!(
        src.contains("[honest] wall_status measures substrate output only"),
        "get_budget must disclaim it only measures substrate output"
    );
    assert!(
        src.contains("[honest] Host context") && src.contains("NOT measured"),
        "get_budget must disclaim host context (conversation, tool dumps) is not measured"
    );
    assert!(
        src.contains("[honest] Substrate discipline"),
        "get_budget must describe the protocol that makes the wall meaningful"
    );
}

// ── AC-W4 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_w4_wall_exceeded_reflexion_write_path() {
    let src = fs::read_to_string("sdk/mcp/server.py").unwrap();
    assert!(
        src.contains("wall_exceeded"),
        "wall_exceeded Reflexion action must appear in server.py"
    );
    assert!(
        src.contains("_wall_exceeded_actors.add(actor)"),
        "dedup add must prevent repeated Reflexion writes per actor"
    );
    assert!(
        src.contains("\"action\": \"wall_exceeded\"")
            || src.contains("action\": \"wall_exceeded\"")
            || src.contains("\"action\": \"wall_exceeded\""),
        "Reflexion must use action=wall_exceeded"
    );
}

// ── AC-D1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_d1_field_soak_diary_sit_exists() {
    assert!(
        std::path::Path::new("tests/integration/field_soak_diary_sit.rs").exists(),
        "field_soak_diary_sit.rs must exist"
    );
}

// ── AC-D2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_d2_diary_sit_runs_30_cycles() {
    let src = fs::read_to_string("tests/integration/field_soak_diary_sit.rs").unwrap();
    assert!(
        src.contains("1..=30"),
        "diary SIT must loop for 30 day cycles (1..=30)"
    );
    assert!(
        src.contains("DayStat"),
        "diary SIT must use DayStat struct to record per-day metrics"
    );
    assert!(
        src.contains("process restart") || src.contains("Process A") || src.contains("Process B"),
        "diary SIT must document two-process simulation"
    );
}

// ── AC-D3 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_d3_diary_sit_registered_in_mod() {
    let src = fs::read_to_string("tests/integration/mod.rs").unwrap();
    assert!(
        src.contains("mod field_soak_diary_sit"),
        "field_soak_diary_sit must be registered in integration/mod.rs"
    );
}

// ── AC-PA1 ────────────────────────────────────────────────────────────────────
#[test]
fn ac_pa1_probe_honesty_runtime_test_exists() {
    assert!(
        std::path::Path::new("sdk/python/tests/test_probe_honesty_runtime.py").exists(),
        "test_probe_honesty_runtime.py must exist in sdk/python/tests/"
    );
}

// ── AC-PA2 ────────────────────────────────────────────────────────────────────
#[test]
fn ac_pa2_probe_test_covers_multiple_unknown_variants() {
    let src = fs::read_to_string("sdk/python/tests/test_probe_honesty_runtime.py").unwrap();
    assert!(src.contains("xyz://"), "must test opaque-URI sensor");
    assert!(src.contains("empty") || src.contains("\"\""), "must test empty sensor_path");
    assert!(src.contains("default"), "must test explicit 'default' sensor");
    assert!(src.contains("ftp://"), "must test unsupported scheme like ftp://");
    assert!(
        src.contains("does_not_raise") || src.contains("not raise"),
        "must verify unknown sensor never raises"
    );
}

// ── AC-PA3 ────────────────────────────────────────────────────────────────────
#[test]
fn ac_pa3_runner_else_branch_is_early_return() {
    let src = fs::read_to_string("sdk/python/hipcortex/runner.py").unwrap();
    // Verify the else branch returns before ok=True
    let idx_else = src.find("unknown_sensor").expect("unknown_sensor in runner.py");
    let idx_ok_true = src.find("ok = True").expect("ok = True in runner.py");
    assert!(
        idx_else < idx_ok_true,
        "unknown_sensor early return (idx={}) must come before ok=True (idx={})",
        idx_else, idx_ok_true
    );
    // Verify the early return carries reachable=False
    assert!(
        src.contains("\"reachable\": False") || src.contains("reachable\": False"),
        "else-branch must set reachable: False"
    );
    // Verify ok=False is set in the else branch
    assert!(
        src.contains("\"ok\": False"),
        "else-branch must set ok: False"
    );
}
