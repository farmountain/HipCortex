//! Acceptance suite v2.1.0 — Cognitive Substrate Coherence
//! AC-SC1: Induced Skill has non-empty preconditions extracted from real records
//! AC-SC2: BeliefExecutive.decay below threshold → JtmsLabel::Out (atomic, no split state)
//! AC-SC3: ClarifyEngine detects env-blocked factor → rewrites success_factors + goal_restated

use std::process;

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

fn main() {
    use hipcortex::belief_executive::BeliefExecutive;
    use hipcortex::clarify_engine::{ClarifyEngine, ClarifyOutcome, ClarifyTrigger};
    use hipcortex::consolidation::{mine_causal_motifs, induce_skill_record};
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{BeliefPayload, GoalPayload, JtmsLabel, SkillPayload, SuccessFactor, GoalStatus};

    // ── AC-SC1 ──────────────────────────────────────────────────────────────
    ac!("AC-SC1 Induced Skill has real preconditions + expected_outcomes (not placeholder)", {
        let mut store = MemoryStore::new_in_memory();

        // Build 2 identical causal chains
        let mut prev = None;
        for _ in 0..2 {
            let a = {
                let mut r = MemoryRecord::new(
                    MemoryType::Temporal, "agent".into(), "observe".into(),
                    "sensor_data".into(), serde_json::json!({}),
                );
                r.derived_from = None;
                let id = r.id;
                store.add(r).unwrap();
                id
            };
            let b = {
                let mut r = MemoryRecord::new(
                    MemoryType::Temporal, "agent".into(), "reflect".into(),
                    "pattern_found".into(), serde_json::json!({}),
                );
                r.derived_from = Some(a);
                let id = r.id;
                store.add(r).unwrap();
                id
            };
            prev = Some(b);
        }
        let _ = prev;

        let motifs = mine_causal_motifs(&store, 2, 2, 5);
        assert!(!motifs.is_empty(), "expected motif");

        let skill = induce_skill_record(&motifs[0], "agent", &store);
        let p: SkillPayload = serde_json::from_value(skill.metadata.clone()).unwrap();

        assert!(!p.preconditions.is_empty(),
            "preconditions must be non-empty, got: {:?}", p.preconditions);
        assert!(!p.expected_outcomes.is_empty(),
            "expected_outcomes must be non-empty, got: {:?}", p.expected_outcomes);
        assert!(
            !p.expected_outcomes[0].contains("pattern repeats"),
            "must not be old placeholder: {:?}", p.expected_outcomes
        );
        // Preconditions must mention the first action's context
        let prec_text = p.preconditions.join(" ");
        assert!(
            prec_text.contains("observe") || prec_text.contains("requires"),
            "preconditions must mention first action: {:?}", p.preconditions
        );
    });

    // ── AC-SC2 ──────────────────────────────────────────────────────────────
    ac!("AC-SC2 BeliefExecutive.decay below threshold → JtmsLabel::Out (atomic)", {
        let mut store = MemoryStore::new_in_memory();

        let bp = BeliefPayload { proposition: "all systems go".into(), confidence: 0.9, ..Default::default() };
        let mut rec = MemoryRecord::new(
            MemoryType::Belief, "agent".into(), "assert".into(), "all systems go".into(),
            serde_json::to_value(&bp).unwrap(),
        );
        rec.confidence = 0.9;
        let id = rec.id;
        store.add(rec).unwrap();

        // Decay below 0.2 threshold
        let retracted = BeliefExecutive::decay(&mut store, id, 0.05);

        // Both confidence and label must agree — no split state
        let stored = store.find_by_id(id).unwrap();
        let p: BeliefPayload = serde_json::from_value(stored.metadata.clone()).unwrap();

        assert!(stored.confidence < 0.2,
            "confidence must be below threshold, got {}", stored.confidence);
        assert_eq!(p.jtms_label, JtmsLabel::Out,
            "JtmsLabel must be Out when confidence < ARCHIVE_THRESHOLD");
        assert!(!retracted.is_empty(), "retracted IDs must be returned");
        assert!(retracted.contains(&id));
    });

    // ── AC-SC3 ──────────────────────────────────────────────────────────────
    ac!("AC-SC3 ClarifyEngine detects env-blocked factor → rewrites + goal_restated Reflexion", {
        let mut store = MemoryStore::new_in_memory();

        // Create goal with a success_factor that references "deploy_production"
        let sf = vec![SuccessFactor { name: "deploy_production".into(), weight: 1.0, satisfied: false }];
        let gp = GoalPayload {
            target_state: "deploy".into(),
            success_factors: sf,
            status: GoalStatus::InProgress,
            ..Default::default()
        };
        let goal_rec = MemoryRecord::new(
            MemoryType::Temporal, "agent".into(), "goal".into(), "deploy".into(),
            serde_json::to_value(&gp).unwrap(),
        );
        let goal_id = goal_rec.id;
        store.add(goal_rec).unwrap();

        // Inject env failure signal: production failed/offline
        let env = MemoryRecord::new(
            MemoryType::Temporal, "env".into(), "failed".into(),
            "deploy_production".into(), serde_json::json!({}),
        );
        store.add(env).unwrap();

        // ClarifyEngine must detect env block and restate
        let outcome = ClarifyEngine::run(&mut store, goal_id, "agent", ClarifyTrigger::EmptyAC, None);
        assert_eq!(outcome, ClarifyOutcome::ClarifiedBySubstrate,
            "env-blocked goal must return ClarifiedBySubstrate, got {:?}", outcome);

        // Factor must be renamed to *_when_available
        let updated_rec = store.find_by_id(goal_id).unwrap();
        let updated_gp: GoalPayload = serde_json::from_value(updated_rec.metadata.clone()).unwrap();
        assert!(
            updated_gp.success_factors[0].name.ends_with("_when_available"),
            "factor must be renamed, got: {}", updated_gp.success_factors[0].name
        );

        // Reflexion{goal_restated} must be written with derived_from = goal_id
        let reflexions = store.all_by_type(MemoryType::Reflexion);
        let restated = reflexions.iter().find(|r| r.action == "goal_restated");
        assert!(restated.is_some(), "must write Reflexion{{goal_restated}}");
        assert_eq!(restated.unwrap().derived_from, Some(goal_id),
            "Reflexion must be linked to goal_id");
    });

    println!("\n=== Acceptance v2.1.0 (Cognitive Substrate Coherence): 3/3 passed ===");
}
