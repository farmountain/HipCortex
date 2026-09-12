//! Clarify ladder unit tests — WP10 / spec §3.1–§3.2, §3.5.
//!
//! The clarified question is *not* "does the ladder resolve anything" — that is covered by
//! `clarify_engine_tests`. It is **"can the ladder be made to stop, and can it be seen
//! stopping"** — the WP10 acceptance criteria:
//!
//! > Each tier fires at most once; budget exhaustion forces T3 and a terminal status; a goal
//! > with testable AC never reaches T3; a goal with prior art never reaches T3; the report
//! > shows the ladder.
//!
//! Each test below pins exactly one of those clauses against the persisted ledger
//! (`ClarifyEngine::ladder_rungs` / `ladder_cycles`), never against internal state — the
//! ledger *is* the guarantee, and reading it back is what proves the bound survives a
//! restart and cannot drift from the rungs it bounds.

use hipcortex::clarify_engine::{
    ClarifyEngine, ClarifyOutcome, ClarifySource, ClarifyTier, ClarifyTrigger, LadderRung,
    SubstrateBudget, MAX_CLARIFY_CYCLES_PER_GOAL, MAX_CLARIFY_TIERS,
};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, GoalPayload, GoalStatus, SuccessFactor};
use hipcortex::persistence::InMemoryBackend;
use hipcortex::world_model_enhanced::WorldModelEnhanced;

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn add_goal(
    store: &mut MemoryStore<InMemoryBackend>,
    actor: &str,
    target: &str,
    factors: Vec<SuccessFactor>,
    criteria: Vec<String>,
) -> uuid::Uuid {
    let p = GoalPayload {
        target_state: target.to_string(),
        success_factors: factors,
        acceptance_criteria: criteria,
        status: GoalStatus::Pending,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Goal,
        actor.to_string(),
        "set_goal".to_string(),
        target.to_string(),
        serde_json::to_value(&p).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

fn add_temporal(
    store: &mut MemoryStore<InMemoryBackend>,
    actor: &str,
    goal_id: uuid::Uuid,
    target: &str,
    step: u32,
) {
    let mut rec = MemoryRecord::new(
        MemoryType::Temporal,
        actor.to_string(),
        "observe".to_string(),
        target.to_string(),
        serde_json::json!({ "step": step }),
    );
    rec.derived_from = Some(goal_id);
    rec.react_iteration = Some(step);
    store.add(rec).unwrap();
}

/// A goal that declares how expensive it is to execute *wrongly*. The ask-cost gate reads
/// `estimated_cost` directly, so the only way to reach the decline branch is to declare a goal
/// cheap enough that spending a human's attention on it costs more than the mistake would.
fn add_goal_with_cost(
    store: &mut MemoryStore<InMemoryBackend>,
    actor: &str,
    target: &str,
    estimated_cost: f64,
) -> uuid::Uuid {
    let p = GoalPayload {
        target_state: target.to_string(),
        estimated_cost,
        status: GoalStatus::Pending,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Goal,
        actor.to_string(),
        "set_goal".to_string(),
        target.to_string(),
        serde_json::to_value(&p).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

fn rung_tiers(rungs: &[LadderRung]) -> Vec<String> {
    rungs.iter().map(|r| r.tier.clone()).collect()
}

// ── AC: each tier fires at most once ─────────────────────────────────────────

#[test]
fn ladder_each_tier_runs_at_most_once_across_repeated_invocations() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "ladder-a", "deploy_service", vec![], vec![]);

    // A budget that cannot run out: with the default 3.0 the *budget* gate would return before
    // the descent loop, and this test would then be proving the budget bound while claiming to
    // prove monotonicity — a green test with no teeth. With b0 = 100 all four invocations
    // actually walk the ladder, so the only thing stopping a second T0 rung is the recorder.
    let big = SubstrateBudget { b0: 100.0 };

    // Four invocations: two more than the ladder has rungs.
    let mut outcomes = Vec::new();
    for _ in 0..4 {
        outcomes.push(ClarifyEngine::run_with_budget(
            &mut store,
            gid,
            "ladder-a",
            ClarifyTrigger::EmptyAC,
            None,
            big,
        ));
    }

    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    for tier in [ClarifyTier::T0Environment, ClarifyTier::T1PriorArt, ClarifyTier::T2Causal] {
        let n = rungs.iter().filter(|r| r.tier == tier.as_str()).count();
        assert_eq!(n, 1, "tier {} must fire exactly once, found {n}: {:?}", tier.as_str(), rung_tiers(&rungs));
    }
    // T3 is terminal: it is recorded once (`asked`) and never appended again.
    assert_eq!(
        rungs.iter().filter(|r| r.tier == ClarifyTier::T3AskUser.as_str()).count(),
        1,
        "T3 must be recorded once, got {:?}",
        rung_tiers(&rungs)
    );
    assert_eq!(rungs.len(), 1 + MAX_CLARIFY_TIERS as usize, "ledger: {:?}", rung_tiers(&rungs));

    // Every invocation is a cycle — that is the counter guarantee 3 caps.
    assert_eq!(
        ClarifyEngine::ladder_cycles(&store, gid),
        4,
        "each invocation must be counted, so the lifetime bound can act"
    );
    assert!(
        outcomes.iter().all(|o| *o == ClarifyOutcome::NeedsUserClarification),
        "an unresolvable goal must end at the question, got {outcomes:?}"
    );
}

// ── AC: budget exhaustion forces T3 ──────────────────────────────────────────

#[test]
fn ladder_budget_exhaustion_forces_t3_without_searching() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "ladder-b", "deploy_service", vec![], vec![]);

    // `exhausted(spend)` is `spend >= b0`, so a zero budget is exhausted before the first
    // rung is attempted — the ladder must go straight to the human.
    let outcome = ClarifyEngine::run_with_budget(
        &mut store,
        gid,
        "ladder-b",
        ClarifyTrigger::EmptyAC,
        None,
        SubstrateBudget { b0: 0.0 },
    );

    assert_eq!(outcome, ClarifyOutcome::NeedsUserClarification);
    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    assert_eq!(
        rung_tiers(&rungs),
        vec![ClarifyTier::T3AskUser.as_str().to_string()],
        "no search rung may be attempted on an exhausted budget"
    );
    let t3 = &rungs[0];
    assert_eq!(t3.outcome, "asked");
    assert!(
        t3.evidence.iter().any(|e| e.contains("substrate_budget_exhausted")),
        "the budget decision must be recorded on the rung, got {:?}",
        t3.evidence
    );
}

// ── AC (guarantee 4): no-progress exit ───────────────────────────────────────

#[test]
fn ladder_no_progress_exit_records_no_duplicate_rungs() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "ladder-c", "deploy_service", vec![], vec![]);

    // A budget large enough that exhaustion cannot mask the no-progress path: with the
    // default 3.0 the spend (1.0 + 2.0 + 4.0) would exhaust first and this branch would
    // never be reached.
    let big = SubstrateBudget { b0: 100.0 };
    let _ = ClarifyEngine::run_with_budget(
        &mut store,
        gid,
        "ladder-c",
        ClarifyTrigger::EmptyAC,
        None,
        big,
    );
    let after_first = ClarifyEngine::ladder_rungs(&store, gid).len();

    let second = ClarifyEngine::run_with_budget(
        &mut store,
        gid,
        "ladder-c",
        ClarifyTrigger::EmptyAC,
        None,
        big,
    );

    assert_eq!(second, ClarifyOutcome::NeedsUserClarification);
    assert_eq!(
        ClarifyEngine::ladder_rungs(&store, gid).len(),
        after_first,
        "a descent that closes no rung must not rewrite the ledger — that would be repetition"
    );
    assert_eq!(ClarifyEngine::ladder_cycles(&store, gid), 2);
    // The rung is already `asked`, so the *outcome* cannot distinguish this exit from budget
    // exhaustion — the reason must. Otherwise "no progress" and "out of budget" would be the
    // same event as far as anyone reading the trace could tell.
    assert_eq!(
        ClarifyEngine::ladder_exit_reasons(&store, gid),
        vec!["no_progress".to_string()],
        "the exit must name itself"
    );
}

// ── AC (guarantee 3): lifetime budget is terminal ────────────────────────────

#[test]
fn ladder_lifetime_budget_exhaustion_is_terminal() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "ladder-d", "deploy_service", vec![], vec![]);

    let mut last = None;
    for _ in 0..(MAX_CLARIFY_CYCLES_PER_GOAL + 2) {
        last = Some(ClarifyEngine::run(
            &mut store,
            gid,
            "ladder-d",
            ClarifyTrigger::EmptyAC,
            None,
        ));
    }

    assert_eq!(
        last,
        Some(ClarifyOutcome::NeedsUserClarification),
        "past the lifetime bound the ladder must still return a decision, never spin"
    );
    assert_eq!(
        ClarifyEngine::ladder_cycles(&store, gid),
        MAX_CLARIFY_CYCLES_PER_GOAL + 2,
        "invocations must keep being counted so the caller can see exhaustion"
    );
    // The caller (ReactEngine) turns exhaustion into `GoalStatus::Abandoned`; this module's
    // contract is only that it stops asking for more work.
    assert_eq!(ClarifyEngine::ladder_rungs(&store, gid).len(), 4);
    // Without the lifetime check, cycle 4 would fall through to the no-progress branch and
    // return the same `NeedsUserClarification` with the same rung count — so the exits would be
    // indistinguishable from outside the engine, which is exactly how an unbounded ladder hides
    // inside a green test. The recorded reasons are what make each bound visible, and their
    // order is itself a claim: the per-goal substrate budget (three rungs cost 1+2+4, so the
    // default b0 = 3.0 is spent after cycle 1) bites strictly before the lifetime bound does.
    assert_eq!(
        ClarifyEngine::ladder_exit_reasons(&store, gid),
        vec![
            "substrate_budget_exhausted".to_string(),
            "lifetime_budget_exhausted".to_string(),
        ],
        "the two bounds must be attributed separately, and the budget must be the first to bite"
    );
}

// ── AC: a goal with testable AC never reaches T3 ─────────────────────────────

#[test]
fn ladder_testable_ac_never_reaches_t3() {
    let mut store = make_store();
    let factors = vec![SuccessFactor {
        name: "service_running".to_string(),
        weight: 1.0,
        satisfied: false,
        observation_pattern: Some("listening on 8080".to_string()),
    }];
    let gid = add_goal(
        &mut store,
        "ladder-e",
        "deploy_service",
        factors,
        vec!["the service answers on 8080".to_string()],
    );

    let outcome = ClarifyEngine::run(
        &mut store,
        gid,
        "ladder-e",
        ClarifyTrigger::UntestableAC,
        None,
    );

    assert_eq!(
        outcome,
        ClarifyOutcome::AlreadyClear,
        "a goal whose factors are all machine-checkable is not ambiguous"
    );
    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    assert_eq!(
        rung_tiers(&rungs),
        vec![ClarifyTier::T0Environment.as_str().to_string()],
        "it must short-circuit at the top of the ladder, got {:?}",
        rung_tiers(&rungs)
    );
    assert_eq!(rungs[0].outcome, "already_decidable");
    assert!(
        rungs.iter().all(|r| r.tier != ClarifyTier::T3AskUser.as_str()),
        "a decidable goal must never spend the user's attention"
    );
}

// ── AC: a goal with prior art never reaches T3 ──────────────────────────────

#[test]
fn ladder_prior_art_never_reaches_t3() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "ladder-f", "deploy_service", vec![], vec![]);

    let bp = BeliefPayload {
        proposition: "deploy_service is available on port 8080".to_string(),
        ..Default::default()
    };
    let mut belief = MemoryRecord::new(
        MemoryType::Belief,
        "ladder-f".to_string(),
        "observed".to_string(),
        "deploy_service".to_string(),
        serde_json::to_value(&bp).unwrap(),
    );
    belief.actor = "ladder-f".to_string();
    store.add(belief).unwrap();

    let outcome = ClarifyEngine::run(
        &mut store,
        gid,
        "ladder-f",
        ClarifyTrigger::EmptyAC,
        None,
    );

    match &outcome {
        ClarifyOutcome::ClarifiedBySubstrate { source, .. } => {
            assert_eq!(*source, ClarifySource::PriorArt, "T1 must own this resolution")
        }
        other => panic!("prior art must resolve before the question, got {other:?}"),
    }
    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    assert!(
        rungs.iter().all(|r| r.tier != ClarifyTier::T3AskUser.as_str()),
        "resolved in the substrate — the user must not be asked: {:?}",
        rung_tiers(&rungs)
    );
    assert_eq!(
        rung_tiers(&rungs),
        vec![
            ClarifyTier::T0Environment.as_str().to_string(),
            ClarifyTier::T1PriorArt.as_str().to_string(),
        ],
        "the ledger must show the descent that did the work"
    );
}

// ── T2 reachability: the rung can actually fire ──────────────────────────────

#[test]
fn ladder_t2_resolves_from_broken_structural_equation() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "ladder-g", "stabilise_robot", vec![], vec![]);

    // A world model with one equation-bearing node, and a goal history that mentions that
    // node often enough that its structural equation is the best explanation of the failure.
    //
    // The arithmetic is `LinearSE::invert_for_u(&[], observed) == observed`, `score = mean
    // |observed|`, and `confidence = score / (score + 1)`. Six mentions per step therefore
    // scores 6.0 and clears the 0.85 gate (which needs `score >= 5.667`).
    let wm = WorldModelEnhanced::new();
    wm.add_causal_node("actuator_torque".to_string()).unwrap();
    wm.rewrite_structural_equation("actuator_torque", vec![]).unwrap();

    for step in 0..3u32 {
        add_temporal(
            &mut store,
            "ladder-g",
            gid,
            "actuator_torque actuator_torque actuator_torque actuator_torque actuator_torque actuator_torque",
            step,
        );
    }

    let outcome = ClarifyEngine::run(
        &mut store,
        gid,
        "ladder-g",
        ClarifyTrigger::EmptyAC,
        Some(&wm),
    );

    match &outcome {
        ClarifyOutcome::ClarifiedBySubstrate { source, evidence } => {
            assert_eq!(*source, ClarifySource::Causal, "T2 must own this resolution; {evidence:?}");
            assert!(
                evidence.iter().any(|e| e.contains("broken_equation=actuator_torque")),
                "the blamed equation must be named: {evidence:?}"
            );
        }
        other => panic!("a broken equation must resolve at T2, got {other:?}"),
    }

    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    let t2 = rungs
        .iter()
        .find(|r| r.tier == ClarifyTier::T2Causal.as_str())
        .expect("T2 rung must be recorded");
    assert_eq!(t2.outcome, "resolved");
    assert!(
        rungs.iter().all(|r| r.tier != ClarifyTier::T3AskUser.as_str()),
        "the substrate explained it — asking the user would be waste: {:?}",
        rung_tiers(&rungs)
    );

    // The repair is offered to the caller as a `CognitiveDelta` hook, recorded for the trace.
    let repairs = store
        .all_by_type(MemoryType::Reflexion)
        .into_iter()
        .filter(|r| r.derived_from == Some(gid) && r.action == "clarify_causal_repair")
        .count();
    assert_eq!(repairs, 1, "exactly one causal-repair reflection per resolved T2 rung");
}

#[test]
fn ladder_t2_records_unavailable_without_a_world_model() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "ladder-h", "stabilise_robot", vec![], vec![]);

    let _ = ClarifyEngine::run(&mut store, gid, "ladder-h", ClarifyTrigger::EmptyAC, None);

    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    let t2 = rungs
        .iter()
        .find(|r| r.tier == ClarifyTier::T2Causal.as_str())
        .expect("T2 must be attempted-or-recorded, never skipped silently");
    // "unavailable" is deliberately distinct from "no_signal": an un-attempted rung means the
    // search was *incomplete*, which is why T3 is then asked with P = 0.5 rather than 1.0.
    assert_eq!(t2.outcome, "unavailable");
}

// ── AC: the report shows the ladder (§3.5) ───────────────────────────────────

#[test]
fn report_shows_the_clarify_ladder() {
    use hipcortex::cognitive_report::build_report;

    let mut store = make_store();
    let actor = "ladder-report";
    let gid = add_goal(&mut store, actor, "deploy_service", vec![], vec![]);
    let _ = ClarifyEngine::run(&mut store, gid, actor, ClarifyTrigger::EmptyAC, None);

    let report = build_report(&store, actor, 0.8);
    let ledger = ClarifyEngine::ladder_rungs(&store, gid);

    assert!(
        !report.clarify_ladder.is_empty(),
        "Q10 can recommend `clarify_goal`; the report must also say why — empty ladder hides it"
    );
    assert_eq!(
        rung_tiers(&report.clarify_ladder),
        rung_tiers(&ledger),
        "the report must be a projection of the ledger, not a second opinion about it"
    );
    assert!(
        report.clarify_ladder.iter().any(|r| r.outcome == "asked"),
        "a goal that reached the human must show that it did: {:?}",
        report.clarify_ladder
    );
}

#[test]
fn unscoped_report_has_no_ladder() {
    use hipcortex::cognitive_report::build_unscoped_report;

    let store = make_store();
    let report = build_unscoped_report(0.5);
    assert!(
        report.clarify_ladder.is_empty(),
        "an unscoped report reads no memory, so it cannot show a ladder"
    );
    let _ = store;
}

// ── AC: a goal with prior art never reaches T3 ───────────────────────────────
//
// The two tests below are a *pair*, and they exist because teeth-probing found the gate
// unproven. `ladder_prior_art_never_reaches_t3` resolves through T1's **Belief** rung, whose
// match is a plain `proposition.contains(target_state)` and which never consults
// `PRIOR_ART_OVERLAP_GATE` at all — so raising the gate to an unreachable 1.01 left that test
// green. The gate is load-bearing for exactly one sub-rung (the prior **Succeeded goal**), and
// only this pair makes it observable: one case that must pass the gate, one that must fail it.
//
// Both drive `ClarifyTrigger::PreSuccess`, not `EmptyAC`, and that is not incidental. An
// `EmptyAC` trigger on a goal that *already* carries factors is answered `AlreadyClear`
// immediately after the T0 rung's `no_signal` (see the preserved legacy guard in
// `run_with_budget`): the caller said "the AC is empty", the goal plainly has one, and the
// environment offers no reason to restate it — so there is nothing to clarify and the ladder
// stops before T1. Only a non-AC trigger descends into the prior-art search.

/// A prior goal that already succeeded — the only T1 source the overlap gate governs.
fn add_succeeded_goal(
    store: &mut MemoryStore<InMemoryBackend>,
    actor: &str,
    target: &str,
    factors: Vec<SuccessFactor>,
) -> uuid::Uuid {
    let p = GoalPayload {
        target_state: target.to_string(),
        success_factors: factors,
        acceptance_criteria: vec![],
        status: GoalStatus::Succeeded,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Goal,
        actor.to_string(),
        "set_goal".to_string(),
        target.to_string(),
        serde_json::to_value(&p).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

fn patterned(name: &str) -> SuccessFactor {
    SuccessFactor {
        name: name.to_string(),
        weight: 1.0,
        satisfied: true,
        observation_pattern: Some(format!("{name} ok")),
    }
}

#[test]
fn ladder_prior_art_gate_admits_a_succeeded_goal_that_covers_the_factors() {
    let mut store = make_store();
    let gid = add_goal(
        &mut store,
        "gate-a",
        "deploy_service_ready",
        vec![SuccessFactor {
            name: "deploy_service".to_string(),
            weight: 1.0,
            satisfied: false,
            observation_pattern: None,
        }],
        vec![],
    );
    // `factor_overlap` is |mine ∩ theirs| / |mine| — mine = {deploy, service},
    // theirs = {deploy, service, confirmed} ⇒ 2/2 = 1.0 ≥ PRIOR_ART_OVERLAP_GATE (0.5).
    add_succeeded_goal(&mut store, "gate-a", "checkout_deploy_ready", vec![patterned("deploy_service_confirmed")]);

    let outcome = ClarifyEngine::run_with_budget(
        &mut store,
        gid,
        "gate-a",
        ClarifyTrigger::PreSuccess,
        None,
        // As in the monotonicity test: the default 3.0 would return at the budget gate and
        // this test would prove the budget bound while claiming to prove the overlap gate.
        SubstrateBudget { b0: 100.0 },
    );

    match &outcome {
        ClarifyOutcome::ClarifiedBySubstrate { source, evidence } => {
            assert_eq!(*source, ClarifySource::PriorArt);
            assert!(
                evidence.iter().any(|e| e.starts_with("prior_art_goal=")),
                "the rung must name the goal it learned from: {evidence:?}"
            );
            assert!(
                evidence.iter().any(|e| e.starts_with("factor_overlap=")),
                "and must report the overlap it gated on: {evidence:?}"
            );
        }
        other => panic!("expected prior-art resolution, got {other:?}"),
    }

    // A rung that reports `resolved` while writing nothing leaves the caller with the same
    // empty AC it arrived with, so the adopted clause must actually land on the goal.
    let reloaded: GoalPayload =
        serde_json::from_value(store.find_by_id(gid).unwrap().metadata.clone()).unwrap();
    assert!(
        reloaded.success_factors.iter().any(|f| f.name == "deploy_service_confirmed"),
        "the adopted factor must be persisted: {:?}",
        reloaded.success_factors
    );
}

#[test]
fn ladder_prior_art_gate_rejects_a_succeeded_goal_with_unrelated_factors() {
    let mut store = make_store();
    let gid = add_goal(
        &mut store,
        "gate-b",
        "deploy_service_ready",
        vec![SuccessFactor {
            name: "deploy_service".to_string(),
            weight: 1.0,
            satisfied: false,
            observation_pattern: None,
        }],
        vec![],
    );
    // Overlap 0.0: a goal that succeeded at something unrelated is not prior art. Without the
    // gate this rung would adopt a clause from *any* Succeeded goal in the store — which is
    // how a substrate talks itself into believing it already knows the answer.
    add_succeeded_goal(&mut store, "gate-b", "widget_inventory_ready", vec![patterned("unrelated_widget")]);

    let outcome = ClarifyEngine::run_with_budget(
        &mut store,
        gid,
        "gate-b",
        ClarifyTrigger::PreSuccess,
        None,
        SubstrateBudget { b0: 100.0 },
    );

    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    let t1 = rungs
        .iter()
        .find(|r| r.tier == ClarifyTier::T1PriorArt.as_str())
        .unwrap_or_else(|| panic!("T1 must have been attempted: {:?}", rung_tiers(&rungs)));
    assert_eq!(
        t1.outcome,
        "no_signal",
        "the gate rejected a 0.0-overlap goal, so T1 found nothing: {:?}",
        rung_tiers(&rungs)
    );
    assert_eq!(
        outcome,
        ClarifyOutcome::NeedsUserClarification,
        "with no prior art and no world model, the ladder has nothing left but the human: {:?}",
        rung_tiers(&rungs)
    );
}

// ── AC: the ask-cost gate can decide *not* to spend a human's attention ───────
//
// Teeth-probing found this branch in no test at all: `declined`, `estimated_cost` and
// `COST_OF_ASKING` did not appear anywhere under `tests/`. Every existing goal carries the
// `GoalPayload::default()` cost, whose non-positive value is read through the
// `DEFAULT_GOAL_COST` fallback, so `p_unresolvable * 1.0 > 0.25` was always true and the
// ladder always asked. That made the gate look like a guarantee while being a constant.
// The pair below pins it in both directions: cheap goal ⇒ decline, absent cost ⇒ ask.

#[test]
fn ladder_ask_cost_gate_declines_when_the_goal_is_cheap() {
    let mut store = make_store();
    // 0.1: wrongly executing this costs less than the 0.25 of a human's attention it would
    // take to ask. p_unresolvable is 0.5 at best here (the T2 rung is unavailable with no
    // world model), so 0.5 * 0.1 = 0.05 does not clear the bar.
    let gid = add_goal_with_cost(&mut store, "cost-a", "tweak_log_level", 0.1);

    let outcome = ClarifyEngine::run_with_budget(
        &mut store,
        gid,
        "cost-a",
        ClarifyTrigger::EmptyAC,
        None,
        SubstrateBudget { b0: 100.0 },
    );

    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    assert_eq!(
        outcome,
        ClarifyOutcome::AlreadyClear,
        "a decline is not a clarification and not a dead end — the caller keeps its current \
         reading and the outer iteration bound still applies"
    );
    let t3 = rungs
        .iter()
        .find(|r| r.tier == ClarifyTier::T3AskUser.as_str())
        .unwrap_or_else(|| panic!("T3 must have been reached: {:?}", rung_tiers(&rungs)));
    assert_eq!(
        t3.outcome, "declined",
        "the terminal rung must record that it chose not to ask, not that it asked: {:?}",
        rungs
    );
    assert!(
        t3.evidence.iter().any(|e| e.starts_with("cost_of_asking=")),
        "a decline is a cost judgement, so the trace must carry the numbers it judged: {:?}",
        t3.evidence
    );
    let declined = store
        .all_by_type(MemoryType::Reflexion)
        .into_iter()
        .filter(|r| r.action == "clarify_ask_declined" && r.derived_from == Some(gid))
        .count();
    assert_eq!(declined, 1, "the decline must be visible in the trace as exactly one Reflexion");
    assert!(
        store
            .all_by_type(MemoryType::Belief)
            .into_iter()
            .all(|r| r.action != "clarify_needed"),
        "declining must not leave a `clarify_needed` Belief behind — that is what an ask writes"
    );
}

#[test]
fn ladder_absent_cost_falls_back_to_the_declared_default_and_still_asks() {
    let mut store = make_store();
    // `GoalPayload::default()` leaves `estimated_cost` at 0.0 while serde would have given
    // 1.0. If the engine read the absent number literally, "risk not declared" would mean
    // "executing this wrongly is free", and the ladder would silently stop asking altogether.
    let gid = add_goal(&mut store, "cost-b", "deploy_service", vec![], vec![]);

    let outcome = ClarifyEngine::run_with_budget(
        &mut store,
        gid,
        "cost-b",
        ClarifyTrigger::EmptyAC,
        None,
        SubstrateBudget { b0: 100.0 },
    );

    let rungs = ClarifyEngine::ladder_rungs(&store, gid);
    assert_eq!(outcome, ClarifyOutcome::NeedsUserClarification);
    let t3 = rungs
        .iter()
        .find(|r| r.tier == ClarifyTier::T3AskUser.as_str())
        .expect("T3 must be reached");
    assert_eq!(t3.outcome, "asked");
    assert!(
        t3.evidence
            .iter()
            .any(|e| e == &format!("cost_of_wrong_execution={}", hipcortex::payloads::DEFAULT_GOAL_COST)),
        "the fallback value must be the one the trace reports: {:?}",
        t3.evidence
    );
}

// ── Contract: the JSON a client actually sends, and what happens when it is unreadable ──
//
// Every other test in this file builds a `GoalPayload` in Rust and serialises *that*, so
// `status`, `acceptance_criteria` and `success_factors` were always present and the ladder never
// once met a body a client would really send. A live `POST /memory/add` carrying only
// `{"target_state": ...}` therefore produced a Goal record *no* consumer could parse, and the
// ladder reported it as `NeedsUserClarification` with an empty ledger — claiming a search it had
// not performed, in the one situation a human most needs the trace to be accurate. This pair
// closes that blind spot: the first pins the minimal body, the second pins what the ladder says
// when a record is unreadable anyway. "The substrate could not read the goal" and "the substrate
// searched and found nothing" are different claims, and only one of them is the user's to fix.

#[test]
fn minimal_client_goal_body_parses_and_only_target_state_is_required() {
    // What `POST /memory/add` with `record_type: "goal"` carries when a client omits the fields
    // it has no opinion about. `GoalPayload` derives `Default` and every one of these fields has
    // a sensible default, so the JSON contract has to agree with the Rust one.
    let parsed: GoalPayload = serde_json::from_value(serde_json::json!({
        "target_state": "ship_the_release"
    }))
    .expect("the minimal client body must parse into GoalPayload");
    assert_eq!(parsed.target_state, "ship_the_release");
    assert_eq!(parsed.status, GoalStatus::Pending);
    assert!(parsed.success_factors.is_empty());
    assert!(parsed.acceptance_criteria.is_empty());

    // The loosening is exactly as wide as "the Rust default is reachable from JSON". A goal with
    // no target is not a goal; a goal with no AC yet is the case the ladder exists for.
    assert!(
        serde_json::from_value::<GoalPayload>(serde_json::json!({})).is_err(),
        "a Goal record with no target_state must still be rejected"
    );
}

#[test]
fn an_unreadable_goal_record_records_why_it_stopped_instead_of_only_asking() {
    let mut store = make_store();
    // A Goal-typed record whose metadata is not a `GoalPayload`. Reachable from the public API
    // (`/memory/add` accepts any metadata for `record_type: "goal"`), so the ladder has to
    // behave sensibly outside the shape its own fixtures use.
    let rec = MemoryRecord::new(
        MemoryType::Goal,
        "ladder-unreadable".to_string(),
        "set_goal".to_string(),
        "mystery".to_string(),
        serde_json::json!({ "not_a_goal_payload": true }),
    );
    let gid = rec.id;
    store.add(rec).unwrap();

    let outcome = ClarifyEngine::run(
        &mut store,
        gid,
        "ladder-unreadable",
        ClarifyTrigger::EmptyAC,
        None,
    );

    // The caller still gets an answer it can act on (the loop parks the goal and stops)...
    assert_eq!(outcome, ClarifyOutcome::NeedsUserClarification);
    // ...but the ledger must not imply a search happened.
    assert!(
        ClarifyEngine::ladder_rungs(&store, gid).is_empty(),
        "no rung can have fired: the ladder could not read the goal"
    );
    let exits = ClarifyEngine::ladder_exit_reasons(&store, gid);
    assert!(
        exits
            .iter()
            .any(|e| e.starts_with("goal_metadata_unparseable")),
        "the trace must name the substrate's own failure rather than show an empty search: {exits:?}"
    );
}
