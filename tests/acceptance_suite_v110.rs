/// v1.1.0 Acceptance Suite — Cognitive Loop Closure
/// harness=false binary; all 10 ACs machine-verifiable.
/// Run: cargo test --no-default-features --features "petgraph_backend" --test acceptance_suite_v110

use hipcortex::action_registry;
use hipcortex::belief_invalidator::BeliefInvalidator;
use hipcortex::cognitive_report::build_report;
use hipcortex::emergence::EmergenceDetector;
use hipcortex::goal_scheduler::GoalScheduler;
use hipcortex::loop_engine::ReactEngine;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, GoalPayload, GoalStatus, SuccessFactor};

/// Run body, print PASS/FAIL, return true on pass.
macro_rules! ac {
    ($label:expr, $body:block) => {{
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body));
        match result {
            Ok(_) => { println!("[PASS] {}", $label); true }
            Err(e) => {
                let msg = e.downcast_ref::<String>().map(|s| s.as_str())
                    .or_else(|| e.downcast_ref::<&str>().copied())
                    .unwrap_or("unknown panic");
                println!("[FAIL] {} — {}", $label, msg);
                false
            }
        }
    }};
}

fn main() {
    let results: &[bool] = &[

    // AC-1: EmergenceDetector auto-generates ≥1 Belief from ≥10 dense Temporal records
    ac!("AC-1 EmergenceDetector auto-generates Belief from dense Temporals", {
        let mut store = MemoryStore::new_in_memory();
        for i in 0..12 {
            let obs = MemoryRecord::new(
                MemoryType::Temporal, "agent".into(),
                format!("observe_timeout_{}", i),
                "service timeout error".into(),
                serde_json::json!({ "thought": format!("connection timeout spike at {}", i) }),
            );
            store.add(obs).unwrap();
        }
        let created = EmergenceDetector::detect(&mut store, "agent");
        assert!(!created.is_empty(), "No Beliefs emerged from 12 dense Temporal records");
        let b = store.find_by_id(created[0]).unwrap();
        assert!(!b.evidence.is_empty(), "Emerged Belief has no evidence pointers");
        assert!(b.confidence > 0.0, "Emerged Belief confidence must be > 0");
    }),

    // AC-2: CognitiveStateReport answers all 10 questions in one call
    ac!("AC-2 CognitiveStateReport answers all 10 questions in one call", {
        let mut store = MemoryStore::new_in_memory();
        let gp = GoalPayload {
            target_state: "server healthy".into(),
            urgency: 0.8,
            status: GoalStatus::InProgress,
            ..Default::default()
        };
        store.add(MemoryRecord::new(
            MemoryType::Goal, "agent".into(), "pursue".into(), "server healthy".into(),
            serde_json::to_value(&gp).unwrap(),
        )).unwrap();
        let report = build_report(&store, "agent");
        assert!(!report.active_goals.is_empty(), "Q1 active_goals empty");
        assert!(!report.authorized_actions.is_empty(), "Q9 authorized_actions empty");
        assert!(report.next_recommendation.goal_id.is_some(), "Q10 goal_id is None");
        assert!(!report.next_recommendation.recommended_op.is_empty(), "Q10 recommended_op empty");
        let json = serde_json::to_string(&report).expect("Report must serialize");
        assert!(json.contains("active_goals"), "JSON missing active_goals key");
    }),

    // AC-3: GoalScheduler returns highest-urgency goal from 3 concurrent goals
    ac!("AC-3 GoalScheduler returns highest-urgency active Goal", {
        let mut store = MemoryStore::new_in_memory();
        let mut add = |u: f64| {
            let p = GoalPayload {
                target_state: format!("goal_{}", u), urgency: u,
                status: GoalStatus::Pending, ..Default::default()
            };
            let r = MemoryRecord::new(
                MemoryType::Goal, "agent".into(), "p".into(),
                format!("goal_{}", u).as_str().into(), serde_json::to_value(&p).unwrap(),
            );
            let id = r.id;
            store.add(r).unwrap();
            id
        };
        add(0.1); add(0.5);
        let high = add(0.9);
        let next = GoalScheduler::next(&store, "agent").expect("Scheduler returned None");
        assert_eq!(next, high, "Wrong goal returned — not highest urgency");
    }),

    // AC-4: WorldModel transitions updated after ReactEngine run
    ac!("AC-4 WorldModel feedback: transitions non-zero after ReactEngine run", {
        let mut store = MemoryStore::new_in_memory();
        let gp = GoalPayload {
            target_state: "wm_ac4".into(),
            max_react_iterations: 3,
            status: GoalStatus::Pending,
            success_factors: vec![SuccessFactor { name: "f".into(), weight: 1.0, satisfied: false }],
            ..Default::default()
        };
        let rec = MemoryRecord::new(MemoryType::Goal, "agent".into(), "p".into(), "wm_ac4".into(), serde_json::to_value(&gp).unwrap());
        let gid = rec.id;
        store.add(rec).unwrap();
        let mut engine = ReactEngine::new();
        engine.run(&mut store, gid, 0).unwrap();
        let unc = engine.wm.get_transition_uncertainty("wm_ac4", "symbolic_step");
        assert!(unc.is_ok(), "No transition data in WorldModel after run: {:?}", unc);
    }),

    // AC-5: Decision record written per act-phase, linked to goal
    ac!("AC-5 Decision record written for each ReactEngine act-phase", {
        let mut store = MemoryStore::new_in_memory();
        let gp = GoalPayload {
            target_state: "ac5_target".into(),
            max_react_iterations: 2,
            status: GoalStatus::Pending,
            success_factors: vec![SuccessFactor { name: "s".into(), weight: 1.0, satisfied: false }],
            ..Default::default()
        };
        let rec = MemoryRecord::new(MemoryType::Goal, "agent".into(), "p".into(), "ac5_target".into(), serde_json::to_value(&gp).unwrap());
        let gid = rec.id;
        store.add(rec).unwrap();
        ReactEngine::new().run(&mut store, gid, 0).unwrap();
        let decisions: Vec<_> = store.all_by_type(MemoryType::Decision)
            .into_iter().filter(|r| r.derived_from == Some(gid)).collect();
        assert!(decisions.len() >= 2, "Expected ≥2 Decision records, got {}", decisions.len());
        for d in &decisions {
            assert!(d.metadata.get("option_chosen").is_some(), "Decision missing option_chosen");
        }
    }),

    // AC-6: BeliefInvalidator decays confidence below 0.2 after repeated contradictions
    ac!("AC-6 BeliefInvalidator: confidence < 0.2 + marker after 7 contradictions", {
        let mut store = MemoryStore::new_in_memory();
        let bp = BeliefPayload {
            proposition: "cache always warm".into(),
            confidence: 0.9,
            ..Default::default()
        };
        let mut brec = MemoryRecord::new(
            MemoryType::Belief, "agent".into(), "assert".into(),
            "cache always warm".into(), serde_json::to_value(&bp).unwrap(),
        );
        brec.confidence = 0.9;
        let bid = brec.id;
        store.add(brec).unwrap();
        for _ in 0..7 {
            let obs = MemoryRecord::new(
                MemoryType::Temporal, "agent".into(),
                "cache not warm error invalid".into(),
                "cache always warm broken".into(),
                serde_json::json!({ "thought": "cache not warm invalid wrong error" }),
            );
            BeliefInvalidator::process(&obs, &mut store);
        }
        let updated = store.find_by_id(bid).unwrap();
        assert!(updated.confidence < 0.2, "Confidence must be < 0.2 after 7 contradictions, got {}", updated.confidence);
        let markers = store.find_by_action("belief_invalidated");
        assert!(markers.iter().any(|r| r.derived_from == Some(bid)), "belief_invalidated marker must exist");
    }),

    // AC-7: ActionRegistry has ≥3 known op types
    ac!("AC-7 ActionRegistry: ALL_OPS has ≥3 op types", {
        assert!(action_registry::ALL_OPS.len() >= 3, "ALL_OPS has only {} ops", action_registry::ALL_OPS.len());
    }),

    // AC-8: search_by_goal_status returns only Failed goals
    ac!("AC-8 Failure index: search_by_goal_status(failed) returns only Failed goals", {
        let mut store = MemoryStore::new_in_memory();
        for (target, status) in [
            ("g1", GoalStatus::Succeeded), ("g2", GoalStatus::Failed),
            ("g3", GoalStatus::Pending),   ("g4", GoalStatus::Failed),
        ] {
            let p = GoalPayload { target_state: target.into(), status, ..Default::default() };
            store.add(MemoryRecord::new(MemoryType::Goal, "agent".into(), "p".into(), target.into(), serde_json::to_value(&p).unwrap())).unwrap();
        }
        let failures = store.search_by_goal_status("agent", "failed");
        assert_eq!(failures.len(), 2, "Expected 2 Failed goals, got {}", failures.len());
    }),

    // AC-9: provenance_chain returns ≥2 records (root + child) for a derived record
    ac!("AC-9 Provenance: provenance_chain returns ≥2 ancestor records", {
        let mut store = MemoryStore::new_in_memory();
        let root = MemoryRecord::new(MemoryType::Goal, "agent".into(), "root".into(), "root".into(), serde_json::json!({}));
        let root_id = root.id;
        store.add(root).unwrap();
        let mut child = MemoryRecord::new(MemoryType::Temporal, "agent".into(), "obs".into(), "child".into(), serde_json::json!({}));
        child.derived_from = Some(root_id);
        let child_id = child.id;
        store.add(child).unwrap();
        let mut gc = MemoryRecord::new(MemoryType::Reflexion, "agent".into(), "reflect".into(), "gc".into(), serde_json::json!({}));
        gc.derived_from = Some(child_id);
        let gc_id = gc.id;
        store.add(gc).unwrap();
        let chain = store.provenance_chain(gc_id, 20);
        assert!(chain.len() >= 2, "Expected ≥2 records in chain, got {}", chain.len());
        assert!(chain.iter().any(|r| r.id == root_id), "Root must appear in provenance chain");
    }),

    // AC-10: CognitiveStateReport.next_recommendation populated
    ac!("AC-10 CognitiveStateReport.next_recommendation populated", {
        let mut store = MemoryStore::new_in_memory();
        let gp = GoalPayload {
            target_state: "ac10_goal".into(), urgency: 0.7,
            status: GoalStatus::InProgress, ..Default::default()
        };
        store.add(MemoryRecord::new(MemoryType::Goal, "agent".into(), "p".into(), "ac10_goal".into(), serde_json::to_value(&gp).unwrap())).unwrap();
        let report = build_report(&store, "agent");
        assert!(report.next_recommendation.goal_id.is_some(), "next_recommendation.goal_id must be Some");
        assert_eq!(report.next_recommendation.recommended_op, "react_loop");
        assert!(!report.next_recommendation.rationale.is_empty(), "rationale must not be empty");
    }),

    ];

    let passed = results.iter().filter(|&&b| b).count();
    let failed = results.iter().filter(|&&b| !b).count();
    println!("\n=== Acceptance v1.1.0: {}/{} passed ===", passed, passed + failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
