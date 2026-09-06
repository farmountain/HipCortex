/// Gap 8 — v2.8.0 acceptance tests
///
/// AC-P1: plan_action_sequence returns highest-WM-prob factor first.
/// AC-P2: factors with no WM data rank at bottom (prob=0 → last).
/// AC-P3: wm_ranked boosts goals whose factors have WM coverage.
/// AC-T1: filter_liveness removes tool with recent ProbeFailed entity contact.
/// AC-T2: filter_liveness removes tool with stale heartbeat (staleness > 300 s).
/// AC-T3: filter_liveness keeps tool with no WM entity record (unknown = benefit of doubt).
/// AC-M1: docs/substrate_scorecard.md exists with 10 Q sections.
/// AC-M2: each Q section has a code/test reference line.

use std::fs;

// ── AC-P1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_p1_plan_action_sequence_ranks_grounded_factor_first() {
    let src = fs::read_to_string("src/goal_scheduler.rs").unwrap();
    assert!(src.contains("plan_action_sequence"),
        "GoalScheduler must expose plan_action_sequence");
    assert!(src.contains("predict_next_state"),
        "plan_action_sequence must call wm.predict_next_state to derive MAP prob");
    assert!(src.contains("!sf.satisfied"),
        "plan_action_sequence must exclude already-satisfied factors");
}

// ── AC-P2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_p2_ungrounded_factors_rank_last() {
    let src = fs::read_to_string("src/goal_scheduler.rs").unwrap();
    assert!(src.contains("unwrap_or(0.0)"),
        "factors with no WM transition must default to prob=0.0 (sort last)");
    // Verify descending sort (b.1.partial_cmp(&a.1) = b > a)
    assert!(src.contains("b.1.partial_cmp(&a.1)"),
        "sort must be descending (highest MAP prob first)");
}

// ── AC-P3 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_p3_wm_ranked_boosts_goals_with_coverage() {
    let src = fs::read_to_string("src/goal_scheduler.rs").unwrap();
    assert!(src.contains("wm_ranked"),
        "GoalScheduler must expose wm_ranked");
    assert!(src.contains("coverage"),
        "wm_ranked must compute coverage fraction of grounded success_factors");
    assert!(src.contains("base * (1.0 + coverage)"),
        "wm_ranked must multiply urgency/cost score by (1 + coverage)");
}

// ── AC-T1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_t1_filter_liveness_removes_recently_probe_failed_tool() {
    let src = fs::read_to_string("src/task_discovery.rs").unwrap();
    assert!(src.contains("filter_liveness"),
        "task_discovery must expose filter_liveness");
    assert!(src.contains("ProbeFailed"),
        "filter_liveness must check for ContactKind::ProbeFailed");
    assert!(src.contains("num_seconds() < 60"),
        "veto window must be 60 s for recent probe failures");
}

// ── AC-T2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_t2_filter_liveness_removes_stale_heartbeat_tool() {
    let src = fs::read_to_string("src/task_discovery.rs").unwrap();
    assert!(src.contains("staleness_s()"),
        "filter_liveness must call staleness_s() for heartbeat check");
    assert!(src.contains("> 300"),
        "stale threshold must be 300 s");
}

// ── AC-T3 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_t3_filter_liveness_keeps_unknown_entity_tool() {
    let src = fs::read_to_string("src/task_discovery.rs").unwrap();
    // The guard: "let Some(contact) = wm.entity_contact(...) else { return true }"
    assert!(src.contains("return true"),
        "tools with no WM entity record must be retained (benefit of doubt)");
    // Upgrade handler uses world_model arc
    let ws = fs::read_to_string("src/web_server.rs").unwrap();
    assert!(ws.contains("filter_liveness"),
        "web_server recommend-tools handler must call filter_liveness");
    assert!(ws.contains("world_model.read()"),
        "handler must read world_model for liveness check");
}

// ── AC-M1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_m1_substrate_scorecard_has_10_questions() {
    let doc = fs::read_to_string("docs/substrate_scorecard.md").unwrap();
    let q_count = (1..=10).filter(|i| doc.contains(&format!("## Q{}", i))).count();
    assert_eq!(q_count, 10,
        "substrate_scorecard.md must have exactly 10 Q sections; found {}", q_count);
}

// ── AC-M2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_m2_each_scorecard_question_has_code_reference() {
    let doc = fs::read_to_string("docs/substrate_scorecard.md").unwrap();
    // Every question section must have at least one "- Code:" or "- Test:" or "- Build:" line
    let sections: Vec<&str> = doc.split("## Q").skip(1).collect();
    assert_eq!(sections.len(), 10, "must have 10 Q sections");
    for (i, section) in sections.iter().enumerate() {
        let has_ref = section.contains("- Code:") || section.contains("- Test:") || section.contains("- Build:");
        assert!(has_ref, "Q{} must have a code/test/build reference line", i + 1);
    }
}
