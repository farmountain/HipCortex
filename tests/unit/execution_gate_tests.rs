use hipcortex::execution_gate::ExecutionGate;
use hipcortex::self_model::{DecisionContext, DecisionEngine, ResourceUsage};
use std::time::Instant;

#[test]
fn test_decision_engine_satisfies_execution_gate() {
    let mut engine = DecisionEngine::new();
    let ctx = DecisionContext {
        priority: 0.9,
        deadline: None,
        user_facing: true,
        cascading_impact: false,
    };
    let resources = ResourceUsage {
        cpu_percent: 10.0,
        memory_mb: 100.0,
        disk_io_mbps: 1.0,
        network_io_mbps: 0.5,
        timestamp: Instant::now(),
    };
    let gate: &mut dyn ExecutionGate = &mut engine;
    let decision = gate.evaluate("test-op", &ctx, 0.95, &resources, 0.9);
    assert!(
        decision.should_execute,
        "healthy system should approve high-confidence op"
    );
}
