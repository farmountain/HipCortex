/// v3.7.0 — Long-Lived Goal Completion
///
/// Closes 3 gaps identified v3.6.0:
/// 1. One-shot ≠ unattended afternoon: continuous guided runner + goal with ACs.
/// 2. Two-runner clarity: daemon recommends via scorecard; guided runner executes.
/// 3. Goal/AC progress: success_factors marked satisfied → goal.status = Succeeded.
///
/// AC-LR1: longrun soak creates goal before runner start (goal_id flows to runner)
/// AC-LR2: longrun soak does NOT use --one-shot (guided = continuous, not single-shot)
/// AC-LR3: hipcortex_runner.py has --guided argparse argument
/// AC-LR4: run_guided reads /substrate/scorecard before probing
/// AC-LR5: run_guided calls /goal/ endpoint (react step)
/// AC-LR6: loop_engine.rs contains score_success_factors_from_intents
/// AC-LR7: diary goal_status = "Succeeded"
/// AC-LR8: diary success_factors_satisfied = true
/// AC-LR9: diary react_iterations >= 2
/// AC-LR10: diary goal_lifecycle includes both "InProgress" and "Succeeded"

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
fn ac_lr1_soak_creates_goal_before_runner() {
    let src = slurp("scripts/longrun_soak_scenario.py");
    assert!(
        src.contains("create_goal(") || src.contains("goal_id ="),
        "AC-LR1: longrun soak must create goal (call create_goal) before starting runner"
    );
    // goal_id must appear before the runner Popen call
    let goal_pos = src.find("create_goal(").unwrap_or(usize::MAX);
    let runner_pos = src.find("Popen(").unwrap_or(usize::MAX);
    assert!(
        goal_pos < runner_pos,
        "AC-LR1: create_goal() must appear before Popen() in soak script"
    );
}

#[test]
fn ac_lr2_soak_not_one_shot() {
    let src = slurp("scripts/longrun_soak_scenario.py");
    assert!(
        !src.contains("--one-shot"),
        "AC-LR2: longrun soak must NOT pass --one-shot to runner (guided = continuous)"
    );
}

#[test]
fn ac_lr3_runner_has_guided_mode() {
    assert!(
        Path::new("scripts/hipcortex_runner.py").exists(),
        "AC-LR3: scripts/hipcortex_runner.py must exist"
    );
    let src = slurp("scripts/hipcortex_runner.py");
    assert!(
        src.contains("--guided"),
        "AC-LR3: hipcortex_runner.py must have --guided argparse argument"
    );
}

#[test]
fn ac_lr4_guided_reads_scorecard() {
    let src = slurp("scripts/hipcortex_runner.py");
    assert!(
        src.contains("scorecard") || src.contains("/substrate/scorecard"),
        "AC-LR4: run_guided must read /substrate/scorecard before probing"
    );
    assert!(
        src.contains("run_guided"),
        "AC-LR4: run_guided function must exist in runner"
    );
}

#[test]
fn ac_lr5_guided_calls_react_endpoint() {
    let src = slurp("scripts/hipcortex_runner.py");
    assert!(
        src.contains("/goal/") || src.contains("react_step"),
        "AC-LR5: run_guided must call /goal/:id/react endpoint"
    );
}

#[test]
fn ac_lr6_loop_engine_has_factor_scorer() {
    let src = slurp("src/modules/loop_engine.rs");
    assert!(
        src.contains("score_success_factors_from_intents"),
        "AC-LR6: loop_engine.rs must contain score_success_factors_from_intents"
    );
}

#[test]
fn ac_lr7_diary_goal_status_succeeded() {
    let diary = read_diary();
    assert!(
        !diary.is_null(),
        "AC-LR7: docs/longrun_soak_example.json must exist and be valid JSON"
    );
    assert_eq!(
        diary["goal_status"].as_str().unwrap_or(""),
        "Succeeded",
        "AC-LR7: diary.goal_status must be 'Succeeded'"
    );
}

#[test]
fn ac_lr8_diary_success_factors_satisfied() {
    let diary = read_diary();
    assert!(
        diary["success_factors_satisfied"].as_bool().unwrap_or(false),
        "AC-LR8: diary.success_factors_satisfied must be true"
    );
}

#[test]
fn ac_lr9_diary_react_iterations_gte_2() {
    let diary = read_diary();
    let iters = diary["react_iterations"].as_u64().unwrap_or(0);
    assert!(
        iters >= 2,
        "AC-LR9: diary.react_iterations must be >= 2, got {}",
        iters
    );
}

#[test]
fn ac_lr10_diary_lifecycle_includes_inprogress_and_succeeded() {
    let diary = read_diary();
    let lifecycle = diary["goal_lifecycle"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let statuses: Vec<&str> = lifecycle
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        statuses.contains(&"InProgress"),
        "AC-LR10: goal_lifecycle must include 'InProgress', got {:?}",
        statuses
    );
    assert!(
        statuses.contains(&"Succeeded"),
        "AC-LR10: goal_lifecycle must include 'Succeeded', got {:?}",
        statuses
    );
}
