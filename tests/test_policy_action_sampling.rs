use hipcortex::world_model_enhanced::entity::EntityState;
use hipcortex::world_model_enhanced::policy::Policy;
use hipcortex::world_model_enhanced::transition::TransitionModel;
use std::collections::HashMap;

#[test]
fn test_policy_action_sampling_and_temperature() {
    let mut policy = Policy::new("AgentAlpha".to_string(), 1.0);
    policy
        .utility_weights
        .insert("cache_hit_rate".to_string(), 2.0);
    policy
        .action_distribution
        .insert("fetch_cache".to_string(), 0.8);
    policy
        .action_distribution
        .insert("slow_query".to_string(), 0.2);

    let state = EntityState {
        properties: vec![1.0, 0.5],
        covariance: vec![vec![0.1, 0.0], vec![0.0, 0.1]],
    };
    let transitions = TransitionModel::new();

    let action = policy.sample_action(&state, &transitions);
    assert!(action == "fetch_cache" || action == "slow_query");

    // Verify temperature scaling changes distribution deterministically at zero/near-zero
    policy.temperature = 0.0001;
    let greedy_action = policy.sample_action(&state, &transitions);
    assert_eq!(greedy_action, "fetch_cache");
}
