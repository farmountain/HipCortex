//! Acceptance suite v2.2.0 — Epistemic Filter Closure
//! AC-Q2:    Q2 (learned_beliefs) requires JtmsLabel::In AND confidence > 0.3; Out excluded.
//! AC-Q8:    Q8 (uncertain_beliefs) includes JtmsLabel::Unknown regardless of confidence.
//! AC-VM1:   VerifierGate::check_and_record writes Temporal on mismatch (Q2-visible).
//! AC-VM2:   VerifierGate::check_and_record Consistent → no Temporal written.
//! AC-3M-SBS: StepByStep InProgress goal survives JSONL restart (execution_mode preserved).
//! AC-3M-Q2:  Q2 JTMS filter correct after JSONL reload — Out excluded, In included.

use std::process;
use tempfile::TempDir;

macro_rules! ac {
    ($label:expr, $body:block) => {{
        let result = std::panic::catch_unwind(|| $body);
        match result {
            Ok(_) => println!("[PASS] {}", $label),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                eprintln!("[FAIL] {} — {}", $label, msg);
                process::exit(1);
            }
        }
    }};
}

fn temp_path(dir: &TempDir, name: &str) -> String {
    dir.path().join(name).to_string_lossy().into_owned()
}

fn main() {
    use hipcortex::cognitive_report::build_report;
    use hipcortex::loop_gates::{VerifierGate, VerifierResult};
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{BeliefPayload, JtmsLabel};

    // ── AC-Q2 ───────────────────────────────────────────────────────────────
    ac!("AC-Q2 Q2 learned_beliefs: Out excluded (high-conf), In included (conf>0.3)", {
        let mut store = MemoryStore::new_in_memory();

        // Out belief at high confidence — must NOT appear in Q2 (split-state prevention).
        let bp_out = BeliefPayload {
            proposition: "server_offline".into(),
            confidence: 0.85,
            jtms_label: JtmsLabel::Out,
            ..Default::default()
        };
        let rec_out = MemoryRecord::new(
            MemoryType::Belief, "agent".into(), "assert".into(), "server_offline".into(),
            serde_json::to_value(&bp_out).unwrap(),
        );
        let out_id = rec_out.id;
        store.add(rec_out).unwrap();

        // In belief at moderate confidence — MUST appear in Q2.
        let bp_in = BeliefPayload {
            proposition: "service_healthy".into(),
            confidence: 0.55,
            jtms_label: JtmsLabel::In,
            ..Default::default()
        };
        let rec_in = MemoryRecord::new(
            MemoryType::Belief, "agent".into(), "assert".into(), "service_healthy".into(),
            serde_json::to_value(&bp_in).unwrap(),
        );
        let in_id = rec_in.id;
        store.add(rec_in).unwrap();

        let report = build_report(&store, "agent", 1.0);

        assert!(
            report.learned_beliefs.iter().any(|b| b.id == in_id),
            "Q2 must include In+conf>0.3 belief; got: {:?}",
            report.learned_beliefs.iter().map(|b| b.proposition.clone()).collect::<Vec<_>>()
        );
        assert!(
            !report.learned_beliefs.iter().any(|b| b.id == out_id),
            "Q2 must exclude Out belief even at conf=0.85; got: {:?}",
            report.learned_beliefs.iter().map(|b| b.proposition.clone()).collect::<Vec<_>>()
        );
    });

    // ── AC-Q8 ───────────────────────────────────────────────────────────────
    ac!("AC-Q8 Q8 uncertain_beliefs: Unknown included regardless of confidence", {
        let mut store = MemoryStore::new_in_memory();

        // Unknown belief at conf=0.72 — above old 0.6 raw cutoff.
        // Must appear in Q8 because Unknown is first-class uncertain.
        let bp_unk = BeliefPayload {
            proposition: "cache_state_unclear".into(),
            confidence: 0.72,
            jtms_label: JtmsLabel::Unknown,
            ..Default::default()
        };
        let rec_unk = MemoryRecord::new(
            MemoryType::Belief, "agent".into(), "assert".into(), "cache_state_unclear".into(),
            serde_json::to_value(&bp_unk).unwrap(),
        );
        let unk_id = rec_unk.id;
        store.add(rec_unk).unwrap();

        // In belief at conf=0.9 — must NOT appear in Q8 (confident + authoritative).
        let bp_in = BeliefPayload {
            proposition: "db_connected".into(),
            confidence: 0.9,
            jtms_label: JtmsLabel::In,
            ..Default::default()
        };
        let rec_in = MemoryRecord::new(
            MemoryType::Belief, "agent".into(), "assert".into(), "db_connected".into(),
            serde_json::to_value(&bp_in).unwrap(),
        );
        let in_id = rec_in.id;
        store.add(rec_in).unwrap();

        let report = build_report(&store, "agent", 1.0);

        assert!(
            report.open_uncertainties.uncertain_beliefs.iter().any(|b| b.id == unk_id),
            "Q8 must include Unknown belief at conf=0.72 (above raw cutoff); got: {:?}",
            report.open_uncertainties.uncertain_beliefs.iter().map(|b| b.proposition.clone()).collect::<Vec<_>>()
        );
        assert!(
            !report.open_uncertainties.uncertain_beliefs.iter().any(|b| b.id == in_id),
            "Q8 must exclude high-conf In belief; got: {:?}",
            report.open_uncertainties.uncertain_beliefs.iter().map(|b| b.proposition.clone()).collect::<Vec<_>>()
        );
    });

    // ── AC-VM1 ──────────────────────────────────────────────────────────────
    ac!("AC-VM1 check_and_record mismatch → Temporal{verifier_mismatch_observed} written", {
        let mut store = MemoryStore::new_in_memory();

        let result = VerifierGate::check_and_record(
            &mut store,
            Some("goal_advancing"),
            "goal_stalling",
            "test_agent",
            None,
        );
        assert!(
            matches!(result, VerifierResult::Mismatch { .. }),
            "expected Mismatch result"
        );

        let temporals = store.all_by_type(MemoryType::Temporal);
        let written = temporals.iter().find(|r| r.action == "verifier_mismatch_observed");
        assert!(
            written.is_some(),
            "check_and_record must write Temporal{{verifier_mismatch_observed}} on mismatch"
        );
        assert_eq!(
            written.unwrap().actor,
            "test_agent",
            "Temporal actor must match caller"
        );
    });

    // ── AC-VM2 ──────────────────────────────────────────────────────────────
    ac!("AC-VM2 check_and_record consistent → no Temporal written", {
        let mut store = MemoryStore::new_in_memory();

        let result = VerifierGate::check_and_record(
            &mut store,
            Some("goal_advancing"),
            "goal_advancing",
            "test_agent",
            None,
        );
        assert!(
            matches!(result, VerifierResult::Consistent),
            "expected Consistent result"
        );

        let temporals = store.all_by_type(MemoryType::Temporal);
        assert!(
            temporals.is_empty(),
            "check_and_record must not write Temporal on Consistent; got {} records",
            temporals.len()
        );
    });

    // ── AC-3M-SBS ────────────────────────────────────────────────────────────
    ac!("AC-3M-SBS StepByStep InProgress goal survives JSONL restart (execution_mode preserved)", {
        use hipcortex::payloads::{GoalExecutionMode, GoalStatus, GoalPayload, SuccessFactor};

        let dir = TempDir::new().unwrap();
        let path = temp_path(&dir, "sbs_mem.jsonl");

        let gp = GoalPayload {
            target_state: "deploy_v2".to_string(),
            status: GoalStatus::InProgress,
            execution_mode: GoalExecutionMode::StepByStep,
            current_iteration: 2,
            success_factors: vec![
                SuccessFactor { name: "tests_pass".to_string(), weight: 1.0, satisfied: true },
                SuccessFactor { name: "deployed".to_string(), weight: 1.0, satisfied: false },
            ],
            ..Default::default()
        };
        {
            let mut s = MemoryStore::new(&path).unwrap();
            s.add(MemoryRecord::new(
                MemoryType::Goal, "agent".into(), "pursue".into(),
                "deploy_v2".into(), serde_json::to_value(&gp).unwrap(),
            )).unwrap();
        }

        let s = MemoryStore::new(&path).unwrap();
        let goals = s.search_by_goal_status("agent", "InProgress");
        assert!(!goals.is_empty(), "StepByStep goal must survive JSONL restart");

        let reloaded_gp: GoalPayload =
            serde_json::from_value(goals[0].metadata.clone()).unwrap();
        assert_eq!(reloaded_gp.execution_mode, GoalExecutionMode::StepByStep,
            "execution_mode must be StepByStep after reload");
        assert_eq!(reloaded_gp.current_iteration, 2,
            "current_iteration must survive reload");
        assert_eq!(reloaded_gp.success_factors.iter().filter(|f| f.satisfied).count(), 1,
            "satisfied factor count must survive reload");
    });

    // ── AC-3M-Q2 ─────────────────────────────────────────────────────────────
    ac!("AC-3M-Q2 Q2 JTMS filter correct after JSONL reload (Out excluded, In included)", {
        use hipcortex::payloads::EpistemicStatus;

        let dir = TempDir::new().unwrap();
        let path = temp_path(&dir, "q2_mem.jsonl");

        // Write: Out belief at high conf + In belief at moderate conf
        {
            let mut s = MemoryStore::new(&path).unwrap();

            let bp_out = BeliefPayload {
                proposition: "legacy_system_active".into(),
                confidence: 0.88,
                jtms_label: JtmsLabel::Out,
                epistemic_status: EpistemicStatus::Observed,
                ..Default::default()
            };
            s.add(MemoryRecord::new(
                MemoryType::Belief, "agent".into(), "assert".into(),
                "legacy_system_active".into(), serde_json::to_value(&bp_out).unwrap(),
            )).unwrap();

            let bp_in = BeliefPayload {
                proposition: "new_system_ready".into(),
                confidence: 0.65,
                jtms_label: JtmsLabel::In,
                epistemic_status: EpistemicStatus::Observed,
                ..Default::default()
            };
            s.add(MemoryRecord::new(
                MemoryType::Belief, "agent".into(), "assert".into(),
                "new_system_ready".into(), serde_json::to_value(&bp_in).unwrap(),
            )).unwrap();
        }

        // Reload and run cognitive_report
        let s = MemoryStore::new(&path).unwrap();
        let report = build_report(&s, "agent", 1.0);

        let in_in_q2 = report.learned_beliefs.iter()
            .any(|b| b.proposition == "new_system_ready");
        let out_in_q2 = report.learned_beliefs.iter()
            .any(|b| b.proposition == "legacy_system_active");

        assert!(in_in_q2,
            "Q2 must include In+conf>0.3 belief after JSONL reload; got: {:?}",
            report.learned_beliefs.iter().map(|b| b.proposition.clone()).collect::<Vec<_>>());
        assert!(!out_in_q2,
            "Q2 must exclude Out belief (conf=0.88) after JSONL reload; got: {:?}",
            report.learned_beliefs.iter().map(|b| b.proposition.clone()).collect::<Vec<_>>());
    });

    println!("\n=== Acceptance v2.2.0 (Epistemic Filter Closure): 6/6 passed ===");
}
