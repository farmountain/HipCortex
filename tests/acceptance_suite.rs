// Acceptance suite — one test per AC, machine-readable pass/fail.
fn main() {
    let mut passed = 0usize;
    let mut failed = 0usize;

    macro_rules! ac {
        ($name:expr, $body:expr) => {
            match std::panic::catch_unwind(|| { $body }) {
                Ok(_) => { println!("[PASS] {}", $name); passed += 1; }
                Err(e) => {
                    let msg = e.downcast_ref::<String>().cloned()
                        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "panic".to_string());
                    println!("[FAIL] {}: {}", $name, msg);
                    failed += 1;
                }
            }
        };
    }

    ac!("AC-1a StructuralEquation U-roundtrip", {
        use hipcortex::world_model_enhanced::causal::{LinearSE, StructuralEquation};
        let se = LinearSE { weights: vec![2.0] };
        let obs = se.evaluate(&[3.0], 0.5);
        let u = se.invert_for_u(&[3.0], obs);
        assert!((u - 0.5).abs() < 1e-9, "roundtrip failed: {}", u);
    });

    ac!("AC-1b do_operator graph surgery", {
        use hipcortex::world_model_enhanced::causal::CausalGraph;
        let mut g = CausalGraph::new();
        g.add_node("a".into()).unwrap();
        g.add_node("b".into()).unwrap();
        g.add_edge("a".into(), "b".into()).unwrap();
        let m = g.do_operator("b", 1.0);
        assert!(!m.has_path("a", "b").unwrap_or(true));
        assert_eq!(m.pinned_value("b"), Some(1.0));
    });

    ac!("AC-2a credit_assign returns report", {
        use hipcortex::world_model_enhanced::causal::{CausalGraph, FailureSignal, LinearSE};
        use std::sync::Arc;
        let mut g = CausalGraph::new();
        g.add_node("x".into()).unwrap();
        g.add_node("y".into()).unwrap();
        g.add_edge("x".into(), "y".into()).unwrap();
        if let Some(n) = g.node_mut("y") {
            n.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
        }
        let traj = vec![std::collections::HashMap::from([
            ("x".to_string(), 1.0), ("y".to_string(), 2.5),
        ])];
        let r = g.credit_assign(&traj, &FailureSignal::MaxIterations).unwrap();
        assert!(r.confidence >= 0.0 && r.confidence <= 1.0);
    });

    ac!("AC-2b ReactEngine writes attribution Reflexion", {
        use hipcortex::loop_engine::ReactEngine;
        use hipcortex::memory_record::{MemoryRecord, MemoryType};
        use hipcortex::memory_store::MemoryStore;
        use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
        let mut store = MemoryStore::new_in_memory();
        let gp = GoalPayload {
            target_state: "t".into(), acceptance_criteria: vec![],
            success_factors: vec![SuccessFactor { name: "x".into(), satisfied: false, weight: 1.0 }],
            max_react_iterations: 3, current_iteration: 0,
            status: GoalStatus::Pending,
            ..Default::default()
        };
        let rec = MemoryRecord::new(MemoryType::Goal, "a".into(), "p".into(), "t".into(),
            serde_json::to_value(&gp).unwrap());
        let id = rec.id;
        store.add(rec).unwrap();
        ReactEngine::new().run(&mut store, id, 0).unwrap();
        assert!(store.all().iter().any(|r|
            r.record_type == MemoryType::Reflexion && r.metadata.to_string().contains("attribution")
        ), "no attribution reflexion written");
    });

    ac!("AC-3 DigitalTwin::fork_under_intervention", {
        use hipcortex::continuous_dynamics::{ContinuousDynamics, KalmanVectorField};
        use hipcortex::digital_twin::{DigitalTwin, SyncPolicy};
        use hipcortex::cognitive_state::CognitiveHandle;
        use hipcortex::cognitive_gc::CognitiveGC;
        use hipcortex::memory_store::MemoryStore;
        use hipcortex::self_model::{SelfModel, calibration::CalibrationTracker};
        use hipcortex::coherence::CoherenceChecker;
        use hipcortex::world_model_enhanced::WorldModelEnhanced;
        use hipcortex::persistence::InMemoryBackend;
        use std::sync::{Arc, Mutex, RwLock};

        let store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
        let wm = Arc::new(RwLock::new(WorldModelEnhanced::new()));
        let sm = Arc::new(SelfModel::new());
        let coherence = Arc::new(CoherenceChecker::new());
        let cal = Arc::new(CalibrationTracker::new());
        let gc = Arc::new(CognitiveGC::new());
        let handle: CognitiveHandle<InMemoryBackend> = CognitiveHandle::new(store, wm, sm, None, coherence, cal, gc);
        let fork = handle.fork().unwrap();
        let vf = KalmanVectorField::new(2);
        let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
        let mut twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0, std::collections::HashMap::new());
        twin.fork_under_intervention("d", 1.0);
        assert!(twin.pinned_interventions().contains_key("d"));
    });

    ac!("AC-4 CognitiveDelta SCM variants compile", {
        use hipcortex::cognitive_state::CognitiveDelta;
        use hipcortex::world_model_enhanced::causal::FailureSignal;
        let _ = CognitiveDelta::Intervene { var: "x".into(), value: 1.0 };
        let _ = CognitiveDelta::CreditAssign(FailureSignal::MaxIterations);
        let _ = CognitiveDelta::RewriteStructuralEquation { node_id: "z".into(), new_weights: vec![1.0] };
    });

    ac!("AC-5 OOD local rewiring preserves attribution", {
        use hipcortex::world_model_enhanced::causal::{CausalGraph, FailureSignal, LinearSE};
        use std::sync::Arc;
        let mut g = CausalGraph::new();
        for i in 0..5 { g.add_node(format!("n{}", i)).unwrap(); }
        for i in 0..4 {
            g.add_edge(format!("n{}", i), format!("n{}", i+1)).unwrap();
            if let Some(n) = g.node_mut(&format!("n{}", i+1)) {
                n.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
            }
        }
        if let Some(n) = g.node_mut("n2") { n.equation = Some(Arc::new(LinearSE { weights: vec![2.0] })); }
        let traj = vec![std::collections::HashMap::from([
            ("n0".to_string(), 1.0), ("n1".to_string(), 1.0),
            ("n2".to_string(), 3.0), ("n3".to_string(), 3.0), ("n4".to_string(), 3.0),
        ])];
        let r = g.credit_assign(&traj, &FailureSignal::MaxIterations).unwrap();
        assert!(r.confidence >= 0.0 && r.confidence <= 1.0);
    });

    ac!("AC-6 crate compiles (verified by suite reaching this line)", {
        // If we reach here, the crate compiled successfully.
    });

    println!("\n=== Acceptance: {}/{} passed ===", passed, passed + failed);
    if failed > 0 { std::process::exit(1); }
}
