use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use hipcortex::task_graph::{TaskGraph, TaskNode, TaskState};
use hipcortex::executive_scheduler::{ExecutiveScheduler, StackFrame};
use hipcortex::world_model_enhanced::{MctsSimulator, TransitionModel};
use hipcortex::self_model::SelfModel;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::session_context::SessionContext;
use hipcortex::topological_memory::CausalTopoGraph;

#[cfg(feature = "tokio")]
use hipcortex::coherence::{CoherenceWriteActor, CoherenceWriteMutation};
#[cfg(feature = "tokio")]
use std::sync::Arc;

#[test]
fn test_cognitive_os_end_to_end() {
    // 1. TaskGraph schedules an intervention step
    let task_id = Uuid::new_v4();
    let mut preconditions = HashMap::new();
    preconditions.insert("status".to_string(), "problem_detected".to_string());
    let mut effects = HashMap::new();
    effects.insert("status".to_string(), "problem_resolved".to_string());

    let task = TaskNode {
        id: task_id,
        summary: "Intervention step to resolve problem".to_string(),
        preconditions,
        effects,
        state: TaskState::Pending,
        cost: 1.0,
    };

    let mut task_graph = TaskGraph::new();
    let node_idx = task_graph.graph.add_node(task.clone());
    task_graph.node_index_map.insert(task_id, node_idx);

    let mut scheduler = ExecutiveScheduler::new(10);
    scheduler.active_graph = Some(task_graph);
    scheduler.goal_stack.push(StackFrame {
        task_id: Uuid::new_v4(),
        local_beliefs: {
            let mut b = HashMap::new();
            b.insert("status".to_string(), "problem_detected".to_string());
            b
        },
        current_step: Some(task_id),
    });

    // 2. ExecutiveScheduler executes the step using MctsSimulator
    let transition_model = TransitionModel::new();
    let mut simulator = MctsSimulator::new("problem_detected".to_string(), 1.414);
    let search_res = simulator.search(
        &transition_model,
        &["resolve_problem".to_string()],
        5,
        2,
        |_| 1.0,
    );
    assert!(search_res.is_ok(), "MctsSimulator should run planning search without error");

    let self_model = SelfModel::new();
    let coherence = CoherenceChecker::new();

    let tick_res = scheduler.tick(&self_model, &coherence);
    assert!(tick_res.is_ok(), "Scheduler tick should complete successfully");
    assert_eq!(
        scheduler.active_graph.as_ref().unwrap().graph[node_idx].state,
        TaskState::Completed,
        "Task step state should transition to Completed after tick"
    );

    // 3. SessionContext evicts obsolete trace nodes using CausalTopoGraph topological recency
    let mut session_context = SessionContext::new(Uuid::new_v4(), 200);
    let mut topo = CausalTopoGraph::new();
    let node1_id = Uuid::new_v4();
    let node2_id = Uuid::new_v4();
    let _ = topo.add_node(node1_id.to_string(), [0.1; 128], HashMap::new());
    let _ = topo.add_node(node2_id.to_string(), [0.1; 128], HashMap::new());

    session_context.page_in(node1_id, 0.5, 60, Some("hash1".into()));
    session_context.page_in(node2_id, 0.8, 60, Some("hash2".into()));
    session_context.paged_items[0].last_accessed = SystemTime::now() - Duration::from_secs(3600);

    let evicted = session_context.evict_with_topological_decay(&topo, 60, 60);
    assert!(evicted >= 1, "Should evict obsolete trace node using CausalTopoGraph topological recency");

    // 4. CoherenceWriteActor checks invariant consistency across temporal and symbolic structures without blocking concurrent readers
    #[cfg(feature = "tokio")]
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let coherence_arc = Arc::new(CoherenceChecker::new());
            let temp_dir = tempfile::tempdir().unwrap();
            let wal_path = temp_dir.path().join("test_wal.log");
            let snapshot_path = temp_dir.path().join("snapshot.db");

            let actor = CoherenceWriteActor::new(
                coherence_arc.clone(),
                wal_path.clone(),
                snapshot_path.clone(),
                100,
            );

            let mutation = CoherenceWriteMutation {
                mutation_id: "mut_1".to_string(),
                payload: b"test_payload".to_vec(),
                target_store: "symbolic_store".to_string(),
            };
            let res = actor.submit_mutation(mutation).await;
            assert!(res.is_ok(), "Mutation should be processed asynchronously without blocking");
            assert!(wal_path.exists(), "WAL file should be written by CoherenceWriteActor");
        });
    }
}
