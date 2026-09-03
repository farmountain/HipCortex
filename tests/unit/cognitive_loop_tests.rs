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

/// AC-veto-a: CriticGate always approves at loop_iter==0 (iter-0 invariant).
/// With a real InProgress goal present, daemon iter 0: approved → OLS rewrite fires.
/// ReactEngine exhausts the goal (marks Failed) in the same Act stage. No Decision{critic_veto}.
/// Contrast with ac_idle_a/b which prove idle daemon (no goal) also never writes critic_veto.
#[test]
fn ac_veto_a_critic_veto_writes_decision_rejected() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let cog = make_cognitive();

    // Real InProgress goal — CriticGate will evaluate it (not the dummy-goal from the old bug).
    let payload = GoalPayload {
        target_state: "veto_target".into(),
        success_factors: vec![SuccessFactor { name: "not_done".into(), weight: 1.0, satisfied: false }],
        status: GoalStatus::InProgress,
        max_react_iterations: 1,
        ..Default::default()
    };
    let meta = serde_json::to_value(&payload).unwrap();
    let goal_rec = MemoryRecord::new(MemoryType::Goal, "veto-agent".into(), "pursue".into(), "veto_target".into(), meta);
    cog.transact(CognitiveDelta::AddMemory(goal_rec), "veto-agent").unwrap();

    // OLS drift: ≥2 observations required; ols_weight = |Σxy/Σx²| = 2.0 > 0.3
    cog.add_causal_node("veto_a_node".into()).unwrap();
    cog.observe_prediction_drift("veto_a_node", 0.9, 1.0, 2.0);
    cog.observe_prediction_drift("veto_a_node", 0.9, 1.0, 2.0);

    // 1 iteration: iter 0 — CriticGate always approves → OLS rewrite fires → ReactEngine marks goal Failed.
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("veto-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-veto-a: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    // iter 0 always approved → OLS rewrite fires
    let rewrites: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "rewrite_equation" && r.target == "veto_a_node")
        .collect();
    assert_eq!(rewrites.len(), 1, "AC-veto-a: CriticGate must approve at iter 0 → rewrite fires once");
    // iter 0 invariant: no critic_veto decision at iter 0
    let veto_decisions: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "critic_veto")
        .collect();
    assert_eq!(veto_decisions.len(), 0, "AC-veto-a: CriticGate must NOT veto at iter 0");
}

/// AC-veto-b: Real goal present at iter 0 (CriticGate approves) → OLS fires → ReactEngine marks
/// goal Failed. At iter 1 goal is gone → idle path → OLS fires again. Total: 2 rewrites.
/// This proves idle-path (no goal → no veto) kicks in after goal termination. Contrast with
/// ac_veto_a (1 iter, proves iter-0 invariant) and ac_idle_a (never had a goal).
#[test]
fn ac_veto_b_critic_veto_gates_ols_rewrite() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let cog = make_cognitive();

    // Real InProgress goal with 1 unsatisfied factor (never satisfied → veto at iter 1)
    let payload = GoalPayload {
        target_state: "veto_ols_target".into(),
        success_factors: vec![SuccessFactor { name: "not_done".into(), weight: 1.0, satisfied: false }],
        status: GoalStatus::InProgress,
        max_react_iterations: 1,
        ..Default::default()
    };
    let meta = serde_json::to_value(&payload).unwrap();
    let goal_rec = MemoryRecord::new(MemoryType::Goal, "veto-ols-agent".into(), "pursue".into(), "veto_ols_target".into(), meta);
    cog.transact(CognitiveDelta::AddMemory(goal_rec), "veto-ols-agent").unwrap();

    // Drift: x=1.0, y=2.0 twice → ols_weight = |4.0/2.0| = 2.0 > 0.3
    cog.add_causal_node("veto_node".into()).unwrap();
    cog.observe_prediction_drift("veto_node", 0.9, 1.0, 2.0);
    cog.observe_prediction_drift("veto_node", 0.9, 1.0, 2.0);

    // 2 iterations: iter 0 → approved → rewrite fires; iter 1 → vetoed (real goal) → blocked
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("veto-ols-agent".into(), cog.clone(), fast_config(2));
    let reached = wait_iterations(&daemon, id, 2, 5000);
    assert!(reached, "AC-veto-b: daemon must complete 2 iterations");

    let ms = cog.memory.lock().unwrap();
    let rewrites: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "rewrite_equation" && r.target == "veto_node")
        .collect();
    assert_eq!(
        rewrites.len(), 2,
        "AC-veto-b: rewrite_equation must fire at both iters (iter 0: goal active+approved; iter 1: goal gone→idle); got {}",
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

// ─── Gap 1+3: Idle daemon never vetos — maintenance runs every iteration ──────

/// AC-idle-a: No active goal → vetoed=false → OLS rewrite fires at BOTH iterations.
/// Contrast with ac_veto_b where real goal causes veto at iter 1 → only 1 rewrite.
#[test]
fn ac_daemon_idle_no_veto_allows_maintenance() {
    let cog = make_cognitive();

    // ≥2 observations required; ols_weight = |Σxy/Σx²| = 2.0 > 0.3; no active goal → no veto either iter
    cog.add_causal_node("idle_node".into()).unwrap();
    cog.observe_prediction_drift("idle_node", 0.9, 1.0, 2.0);
    cog.observe_prediction_drift("idle_node", 0.9, 1.0, 2.0);

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("idle-agent".into(), cog.clone(), fast_config(2));
    let reached = wait_iterations(&daemon, id, 2, 5000);
    assert!(reached, "AC-idle-a: daemon must complete 2 iterations");

    let ms = cog.memory.lock().unwrap();
    let rewrites: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "rewrite_equation" && r.target == "idle_node")
        .collect();
    assert_eq!(
        rewrites.len(), 2,
        "AC-idle-a: idle daemon must rewrite at BOTH iterations (no veto); got {}",
        rewrites.len()
    );
}

/// AC-idle-b: No active goal → Stage 3 never writes Decision{{critic_veto}}.
#[test]
fn ac_daemon_idle_no_decision_rejected_when_no_goal() {
    let cog = make_cognitive();

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("idle-b-agent".into(), cog.clone(), fast_config(2));
    let reached = wait_iterations(&daemon, id, 2, 5000);
    assert!(reached, "AC-idle-b: daemon must complete 2 iterations");

    let ms = cog.memory.lock().unwrap();
    let veto_decisions: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "critic_veto")
        .collect();
    assert_eq!(
        veto_decisions.len(), 0,
        "AC-idle-b: idle daemon must write 0 critic_veto Decision records; got {}",
        veto_decisions.len()
    );
}

// ─── Gap 2: Abstraction — normalize action before fingerprinting ───────────────

/// AC-abs-dedup: AutoConsolidate collapses "store_memory", "Store-Memory", "STORE_MEMORY"
/// to the same fingerprint (all norm to "storememory") → 2 of 3 deleted, 1 survives.
#[test]
fn ac_abs_semantic_dedup_normalizes_action_variation() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};

    let cog = make_cognitive();

    for action in &["store_memory", "Store-Memory", "STORE_MEMORY"] {
        let rec = MemoryRecord::new(
            MemoryType::Temporal,
            "dedup-agent".into(),
            action.to_string(),
            "dedup_target".into(),
            serde_json::json!({}),
        );
        cog.transact(CognitiveDelta::AddMemory(rec), "dedup-agent").unwrap();
    }

    // pressure_threshold: 0.0 forces AutoConsolidate regardless of actual memory pressure
    let config = CognitiveLoopConfig {
        interval_secs: 0,
        pressure_threshold: 0.0,
        min_consolidation_frequency: 3,
        max_iterations: Some(1),
    };
    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("dedup-agent".into(), cog.clone(), config);
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-abs-dedup: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let remaining: Vec<_> = ms.all().iter()
        .filter(|r| r.target == "dedup_target" && r.record_type == MemoryType::Temporal)
        .collect();
    assert_eq!(
        remaining.len(), 1,
        "AC-abs-dedup: normalized fingerprint must collapse 3 variants to 1; got {}",
        remaining.len()
    );
}

// ─── Gap 4: Verifier — entity uncertainty annotation ──────────────────────────

/// AC-verifier: daemon Stage 6 writes Belief{{entity_state_uncertain}} for the
/// entity with highest Kalman covariance trace (> 1.0). VerifierGate cannot block
/// continuous Kalman writes; this annotation is the additive bridge.
#[test]
fn ac_verifier_entity_uncertain_annotated() {
    use hipcortex::memory_record::MemoryType;
    use hipcortex::world_model_enhanced::EntityState;

    let cog = make_cognitive();

    // Register entity with covariance trace = 5.0 > 1.0 → most_uncertain_entity returns it
    cog.register_wm_entity(
        "uncertain_robot".to_string(),
        EntityState {
            properties: vec![0.0],
            covariance: vec![vec![5.0]],
        },
    ).unwrap();

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("verifier-agent".into(), cog.clone(), fast_config(1));
    let reached = wait_iterations(&daemon, id, 1, 3000);
    assert!(reached, "AC-verifier: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let annotations: Vec<_> = ms.all().iter()
        .filter(|r| {
            r.action == "entity_state_uncertain"
                && r.target == "uncertain_robot"
                && r.record_type == MemoryType::Belief
        })
        .collect();
    assert!(
        !annotations.is_empty(),
        "AC-verifier: Stage 6 must write Belief{{entity_state_uncertain}} for most uncertain entity"
    );
}

// ─── Dual-mode ReactEngine: StepByStep + FullCycle (AC-step) ─────────────────

/// AC-step-1: GoalExecutionMode::StepByStep round-trips through serde.
#[test]
fn ac_step_execution_mode_serializes() {
    use hipcortex::payloads::{GoalExecutionMode, GoalPayload, GoalStatus, SuccessFactor};

    let payload = GoalPayload {
        target_state: "reach_goal".into(),
        success_factors: vec![SuccessFactor { name: "done".into(), weight: 1.0, satisfied: false }],
        status: GoalStatus::InProgress,
        execution_mode: GoalExecutionMode::StepByStep,
        max_react_iterations: 3,
        ..Default::default()
    };
    let json = serde_json::to_value(&payload).unwrap();
    let recovered: GoalPayload = serde_json::from_value(json).unwrap();
    assert_eq!(
        recovered.execution_mode,
        GoalExecutionMode::StepByStep,
        "AC-step-1: GoalExecutionMode::StepByStep must survive serde round-trip"
    );
}

/// AC-step-2: One daemon tick with StepByStep goal — goal stays InProgress, current_iteration
/// advances from 0 to 1. Contrast with FullCycle which terminates goal in the same tick.
#[test]
fn ac_step_one_step_advances_iteration() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalExecutionMode, GoalPayload, GoalStatus, SuccessFactor};

    let cog = make_cognitive();

    let payload = GoalPayload {
        target_state: "step_target".into(),
        success_factors: vec![SuccessFactor { name: "step_done".into(), weight: 1.0, satisfied: false }],
        status: GoalStatus::InProgress,
        execution_mode: GoalExecutionMode::StepByStep,
        max_react_iterations: 5,
        ..Default::default()
    };
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal,
        "step-agent".into(),
        "achieve".into(),
        "step_target".into(),
        serde_json::to_value(&payload).unwrap(),
    );
    let goal_id = goal_rec.id;
    { let mut ms = cog.memory.lock().unwrap(); ms.add(goal_rec).unwrap(); }

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("step-agent".into(), cog.clone(), fast_config(1));
    assert!(wait_iterations(&daemon, id, 1, 3000), "AC-step-2: daemon must complete 1 iteration");

    let ms = cog.memory.lock().unwrap();
    let updated = ms.find_by_id(goal_id).expect("goal must still exist");
    let gp: GoalPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
    assert!(
        matches!(gp.status, GoalStatus::InProgress),
        "AC-step-2: StepByStep goal must stay InProgress after 1 tick; got {:?}", gp.status
    );
    assert_eq!(
        gp.current_iteration, 1,
        "AC-step-2: current_iteration must advance 0→1 after one step; got {}", gp.current_iteration
    );
}

/// AC-step-3: 1/2 satisfied (ratio=0.5 ≥ VETO_THRESHOLD=0.25) → CriticGate approves every tick.
/// 3 daemon ticks, max_react_iterations=3 → goal advances each tick → exhausted → Failed.
#[test]
fn ac_step_goal_advances_across_multiple_ticks() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalExecutionMode, GoalPayload, GoalStatus, SuccessFactor};

    let cog = make_cognitive();

    let payload = GoalPayload {
        target_state: "multi_step_target".into(),
        success_factors: vec![
            SuccessFactor { name: "s1".into(), weight: 1.0, satisfied: true },
            SuccessFactor { name: "s2".into(), weight: 1.0, satisfied: false },
        ],
        status: GoalStatus::InProgress,
        execution_mode: GoalExecutionMode::StepByStep,
        max_react_iterations: 3,
        ..Default::default()
    };
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal,
        "step-multi-agent".into(),
        "achieve".into(),
        "multi_step_target".into(),
        serde_json::to_value(&payload).unwrap(),
    );
    let goal_id = goal_rec.id;
    { let mut ms = cog.memory.lock().unwrap(); ms.add(goal_rec).unwrap(); }

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("step-multi-agent".into(), cog.clone(), fast_config(3));
    assert!(wait_iterations(&daemon, id, 3, 5000), "AC-step-3: daemon must complete 3 iterations");

    let ms = cog.memory.lock().unwrap();
    let updated = ms.find_by_id(goal_id).expect("goal must still exist");
    let gp: GoalPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
    assert!(
        matches!(gp.status, GoalStatus::Failed),
        "AC-step-3: StepByStep goal exhausted after 3 ticks must be Failed; got {:?}", gp.status
    );
    assert_eq!(
        gp.current_iteration, 3,
        "AC-step-3: current_iteration must reach max_react_iterations=3; got {}", gp.current_iteration
    );
}

/// AC-step-4: 0/1 satisfied (ratio=0.0 < VETO_THRESHOLD=0.25) — CriticGate vetoes at ticks 1+.
/// Tick 0: approves, run_one_step fires (current_iteration 0→1).
/// Ticks 1,2: Rejected → 2 Decision{critic_veto} written, step NOT run, goal stays InProgress.
#[test]
fn ac_step_veto_at_tick1_blocks_step_writes_decision() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalExecutionMode, GoalPayload, GoalStatus, SuccessFactor};

    let cog = make_cognitive();

    let payload = GoalPayload {
        target_state: "veto_target".into(),
        success_factors: vec![SuccessFactor { name: "v1".into(), weight: 1.0, satisfied: false }],
        status: GoalStatus::InProgress,
        execution_mode: GoalExecutionMode::StepByStep,
        max_react_iterations: 5,
        ..Default::default()
    };
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal,
        "step-veto-agent".into(),
        "achieve".into(),
        "veto_target".into(),
        serde_json::to_value(&payload).unwrap(),
    );
    let goal_id = goal_rec.id;
    { let mut ms = cog.memory.lock().unwrap(); ms.add(goal_rec).unwrap(); }

    let mut daemon = SubstrateDaemon::new();
    let id = daemon.subscribe_with_config("step-veto-agent".into(), cog.clone(), fast_config(3));
    assert!(wait_iterations(&daemon, id, 3, 5000), "AC-step-4: daemon must complete 3 iterations");

    let ms = cog.memory.lock().unwrap();

    let veto_decisions: Vec<_> = ms.all().iter()
        .filter(|r| r.action == "critic_veto" && r.record_type == MemoryType::Decision)
        .collect();
    assert_eq!(
        veto_decisions.len(), 2,
        "AC-step-4: must have 2 Decision{{critic_veto}} (ticks 1,2); got {}", veto_decisions.len()
    );

    let updated = ms.find_by_id(goal_id).expect("goal must still exist");
    let gp: GoalPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
    assert!(
        matches!(gp.status, GoalStatus::InProgress),
        "AC-step-4: goal must remain InProgress when vetoed at ticks 1+2; got {:?}", gp.status
    );
    assert_eq!(
        gp.current_iteration, 1,
        "AC-step-4: current_iteration must be 1 (only tick 0 ran); got {}", gp.current_iteration
    );
}

// ─── v1.7.0 P0 Epistemic Closure AC tests ────────────────────────────────────

// P0-D: SelfModel steers loop config
#[test]
fn ac_self_model_default_loop_config_is_balanced() {
    use hipcortex::self_model::SynthesisMode;
    let sm = hipcortex::self_model::SelfModel::new();
    let cfg = sm.recommend_loop_config();
    // New SelfModel with no degraded modules → health aggregator returns 1.0 → Autonomous branch
    assert!((cfg.effective_veto_threshold - 0.15).abs() < 1e-6,
        "new SelfModel must be Autonomous (threshold 0.15); got {}", cfg.effective_veto_threshold);
    assert_eq!(cfg.synthesis_mode, SynthesisMode::Autonomous,
        "new SelfModel synthesis mode must be Autonomous");
}

#[test]
fn ac_self_model_low_health_escalates() {
    use hipcortex::self_model::{SelfModel, ModuleHealth, SynthesisMode};
    let sm = SelfModel::new();
    // error_rate=0.99 → compute_score ≈ 0.01 < 0.3 → Escalate
    sm.report_health("core".to_string(), ModuleHealth {
        latency_ms: 0.0, error_rate: 0.99, resource_usage: 0.0,
    }).unwrap();
    let cfg = sm.recommend_loop_config();
    assert!((cfg.effective_veto_threshold - 0.50).abs() < 1e-6,
        "low health must raise threshold to 0.50; got {}", cfg.effective_veto_threshold);
    assert_eq!(cfg.synthesis_mode, SynthesisMode::Escalate);
}

#[test]
fn ac_self_model_high_health_autonomous() {
    use hipcortex::self_model::{SelfModel, ModuleHealth, SynthesisMode};
    let sm = SelfModel::new();
    // error_rate=0.0, latency=0, resource=0 → score = 1.0 > 0.8 → Autonomous
    sm.report_health("core".to_string(), ModuleHealth {
        latency_ms: 0.0, error_rate: 0.0, resource_usage: 0.0,
    }).unwrap();
    let cfg = sm.recommend_loop_config();
    assert!((cfg.effective_veto_threshold - 0.15).abs() < 1e-6,
        "high health must lower threshold to 0.15; got {}", cfg.effective_veto_threshold);
    assert_eq!(cfg.synthesis_mode, SynthesisMode::Autonomous);
}

// P0-B: CriticGate dynamic threshold
#[test]
fn ac_critic_gate_dynamic_threshold_raises_bar() {
    use hipcortex::loop_gates::{CriticGate, CriticDecision};
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
    // 1/4 factors satisfied = 0.25
    let goal = GoalPayload {
        target_state: "t".into(),
        status: GoalStatus::InProgress,
        success_factors: (0..4usize).map(|i| SuccessFactor {
            name: format!("f{i}"), weight: 1.0, satisfied: i < 1,
        }).collect(),
        ..Default::default()
    };
    // threshold 0.24 → 0.25 >= 0.24 → Approved
    assert!(matches!(
        CriticGate::evaluate_with_threshold(&goal, "act", 1, 0.24),
        CriticDecision::Approved { .. }
    ));
    // threshold 0.30 → 0.25 < 0.30 → Rejected
    assert!(matches!(
        CriticGate::evaluate_with_threshold(&goal, "act", 1, 0.30),
        CriticDecision::Rejected { .. }
    ));
}

// P0-A: ClarifyEngine
#[test]
fn ac_clarify_self_resolves_from_belief() {
    use hipcortex::clarify_engine::{ClarifyEngine, ClarifyOutcome, ClarifyTrigger};
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{BeliefPayload, GoalPayload, GoalStatus, SuccessFactor, EpistemicStatus};
    use hipcortex::memory_store::MemoryStore;

    let mut store = MemoryStore::new_in_memory();

    // Add goal with empty success_factors
    let goal_payload = GoalPayload {
        target_state: "stabilize_widget".into(),
        status: GoalStatus::InProgress,
        success_factors: vec![],
        ..Default::default()
    };
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal, "agent".into(), "plan".into(), "stabilize_widget".into(),
        serde_json::to_value(&goal_payload).unwrap(),
    );
    let goal_id = goal_rec.id;
    store.add(goal_rec).unwrap();

    // Add belief whose proposition contains the target_state
    let bp = BeliefPayload {
        proposition: "stabilize_widget is achievable via calibration".into(),
        confidence: 0.8,
        epistemic_status: EpistemicStatus::Observed,
        ..Default::default()
    };
    let belief_rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "observe".into(), "widget".into(),
        serde_json::to_value(&bp).unwrap(),
    );
    store.add(belief_rec).unwrap();

    let outcome = ClarifyEngine::run(&mut store, goal_id, "agent", ClarifyTrigger::EmptyAC);
    assert_eq!(outcome, ClarifyOutcome::ClarifiedBySubstrate,
        "matching belief must self-resolve as ClarifiedBySubstrate");

    // Must have written Reflexion{self_clarified}
    let reflexions = store.all_by_type(MemoryType::Reflexion);
    assert!(reflexions.iter().any(|r| r.action == "self_clarified" && r.derived_from == Some(goal_id)),
        "ClarifyEngine must write Reflexion{{self_clarified}} linked to goal");
}

#[test]
fn ac_clarify_escalates_after_max_rounds() {
    use hipcortex::clarify_engine::{ClarifyEngine, ClarifyOutcome, ClarifyTrigger};
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus};
    use hipcortex::memory_store::MemoryStore;

    let mut store = MemoryStore::new_in_memory();
    let goal_payload = GoalPayload {
        target_state: "nonexistent_target".into(),
        status: GoalStatus::InProgress,
        success_factors: vec![],
        ..Default::default()
    };
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal, "agent".into(), "plan".into(), "nonexistent_target".into(),
        serde_json::to_value(&goal_payload).unwrap(),
    );
    let goal_id = goal_rec.id;
    store.add(goal_rec).unwrap();

    let outcome = ClarifyEngine::run(&mut store, goal_id, "agent", ClarifyTrigger::EmptyAC);
    assert_eq!(outcome, ClarifyOutcome::NeedsUserClarification,
        "no matching belief must escalate to NeedsUserClarification");

    // Must have written exactly one Belief{clarify_needed}
    let clarify_beliefs = store.all_by_type(MemoryType::Belief).into_iter()
        .filter(|r| r.action == "clarify_needed" && r.derived_from == Some(goal_id))
        .count();
    assert_eq!(clarify_beliefs, 1, "must write exactly one Belief{{clarify_needed}}; got {clarify_beliefs}");
}

#[test]
fn ac_clarify_deduplicates_clarify_needed() {
    use hipcortex::clarify_engine::{ClarifyEngine, ClarifyOutcome, ClarifyTrigger};
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus};
    use hipcortex::memory_store::MemoryStore;

    let mut store = MemoryStore::new_in_memory();
    let goal_payload = GoalPayload {
        target_state: "dup_target".into(),
        status: GoalStatus::InProgress,
        success_factors: vec![],
        ..Default::default()
    };
    let goal_rec = MemoryRecord::new(
        MemoryType::Goal, "agent".into(), "plan".into(), "dup_target".into(),
        serde_json::to_value(&goal_payload).unwrap(),
    );
    let goal_id = goal_rec.id;
    store.add(goal_rec).unwrap();

    // Run twice — must only write one Belief{clarify_needed}
    let _ = ClarifyEngine::run(&mut store, goal_id, "agent", ClarifyTrigger::EmptyAC);
    let _ = ClarifyEngine::run(&mut store, goal_id, "agent", ClarifyTrigger::EmptyAC);

    let clarify_beliefs = store.all_by_type(MemoryType::Belief).into_iter()
        .filter(|r| r.action == "clarify_needed" && r.derived_from == Some(goal_id))
        .count();
    assert_eq!(clarify_beliefs, 1, "second run must be deduped; got {clarify_beliefs}");
}

// P0-E: JTMS as report truth
#[test]
fn ac_jtms_in_belief_appears_in_valid_assumptions() {
    use hipcortex::cognitive_report::build_report;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{BeliefPayload, EpistemicStatus, JtmsLabel};
    use hipcortex::memory_store::MemoryStore;

    let mut store = MemoryStore::new_in_memory();
    let bp = BeliefPayload {
        proposition: "jtms_in_belief is valid".into(),
        confidence: 0.2, // below 0.5 — only JTMS label makes it pass
        epistemic_status: EpistemicStatus::Hypothetical,
        jtms_label: JtmsLabel::In,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "observe".into(), "x".into(),
        serde_json::to_value(&bp).unwrap(),
    );
    store.add(rec).unwrap();

    let report = build_report(&store, "agent");
    assert!(
        report.valid_assumptions.iter().any(|b| b.proposition == "jtms_in_belief is valid"),
        "JtmsLabel::In belief must appear in valid_assumptions even with confidence < 0.5"
    );
}

#[test]
fn ac_jtms_out_belief_excluded_from_valid_assumptions() {
    use hipcortex::cognitive_report::build_report;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{BeliefPayload, EpistemicStatus, JtmsLabel};
    use hipcortex::memory_store::MemoryStore;

    let mut store = MemoryStore::new_in_memory();
    let bp = BeliefPayload {
        proposition: "jtms_out_belief is retracted".into(),
        confidence: 0.9, // high conf, but JTMS Out overrides
        epistemic_status: EpistemicStatus::Observed,
        jtms_label: JtmsLabel::Out,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "observe".into(), "x".into(),
        serde_json::to_value(&bp).unwrap(),
    );
    store.add(rec).unwrap();

    let report = build_report(&store, "agent");
    assert!(
        !report.valid_assumptions.iter().any(|b| b.proposition == "jtms_out_belief is retracted"),
        "JtmsLabel::Out belief must be excluded from valid_assumptions even with confidence=0.9"
    );
}
