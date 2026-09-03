/// v1.9.0 Acceptance Suite — 3-Month Agent Coherence
/// harness=false binary; all ACs machine-verifiable.
/// Run: cargo test --no-default-features --features "petgraph_backend" --test acceptance_suite_v190

use hipcortex::cognitive_report::build_report;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{
    BeliefPayload, EpistemicStatus, GoalPayload, GoalStatus, JtmsLabel, SkillPayload,
    SuccessFactor,
};
use hipcortex::world_model_enhanced::entity::{EntityObservation, EntityState};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::time::Instant;
use tempfile::TempDir;

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

fn temp_path(dir: &TempDir, name: &str) -> String {
    dir.path().join(name).to_string_lossy().into_owned()
}

fn main() {
    let results: &[bool] = &[

    // AC-R1: InProgress goal survives process restart (JSONL persistence)
    ac!("AC-R1 InProgress goal persists across restart", {
        let dir = TempDir::new().unwrap();
        let path = temp_path(&dir, "mem.jsonl");
        let gp = GoalPayload {
            target_state: "ac_r1_goal".to_string(),
            status: GoalStatus::InProgress,
            success_factors: vec![SuccessFactor {
                name: "done".to_string(), weight: 1.0, satisfied: false,
            }],
            ..Default::default()
        };
        {
            let mut s = MemoryStore::new(&path).unwrap();
            s.add(MemoryRecord::new(MemoryType::Goal, "agent".into(), "pursue".into(),
                "ac_r1_goal".into(), serde_json::to_value(&gp).unwrap())).unwrap();
        }
        let s = MemoryStore::new(&path).unwrap();
        let goals = s.search_by_goal_status("agent", "InProgress");
        assert!(!goals.is_empty(), "InProgress goal must survive restart");
        assert_eq!(goals[0].target, "ac_r1_goal");
    }),

    // AC-R2: JTMS-In belief label preserved after restart
    ac!("AC-R2 JTMS-In belief label survives restart", {
        let dir = TempDir::new().unwrap();
        let path = temp_path(&dir, "mem.jsonl");
        let bp = BeliefPayload {
            proposition: "sky_is_blue".to_string(),
            confidence: 0.95,
            jtms_label: JtmsLabel::In,
            epistemic_status: EpistemicStatus::Observed,
            ..Default::default()
        };
        {
            let mut s = MemoryStore::new(&path).unwrap();
            s.add(MemoryRecord::new(MemoryType::Belief, "agent".into(), "assert".into(),
                "sky_is_blue".into(), serde_json::to_value(&bp).unwrap())).unwrap();
        }
        let s = MemoryStore::new(&path).unwrap();
        let report = build_report(&s, "agent", 0.9);
        let entry = report.valid_assumptions.iter()
            .find(|b| b.proposition == "sky_is_blue");
        assert!(entry.is_some(), "JTMS-In belief must appear in valid_assumptions after restart");
        assert!(!entry.unwrap().epistemic_status.starts_with("Provisional"),
            "JTMS-In must NOT be Provisional");
    }),

    // AC-R3: Unknown+0.5 belief appears as Provisional after restart
    ac!("AC-R3 Provisional belief tagged correctly after restart", {
        let dir = TempDir::new().unwrap();
        let path = temp_path(&dir, "mem.jsonl");
        let bp = BeliefPayload {
            proposition: "cache_warm_maybe".to_string(),
            confidence: 0.6,
            jtms_label: JtmsLabel::Unknown,
            epistemic_status: EpistemicStatus::Hypothetical,
            ..Default::default()
        };
        {
            let mut s = MemoryStore::new(&path).unwrap();
            s.add(MemoryRecord::new(MemoryType::Belief, "agent".into(), "assert".into(),
                "cache_warm_maybe".into(), serde_json::to_value(&bp).unwrap())).unwrap();
        }
        let s = MemoryStore::new(&path).unwrap();
        let report = build_report(&s, "agent", 0.9);
        let entry = report.valid_assumptions.iter()
            .find(|b| b.proposition == "cache_warm_maybe");
        assert!(entry.is_some(), "Unknown+0.5 belief must survive restart into valid_assumptions");
        assert!(entry.unwrap().epistemic_status.starts_with("Provisional"),
            "Unknown+0.5 must be tagged Provisional after restart");
    }),

    // AC-R4: Skill abstraction persists across restart
    ac!("AC-R4 Skill abstraction survives restart into emergent_abstractions", {
        let dir = TempDir::new().unwrap();
        let path = temp_path(&dir, "mem.jsonl");
        let sp = SkillPayload {
            procedure: "backoff_retry".to_string(),
            preconditions: vec![],
            expected_outcomes: vec!["success".to_string()],
        };
        {
            let mut s = MemoryStore::new(&path).unwrap();
            s.add(MemoryRecord::new(MemoryType::Skill, "agent".into(), "induced".into(),
                "backoff_retry".into(), serde_json::to_value(&sp).unwrap())).unwrap();
        }
        let s = MemoryStore::new(&path).unwrap();
        let report = build_report(&s, "agent", 0.9);
        let found = report.emergent_abstractions.iter()
            .any(|b| b.proposition.contains("backoff_retry") && b.epistemic_status == "Skill");
        assert!(found, "Skill must appear in emergent_abstractions after restart");
    }),

    // AC-R5: WorldModel transition data survives save+load cycle
    ac!("AC-R5 WM transitions survive save/load (restart)", {
        let dir = TempDir::new().unwrap();
        let wm_path = temp_path(&dir, "wm.json");
        {
            let wm = WorldModelEnhanced::new();
            wm.observe_transition("s0".to_string(), "act".to_string(), "s1".to_string());
            wm.save(&wm_path).unwrap();
        }
        let wm = WorldModelEnhanced::load(&wm_path).expect("WM load must succeed");
        let unc = wm.get_transition_uncertainty("s0", "act");
        assert!(unc.is_ok(), "WM transition data must survive restart: {:?}", unc);
    }),

    // AC-O1: OOD anomaly detected when entity observation far from Kalman mean
    ac!("AC-O1 OOD anomaly severity > threshold for far observation", {
        let wm = WorldModelEnhanced::new();
        let init = EntityState { properties: vec![0.0], covariance: vec![vec![0.1]] };
        wm.register_entity("sensor".to_string(), init).unwrap();
        // Warm up Kalman filter near mean
        for _ in 0..5 {
            let obs = EntityObservation {
                measured_properties: vec![0.05],
                measurement_noise: vec![vec![0.1]],
                timestamp: Instant::now(),
            };
            wm.update_entity("sensor", obs).unwrap();
        }
        // Far observation — distribution shift
        let ood = EntityObservation {
            measured_properties: vec![50.0],
            measurement_noise: vec![vec![0.1]],
            timestamp: Instant::now(),
        };
        wm.update_entity("sensor", ood).unwrap();
        let anomalies = wm.get_entity_anomalies("sensor").unwrap();
        assert!(!anomalies.is_empty(), "OOD observation must produce Anomaly records");
        let triggered = anomalies.iter().any(|a| a.severity > a.threshold);
        assert!(triggered,
            "OOD anomaly severity must exceed threshold; got severity={:.2} threshold={:.2}",
            anomalies[0].severity, anomalies[0].threshold);
    }),

    // AC-O2: Normal observations within threshold (targeted — not always-on noise)
    ac!("AC-O2 Normal observations do NOT exceed anomaly threshold", {
        let wm = WorldModelEnhanced::new();
        let init = EntityState { properties: vec![0.0], covariance: vec![vec![1.0]] };
        wm.register_entity("stable".to_string(), init).unwrap();
        for v in [0.1_f64, -0.1, 0.2, -0.2, 0.05] {
            let obs = EntityObservation {
                measured_properties: vec![v],
                measurement_noise: vec![vec![1.0]],
                timestamp: Instant::now(),
            };
            wm.update_entity("stable", obs).unwrap();
        }
        let anomalies = wm.get_entity_anomalies("stable").unwrap_or_default();
        let over = anomalies.iter().filter(|a| a.severity > a.threshold).count();
        assert_eq!(over, 0,
            "Normal observations must not exceed threshold; {} exceeded", over);
    }),

    ];

    let passed = results.iter().filter(|&&b| b).count();
    let failed = results.iter().filter(|&&b| !b).count();
    println!("\n=== Acceptance v1.9.0 (3-Month Agent): {}/{} passed ===", passed, passed + failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
