#[cfg(test)]
mod tests {
    use hipcortex::loop_engine::ReactEngine;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    #[test]
    fn test_react_engine_runs_one_iteration_and_writes_observation() {
        let mut store = MemoryStore::new_in_memory();

        let goal_payload = GoalPayload {
            target_state: "x done".to_string(),
            acceptance_criteria: vec!["x done".to_string()],
            success_factors: vec![SuccessFactor {
                name: "x".to_string(),
                weight: 1.0,
                satisfied: false,
            }],
            max_react_iterations: 1,
            status: GoalStatus::Pending,
            current_iteration: 0,
            ..Default::default()
        };
        let goal = MemoryRecord::new(
            MemoryType::Goal,
            "test".into(),
            "achieve".into(),
            "x done".into(),
            serde_json::to_value(&goal_payload).unwrap(),
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        engine.run(&mut store, goal_id, 1).unwrap();

        let obs: Vec<_> = store
            .all()
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Temporal
                    && r.derived_from == Some(goal_id)
                    && r.react_iteration == Some(0)
            })
            .collect();
        assert!(
            !obs.is_empty(),
            "ReactEngine must write at least one Temporal observation per iteration"
        );
    }
}
