#[cfg(test)]
mod tests {
    use hipcortex::loop_engine::ReactEngine;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    fn make_goal(criteria: Vec<String>, factors: Vec<SuccessFactor>, max_iter: u32) -> MemoryRecord {
        let payload = GoalPayload {
            target_state: "test_target".to_string(),
            acceptance_criteria: criteria,
            success_factors: factors,
            max_react_iterations: max_iter,
            status: GoalStatus::Pending,
            current_iteration: 0,
        };
        MemoryRecord::new(
            MemoryType::Goal,
            "e2e_test".into(),
            "achieve".into(),
            "test_target".into(),
            serde_json::to_value(&payload).unwrap(),
        )
    }

    /// E2E-1: All success_factors pre-marked satisfied → loop exits Succeeded on first iteration.
    #[test]
    fn test_react_loop_succeeds_when_all_factors_satisfied() {
        let mut store = MemoryStore::new_in_memory();
        let goal = make_goal(
            vec!["done".to_string()],
            vec![SuccessFactor { name: "done".into(), weight: 1.0, satisfied: true }],
            5,
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        let status = engine.run(&mut store, goal_id, 1).unwrap();

        assert!(matches!(status, GoalStatus::Succeeded), "Expected Succeeded, got {:?}", status);

        let updated = store.find_by_id(goal_id).unwrap();
        let payload: GoalPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
        assert!(matches!(payload.status, GoalStatus::Succeeded));
    }

    /// E2E-2: No factors satisfied → loop exhausts max_iterations → returns Failed.
    #[test]
    fn test_react_loop_fails_after_max_iterations() {
        let mut store = MemoryStore::new_in_memory();
        let goal = make_goal(
            vec!["impossible".to_string()],
            vec![SuccessFactor { name: "impossible".into(), weight: 1.0, satisfied: false }],
            3,
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        let status = engine.run(&mut store, goal_id, 1).unwrap();

        assert!(matches!(status, GoalStatus::Failed), "Expected Failed, got {:?}", status);

        let updated = store.find_by_id(goal_id).unwrap();
        let payload: GoalPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
        assert!(matches!(payload.status, GoalStatus::Failed));
    }

    /// E2E-3: Provenance chain — each iteration writes exactly 1 Temporal + 1 Reflexion record.
    /// With max_iterations=2 and no satisfaction: expect 2 Temporal + 2 Reflexion records.
    #[test]
    fn test_react_loop_provenance_chain_correct() {
        let mut store = MemoryStore::new_in_memory();
        let goal = make_goal(
            vec![],
            vec![SuccessFactor { name: "x".into(), weight: 1.0, satisfied: false }],
            2,
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        engine.run(&mut store, goal_id, 1).unwrap();

        let all = store.all();

        let temporal_obs: Vec<_> = all.iter().filter(|r| {
            r.record_type == MemoryType::Temporal && r.derived_from == Some(goal_id)
        }).collect();
        let reflexion_obs: Vec<_> = all.iter().filter(|r| {
            r.record_type == MemoryType::Reflexion && r.derived_from == Some(goal_id)
        }).collect();

        assert_eq!(temporal_obs.len(), 2,
            "Expected 2 Temporal observations (one per iteration), got {}", temporal_obs.len());
        assert_eq!(reflexion_obs.len(), 2,
            "Expected 2 Reflexion critiques (one per failed iteration), got {}", reflexion_obs.len());

        let mut iter_vals: Vec<u32> = temporal_obs.iter()
            .filter_map(|r| r.react_iteration)
            .collect();
        iter_vals.sort();
        assert_eq!(iter_vals, vec![0, 1], "react_iteration must be 0 and 1");

        let search_results = store.search_semantic(None, "test_target", 100, false);
        assert!(!search_results.iter().any(|(r, _)| r.status == "archived"),
            "Archived records must not appear in default search");
    }

    /// E2E-4: GoalStatus progression — only one goal record exists after ReactEngine completes.
    #[test]
    fn test_react_loop_goal_record_not_duplicated() {
        let mut store = MemoryStore::new_in_memory();
        let goal = make_goal(
            vec!["unreachable".to_string()],
            vec![SuccessFactor { name: "u".into(), weight: 1.0, satisfied: false }],
            1,
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        engine.run(&mut store, goal_id, 1).unwrap();

        let updated = store.find_by_id(goal_id).unwrap();
        let payload: GoalPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
        assert!(matches!(payload.status, GoalStatus::Failed),
            "Expected Failed after exhausted iterations");

        let all = store.all();
        let goal_records: Vec<_> = all.iter()
            .filter(|r| r.id == goal_id && r.record_type == MemoryType::Goal)
            .collect();
        assert_eq!(goal_records.len(), 1, "Goal must not be duplicated by ReactEngine");
    }

    /// E2E-5: CognitiveGC — referenced obs gets Archive; unreferenced gets Delete.
    #[test]
    fn test_cognitive_gc_with_react_provenance() {
        use hipcortex::cognitive_gc::{CognitiveGC, GcAction};
        use uuid::Uuid;

        let mut gc = CognitiveGC::new();
        let obs_id = Uuid::new_v4();
        let goal_id = Uuid::new_v4();
        let orphan_id = Uuid::new_v4();

        gc.register_reference(obs_id, goal_id);

        assert_eq!(gc.gc_action(obs_id), GcAction::Archive,
            "Referenced observation must be archived, not deleted");
        assert_eq!(gc.gc_action(orphan_id), GcAction::Delete,
            "Unreferenced observation must be deleted");

        gc.deregister_referencing(goal_id);
        assert_eq!(gc.gc_action(obs_id), GcAction::Delete,
            "After goal removed, observation must be deletable");
    }
}
