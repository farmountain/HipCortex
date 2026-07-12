use hipcortex::world_model_enhanced::transition::{StateTransition, TransitionModel};

#[test]
fn test_surprise_booster_dirichlet_system_id_update() {
    let mut transitions = TransitionModel::new();
    transitions.record_transition(StateTransition {
        from_state: "state_A".to_string(),
        action: "act_build".to_string(),
        to_state: "state_OOM".to_string(),
    }).unwrap();

    // Normal transition increment adds +1.0
    let normal_prob = transitions.predict("state_A", "act_build");
    assert!(normal_prob.is_ok());
    let prob_map = normal_prob.unwrap().probabilities;
    assert_eq!(prob_map.len(), 2);

    // Apply surprise-driven System-ID booster update (gamma = 5.0) after unexpected failure
    transitions.update_with_system_id("state_A", "act_build", "state_CRASH", 5.0);

    let updated = transitions.predict("state_A", "act_build").unwrap();
    let crash_prob = updated.probabilities.get("state_CRASH").cloned().unwrap_or(0.0);
    assert!(crash_prob > 0.6, "Surprise booster failed to rapidly re-learn transition distribution! crash_prob: {}", crash_prob);
}

#[test]
fn test_loop_engine_surprise_reflexion() {
    use hipcortex::topological_memory::CausalTopoGraph;
    use hipcortex::loop_engine::LoopEngine;

    let mut engine = LoopEngine::new(CausalTopoGraph::new());

    // 1. Test low surprise (< 0.12): standard count update
    let result_low = engine.process_surprise_reflexion("state_init", "act_step", "state_next", 0.05);
    assert_eq!(result_low, Ok(true));
    assert_eq!(engine.metrics.mutations, 0); // No mutation count check triggered for standard update

    // 2. Test high surprise (>= 0.12): accelerated System-ID booster update + Coherence check
    let result_high = engine.process_surprise_reflexion("state_init", "act_step", "state_anomaly", 0.45);
    assert_eq!(result_high, Ok(true));
    assert_eq!(engine.metrics.mutations, 1); // High surprise booster verified by coherence gate & mutation counted
}

