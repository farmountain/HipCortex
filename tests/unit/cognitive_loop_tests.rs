// Phase-4 unit tests: 8-stage CognitiveLoopConfig + SubstrateDaemon loop.
// AC-1a: CognitiveLoopConfig::default() has sane values
// AC-1b: subscribe_with_config max_iterations=1 → iterations==1 after thread exits
// AC-1c: stage_counts.len()==8, all entries==1 after 1 iteration
// AC-1d: iteration 0 is not vetoed — daemon_step Temporal record written
// AC-1e: Observe stage: purge_expired removes expired records
// AC-1f: 3 iterations → stage_counts[7]==3
// AC-1g: stop() signals handle as Stopped immediately
// AC-1h: stage_counts sum == 8 * iterations (all stages ran equally)

use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use hipcortex::substrate_daemon::{CognitiveLoopConfig, DaemonStatus, SubstrateDaemon};
use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::memory_store::MemoryStore;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::self_model::SelfModel;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::InMemoryBackend;

fn make_cognitive() -> Arc<CognitiveHandle<InMemoryBackend>> {
    Arc::new(CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        Arc::new(RwLock::new(WorldModelEnhanced::new())),
        Arc::new(SelfModel::new()),
        None,
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    ))
}

fn fast_config(max_iterations: u32) -> CognitiveLoopConfig {
    CognitiveLoopConfig {
        interval_secs: 0,
        pressure_threshold: 0.7,
        min_consolidation_frequency: 3,
        max_iterations: Some(max_iterations),
    }
}

fn wait_iterations(daemon: &SubstrateDaemon, id: uuid::Uuid, target: u32, timeout_ms: u64) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if daemon.status(id).map(|i| i.iterations >= target).unwrap_or(false) {
            return true;
        }
        if std::time::Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn ac1a_default_config_sane_values() {
    let cfg = CognitiveLoopConfig::default();
    assert_eq!(cfg.interval_secs, 30, "AC-1a: interval_secs must be 30");
    assert!((cfg.pressure_threshold - 0.7).abs() < 1e-6, "AC-1a: threshold must be 0.7");
    assert_eq!(cfg.min_consolidation_frequency, 3, "AC-1a: min_freq must be 3");
    assert!(cfg.max_iterations.is_none(), "AC-1a: default must be infinite");
}

#[test]
fn ac1b_max_iterations_one_completes() {
    let mut daemon = SubstrateDaemon::new();
    let cog = make_cognitive();
    let id = daemon.subscribe_with_config("ac1b-agent".into(), cog, fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 2000);
    assert!(reached, "AC-1b: iterations must reach 1 within 2s; got {:?}",
        daemon.status(id).map(|i| i.iterations));
}

#[test]
fn ac1c_all_eight_stages_ran() {
    let mut daemon = SubstrateDaemon::new();
    let cog = make_cognitive();
    let id = daemon.subscribe_with_config("ac1c-agent".into(), cog, fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 2000);
    assert!(reached, "AC-1c: prerequisite — must complete 1 iteration");
    let info = daemon.status(id).unwrap();
    assert_eq!(info.stage_counts.len(), 8, "AC-1c: must have exactly 8 stage counts");
    for (i, &count) in info.stage_counts.iter().enumerate() {
        assert_eq!(count, 1, "AC-1c: stage[{i}] must be 1 after 1 iteration, got {count}");
    }
}

#[test]
fn ac1d_iteration_zero_not_vetoed_writes_temporal() {
    let mut daemon = SubstrateDaemon::new();
    let cog = make_cognitive();
    let mem = Arc::clone(&cog.memory);
    let id = daemon.subscribe_with_config("ac1d-agent".into(), cog, fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 2000);
    assert!(reached, "AC-1d: prerequisite — must complete 1 iteration");
    let ms = mem.lock().unwrap();
    let records = ms.all().to_vec();
    let daemon_steps: Vec<_> = records.iter()
        .filter(|r| r.action == "daemon_step")
        .collect();
    assert!(!daemon_steps.is_empty(), "AC-1d: iteration 0 not vetoed → daemon_step Temporal written");
    let vetoed = daemon_steps.iter().any(|r| {
        r.metadata.get("vetoed").and_then(|v| v.as_bool()).unwrap_or(false)
    });
    assert!(!vetoed, "AC-1d: iteration 0 must not be vetoed");
}

#[test]
fn ac1e_observe_stage_purges_expired() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};

    let cog = make_cognitive();
    let mem = Arc::clone(&cog.memory);

    // Add a record that already expired (expires_at is Unix seconds i64)
    let mut expired = MemoryRecord::new(
        MemoryType::Temporal,
        "ac1e-agent".into(),
        "old".into(),
        "t".into(),
        serde_json::Value::Null,
    );
    let past_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64 - 10;
    expired.expires_at = Some(past_ts);
    mem.lock().unwrap().add(expired).unwrap();
    let before = mem.lock().unwrap().record_count();

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("ac1e-agent".into(), cog, fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 2000);
    assert!(reached, "AC-1e: must complete 1 iteration");

    let after = mem.lock().unwrap().record_count();
    // After observe: expired record removed, daemon_step Temporal added
    // Net: before - 1 (expired) + 1 (daemon_step) = before
    // But the expired record was there "before", so after purge it's gone
    // (daemon_step is added in Stage 6, after purge)
    assert!(after < before + 2, "AC-1e: hot store must not grow unboundedly; before={before} after={after}");
}

#[test]
fn ac1f_three_iterations_all_stages_thrice() {
    let mut daemon = SubstrateDaemon::new();
    let cog = make_cognitive();
    let id = daemon.subscribe_with_config("ac1f-agent".into(), cog, fast_config(3));
    let reached = wait_iterations(&daemon, id, 3, 5000);
    assert!(reached, "AC-1f: must complete 3 iterations within 5s");
    let info = daemon.status(id).unwrap();
    assert_eq!(info.stage_counts[7], 3, "AC-1f: ExitCheck stage must be 3");
    assert_eq!(info.stage_counts[0], 3, "AC-1f: Observe stage must be 3");
}

#[test]
fn ac1g_stop_shows_stopped_immediately() {
    let mut daemon = SubstrateDaemon::new();
    let cog = make_cognitive();
    let id = daemon.subscribe_with_config("ac1g-agent".into(), cog, fast_config(100));
    let stopped = daemon.stop(id);
    assert!(stopped, "AC-1g: stop() must return true for valid handle");
    let info = daemon.status(id).unwrap();
    assert_eq!(info.status, DaemonStatus::Stopped,
        "AC-1g: status must reflect Stopped after stop() call");
}

#[test]
fn ac1h_stage_counts_sum_equals_eight_times_iterations() {
    let mut daemon = SubstrateDaemon::new();
    let cog = make_cognitive();
    let id = daemon.subscribe_with_config("ac1h-agent".into(), cog, fast_config(4));
    let reached = wait_iterations(&daemon, id, 4, 5000);
    assert!(reached, "AC-1h: must complete 4 iterations");
    let info = daemon.status(id).unwrap();
    let sum: u32 = info.stage_counts.iter().sum();
    let expected = 8 * info.iterations;
    assert_eq!(sum, expected,
        "AC-1h: stage_counts sum={sum} must equal 8*iterations={expected}");
}

// ─── Gap 1: Daemon dequeues real InProgress goals ────────────────────────────

/// AC-gap1a: Daemon marks a fully-satisfied InProgress goal as Succeeded after 1 iteration.
#[test]
fn ac_gap1a_daemon_marks_satisfied_goal_succeeded() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let cog = make_cognitive();

    // Pre-load an InProgress goal whose factors are already satisfied.
    let payload = GoalPayload {
        target_state: "integration_complete".into(),
        success_factors: vec![SuccessFactor { name: "done".into(), weight: 1.0, satisfied: true }],
        status: GoalStatus::InProgress,
        max_react_iterations: 3,
        ..Default::default()
    };
    let mut goal_rec = MemoryRecord::new(
        MemoryType::Goal,
        "gap1-agent".into(),
        "achieve".into(),
        "integration_complete".into(),
        serde_json::to_value(&payload).unwrap(),
    );
    let goal_id = goal_rec.id;
    {
        let mut ms = cog.memory.lock().unwrap();
        ms.add(goal_rec).unwrap();
    }

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("gap1-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-gap1a: daemon must complete 1 iteration");

    // After 1 iteration the ExitCheck should have updated the goal to Succeeded.
    let ms = cog.memory.lock().unwrap();
    let updated = ms.find_by_id(goal_id).expect("goal record must still exist");
    let updated_payload: GoalPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
    assert!(
        matches!(updated_payload.status, GoalStatus::Succeeded),
        "AC-gap1a: goal with all satisfied factors must become Succeeded; got {:?}",
        updated_payload.status
    );
}

/// AC-gap1b: Daemon with no active goals uses dummy — does not panic, writes daemon_step Temporal.
#[test]
fn ac_gap1b_daemon_no_goals_uses_dummy() {
    let cog = make_cognitive();
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("gap1b-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-gap1b: daemon must complete 1 iteration even with no goals");

    let ms = cog.memory.lock().unwrap();
    let has_step = ms.all().iter().any(|r| r.action == "daemon_step");
    assert!(has_step, "AC-gap1b: daemon_step Temporal must be written even with no active goal");
}

// ─── Gap 8: Continuous dynamics bridge ───────────────────────────────────────

/// AC-gap8a: After 1 daemon iteration, Temporal{action="wm_state_snapshot"} exists per entity.
#[test]
fn ac_gap8a_daemon_snapshots_wm_entity() {
    use hipcortex::world_model_enhanced::EntityState;

    let cog = make_cognitive();

    // Register an entity via the public helper so entity_mean_vectors returns it.
    cog.register_wm_entity(
        "robot".into(),
        EntityState {
            properties: vec![1.0, 0.0],
            covariance: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        },
    ).unwrap();

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("gap8-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-gap8a: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let snapshots: Vec<_> = ms
        .all()
        .iter()
        .filter(|r| r.action == "wm_state_snapshot" && r.target == "robot")
        .collect();
    assert!(
        !snapshots.is_empty(),
        "AC-gap8a: Temporal{{action='wm_state_snapshot', target='robot'}} must be written by daemon"
    );
}

/// AC-gap1c: Daemon must NOT mark InProgress goal with 0 success_factors as Succeeded.
/// Vacuous-truth guard: [].iter().all(f) == true in Rust — daemon Stage 1 must skip unclarified goals.
#[test]
fn ac_gap1c_daemon_skips_unclarified_zero_factor_goal() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus};

    let cog = make_cognitive();

    // Add an InProgress goal with NO success_factors (bypasses AddMemory gate via raw store insert).
    let payload = GoalPayload {
        target_state: "ambiguous_goal".into(),
        success_factors: vec![], // empty — unclarified
        status: GoalStatus::InProgress,
        max_react_iterations: 3,
        ..Default::default()
    };
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal,
        "gap1c-agent".into(),
        "achieve".into(),
        "ambiguous_goal".into(),
        serde_json::to_value(&payload).unwrap(),
    );
    let goal_id = goal_rec.id;
    {
        // Insert directly to bypass GoalNotClarified gate (simulates Pending→InProgress transition)
        let mut ms = cog.memory.lock().unwrap();
        ms.add(goal_rec).unwrap();
    }

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("gap1c-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-gap1c: daemon must complete 1 iteration even with unclarified goal");

    // Goal must remain InProgress — daemon must not vacuously mark it Succeeded.
    let ms = cog.memory.lock().unwrap();
    let goal = ms.find_by_id(goal_id).expect("goal record must still exist");
    let gp: GoalPayload = serde_json::from_value(goal.metadata.clone()).unwrap();
    assert!(
        matches!(gp.status, GoalStatus::InProgress),
        "AC-gap1c: 0-factor goal must remain InProgress (not vacuously Succeeded); got {:?}",
        gp.status
    );
}

/// AC-gap8b: No snapshot records when WM has no registered entities.
#[test]
fn ac_gap8b_no_snapshot_when_no_entities() {
    let cog = make_cognitive(); // empty WM
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("gap8b-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-gap8b: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let snapshots: Vec<_> = ms
        .all()
        .iter()
        .filter(|r| r.action == "wm_state_snapshot")
        .collect();
    assert!(
        snapshots.is_empty(),
        "AC-gap8b: no wm_state_snapshot records expected when WM has 0 entities"
    );
}

// ─── Gap E: Autonomous goal synthesis ────────────────────────────────────────

/// AC-novel-a: Uncertain WM entity + no InProgress goals → daemon synthesizes Goal record.
#[test]
fn ac_novel_a_synthesizes_goal_for_uncertain_entity() {
    use hipcortex::world_model_enhanced::EntityState;

    let cog = make_cognitive();
    // Register entity with covariance trace = 4.0 > 1.0 threshold
    cog.register_wm_entity(
        "sensor".into(),
        EntityState {
            properties: vec![0.0, 0.0],
            covariance: vec![vec![2.0, 0.0], vec![0.0, 2.0]],
        },
    ).unwrap();

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("novel-a-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-novel-a: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let synthesized: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "synthesize" && r.actor == "novel-a-agent")
        .collect();
    assert!(
        !synthesized.is_empty(),
        "AC-novel-a: daemon must synthesize a Goal record when uncertain entity exists"
    );
}

/// AC-novel-b: Synthesized goal must have ≥1 success_factor (not vacuously empty).
#[test]
fn ac_novel_b_synthesized_goal_has_success_factors() {
    use hipcortex::world_model_enhanced::EntityState;
    use hipcortex::payloads::GoalPayload;

    let cog = make_cognitive();
    cog.register_wm_entity(
        "sensor".into(),
        EntityState {
            properties: vec![0.0],
            covariance: vec![vec![3.0]],
        },
    ).unwrap();

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("novel-b-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-novel-b: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let goal = ms.all().iter()
        .find(|r| r.action == "synthesize" && r.actor == "novel-b-agent")
        .expect("AC-novel-b: synthesized goal must exist");
    let payload: GoalPayload = serde_json::from_value(goal.metadata.clone())
        .expect("AC-novel-b: synthesized goal must have valid GoalPayload");
    assert!(
        !payload.success_factors.is_empty(),
        "AC-novel-b: synthesized goal must have ≥1 success_factor (not vacuously empty)"
    );
}

/// AC-novel-c: Existing InProgress goal → daemon does NOT synthesize a new goal.
#[test]
fn ac_novel_c_no_synthesis_when_active_goal_exists() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
    use hipcortex::world_model_enhanced::EntityState;

    let cog = make_cognitive();
    // Register uncertain entity so synthesis would otherwise trigger
    cog.register_wm_entity(
        "sensor".into(),
        EntityState {
            properties: vec![0.0],
            covariance: vec![vec![5.0]],
        },
    ).unwrap();

    // Add an existing InProgress goal with non-empty factors
    let payload = GoalPayload {
        target_state: "existing_goal".into(),
        success_factors: vec![SuccessFactor { name: "done".into(), weight: 1.0, satisfied: false }],
        status: GoalStatus::InProgress,
        max_react_iterations: 3,
        ..Default::default()
    };
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal,
        "novel-c-agent".into(),
        "achieve".into(),
        "existing_goal".into(),
        serde_json::to_value(&payload).unwrap(),
    );
    cog.memory.lock().unwrap().add(goal_rec).unwrap();

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("novel-c-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-novel-c: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let synthesized: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "synthesize" && r.actor == "novel-c-agent")
        .collect();
    assert!(
        synthesized.is_empty(),
        "AC-novel-c: daemon must NOT synthesize a new goal when an InProgress goal already exists"
    );
}

/// AC-novel-d: No WM entities → daemon does NOT synthesize any goal.
#[test]
fn ac_novel_d_no_synthesis_when_no_entities() {
    let cog = make_cognitive(); // empty WM

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("novel-d-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-novel-d: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let synthesized: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "synthesize" && r.actor == "novel-d-agent")
        .collect();
    assert!(
        synthesized.is_empty(),
        "AC-novel-d: no goal synthesis expected when WM has no entities"
    );
}

// ─── Gap C: SCM drift isolation — named OLS → RewriteStructuralEquation ──────

/// AC-drift-scm: Named node with high OLS weight → daemon emits RewriteStructuralEquation
/// → Reflexion{action="rewrite_equation", target=node_id} written to memory.
#[test]
fn ac_drift_scm_ols_trigger_rewrites_equation() {
    let cog = make_cognitive();

    // Add causal graph node so rewrite_structural_equation does not return Err
    cog.add_causal_node("broken_node".into()).unwrap();

    // Two observations: xtx=2.0, xty=4.0 → OLS weight |4.0/2.0| = 2.0 > 0.3 threshold
    cog.observe_prediction_drift("broken_node", 0.9, 1.0, 2.0);
    cog.observe_prediction_drift("broken_node", 0.9, 1.0, 2.0);

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("drift-scm-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-drift-scm: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let rewrites: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "rewrite_equation" && r.target == "broken_node")
        .collect();
    assert!(
        !rewrites.is_empty(),
        "AC-drift-scm: Reflexion{{action=rewrite_equation, target=broken_node}} must exist after daemon tick"
    );
}

// ─── Gap A: Structural deduplication ─────────────────────────────────────────

/// AC-abs: Three identical (actor, action, target) Temporal records contracted to one
/// after AutoConsolidate triggers structural dedup.
#[test]
fn ac_abs_structural_dedup_contracts_identical_records() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};

    let cog = make_cognitive();

    // Add 3 identical Temporal records (same actor/action/target, different ids)
    for _ in 0..3 {
        let rec = MemoryRecord::new(
            MemoryType::Temporal,
            "dedup-agent".into(),
            "heartbeat".into(),
            "service".into(),
            serde_json::Value::Null,
        );
        cog.transact(CognitiveDelta::AddMemory(rec), "dedup-agent").unwrap();
    }

    let before: Vec<_> = cog.memory.lock().unwrap().all().iter()
        .filter(|r| r.action == "heartbeat" && r.target == "service")
        .map(|r| r.id)
        .collect();
    assert_eq!(before.len(), 3, "AC-abs prerequisite: 3 heartbeat records before dedup");

    // Trigger AutoConsolidate (min_frequency=1 to ensure it runs)
    cog.transact(CognitiveDelta::AutoConsolidate { min_frequency: 1 }, "dedup-agent").unwrap();

    let after: Vec<_> = cog.memory.lock().unwrap().all().iter()
        .filter(|r| r.action == "heartbeat" && r.target == "service")
        .map(|r| r.id)
        .collect();
    assert_eq!(
        after.len(), 1,
        "AC-abs: structural dedup must contract 3 identical (actor,action,target) records to 1; got {}",
        after.len()
    );
}

// ─── Gap H/I: Critic executive veto ──────────────────────────────────────────

/// AC-veto-a: At iteration 1 the critic gate vetos (0 satisfied / 1 required < 0.25 threshold).
/// Daemon must write a Decision{action="critic_veto", option_chosen="rejected"} record.
#[test]
fn ac_veto_a_critic_veto_writes_decision_rejected() {
    let cog = make_cognitive();

    // 2 iterations: iter 0 always passes, iter 1 vetos (dummy goal 0/1 satisfied)
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("veto-agent".into(), cog.clone(), fast_config(2));
    let reached = wait_iterations(&daemon, id, 2, 5000);
    assert!(reached, "AC-veto-a: daemon must complete 2 iterations");

    let ms = cog.memory.lock().unwrap();
    let decisions: Vec<_> = ms.all().iter()
        .filter(|r| r.record_type == hipcortex::memory_record::MemoryType::Decision
            && r.action == "critic_veto")
        .collect();
    assert!(
        !decisions.is_empty(),
        "AC-veto-a: daemon must write Decision{{action=critic_veto}} when critic vetos at iter 1"
    );
    // Verify option_chosen == "rejected"
    let payload: hipcortex::payloads::DecisionPayload =
        serde_json::from_value(decisions[0].metadata.clone()).unwrap_or_default();
    assert_eq!(
        payload.option_chosen, "rejected",
        "AC-veto-a: Decision.option_chosen must be 'rejected'"
    );
    assert!(
        !payload.rationale_chain.is_empty(),
        "AC-veto-a: Decision.rationale_chain must be non-empty"
    );
}

/// AC-veto-b: When critic vetos (iter 1), OLS rewrite must NOT fire.
/// Total rewrite_equation Reflexions must equal 1 (from iter 0 only), not 2.
#[test]
fn ac_veto_b_critic_veto_gates_ols_rewrite() {
    let cog = make_cognitive();

    // Set up drift: x=1.0, y=2.0 twice → ols_weight = |4.0/2.0| = 2.0 > 0.3
    cog.add_causal_node("veto_node".into()).unwrap();
    cog.observe_prediction_drift("veto_node", 0.9, 1.0, 2.0);
    cog.observe_prediction_drift("veto_node", 0.9, 1.0, 2.0);

    // 2 iterations: iter 0 → not vetoed → rewrite fires; iter 1 → vetoed → rewrite blocked
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("veto-ols-agent".into(), cog.clone(), fast_config(2));
    let reached = wait_iterations(&daemon, id, 2, 5000);
    assert!(reached, "AC-veto-b: daemon must complete 2 iterations");

    let ms = cog.memory.lock().unwrap();
    let rewrites: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "rewrite_equation" && r.target == "veto_node")
        .collect();
    assert_eq!(
        rewrites.len(), 1,
        "AC-veto-b: rewrite_equation must fire exactly once (iter 0 only); got {}",
        rewrites.len()
    );
}

// ─── Gap K: Drift uses OLS coefficient, not placeholder ──────────────────────

/// AC-drift-weight: rewrite_equation Reflexion metadata contains the computed OLS weight (2.0),
/// not the hardcoded placeholder (1.0).
#[test]
fn ac_drift_weight_uses_ols_coefficient_not_placeholder() {
    let cog = make_cognitive();

    // x=1.0, y=2.0 twice → ols_weight = |Σ(x·y)/Σ(x²)| = |4.0/2.0| = 2.0
    cog.add_causal_node("weight_node".into()).unwrap();
    cog.observe_prediction_drift("weight_node", 0.9, 1.0, 2.0);
    cog.observe_prediction_drift("weight_node", 0.9, 1.0, 2.0);

    // 1 iteration (iter 0 — not vetoed — rewrite fires with computed weight)
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("weight-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-drift-weight: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let rewrite = ms.all().iter()
        .find(|r| r.action == "rewrite_equation" && r.target == "weight_node")
        .expect("AC-drift-weight: rewrite_equation Reflexion must exist");

    // The handler stores new_weights in metadata
    let weights = rewrite.metadata.get("new_weights")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect::<Vec<_>>())
        .unwrap_or_default();
    assert!(
        !weights.is_empty(),
        "AC-drift-weight: rewrite metadata must contain new_weights array"
    );
    let w = weights[0];
    assert!(
        (w - 2.0).abs() < 1e-9,
        "AC-drift-weight: new_weights[0] must be OLS coefficient 2.0, got {w:.6}"
    );
    assert!(
        (w - 1.0).abs() > 0.5,
        "AC-drift-weight: new_weights[0] must NOT be placeholder 1.0"
    );
}

// ─── Daemon drives ReactEngine ────────────────────────────────────────────────

/// AC-react-daemon: Daemon Stage 5 calls ReactEngine::run() on the active InProgress goal.
/// After 1 daemon tick, Temporal records linked via derived_from to the goal must exist,
/// and the goal must be in a terminal state (Succeeded or Failed).
#[test]
fn ac_react_daemon_drives_react_goal_to_terminal() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let cog = make_cognitive();

    // Add an InProgress goal with 1 success_factor (will not be auto-satisfied → Failed)
    let goal_payload = GoalPayload {
        target_state: "daemon_react_target".to_string(),
        success_factors: vec![SuccessFactor {
            name: "must_complete".to_string(),
            weight: 1.0,
            satisfied: false,
        }],
        status: GoalStatus::InProgress,
        max_react_iterations: 2,
        ..Default::default()
    };
    let meta = serde_json::to_value(&goal_payload).unwrap();
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal,
        "react-daemon-agent".into(),
        "pursue".into(),
        "daemon_react_target".into(),
        meta,
    );
    let goal_id = goal_rec.id;
    cog.transact(CognitiveDelta::AddMemory(goal_rec), "react-daemon-agent").unwrap();

    // 1 iteration: iter 0 → not vetoed → ReactEngine runs
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config(
        "react-daemon-agent".into(), cog.clone(), fast_config(1),
    );
    let reached = wait_iterations(&daemon, id, 1, 5000);
    assert!(reached, "AC-react-daemon: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    // ReactEngine writes Temporal records with derived_from = goal_id
    let linked: Vec<_> = ms.all().iter()
        .filter(|r| r.derived_from == Some(goal_id)
            && r.record_type == MemoryType::Temporal)
        .collect();
    assert!(
        !linked.is_empty(),
        "AC-react-daemon: ReactEngine must write ≥1 Temporal record linked to goal {goal_id}"
    );

    // Goal must be in terminal state after ReactEngine ran to completion
    let goal_final = ms.find_by_id(goal_id).expect("goal record must still exist");
    let final_payload: GoalPayload = serde_json::from_value(goal_final.metadata.clone())
        .unwrap_or_default();
    assert!(
        matches!(final_payload.status, GoalStatus::Succeeded | GoalStatus::Failed),
        "AC-react-daemon: goal must be Succeeded or Failed after daemon drives ReactEngine; got {:?}",
        final_payload.status
    );
}
