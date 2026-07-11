use std::collections::HashMap;
use hipcortex::world_model_enhanced::simulator::{SimulationHarness, SimulationTrajectory};
use hipcortex::world_model_enhanced::policy::Policy;
use hipcortex::world_model_enhanced::constraint::ConstraintEngine;
use hipcortex::world_model_enhanced::metalaw::MetaLawEngine;
use hipcortex::world_model_enhanced::transition::TransitionModel;

#[test]
fn test_simulate_trajectory_with_topological_headroom_pruning() {
    let mut policy = Policy::new("AgentAlpha".to_string(), 0.001);
    policy.action_distribution.insert("execute_step".to_string(), 1.0);

    let mut policies = HashMap::new();
    policies.insert("AgentAlpha".to_string(), policy);

    let constraints = ConstraintEngine::new();
    let meta_laws = MetaLawEngine::new();
    let transitions = TransitionModel::new();

    let mut initial_metrics = HashMap::new();
    initial_metrics.insert("latency_ms".to_string(), 10.0);

    let harness = SimulationHarness::new(policies, constraints, meta_laws, transitions);
    let trajectory = harness.simulate_trajectory("AgentAlpha", initial_metrics, 15, "headroom").expect("Simulation failed");

    assert_eq!(trajectory.steps.len(), 15);
    // Verify headroom mode pruning flag was invoked across multi-step execution
    assert_eq!(trajectory.pruning_mode_applied, "headroom");
}
