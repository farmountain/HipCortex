use uuid::Uuid;
use std::time::{SystemTime, Duration};

use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::procedural_cache::{ProceduralCache, ProceduralTrace, FSMState, FSMTransition};

#[test]
fn test_temporal_indexer_insert_and_retrieve() {
    let mut indexer = TemporalIndexer::new(2, 3600);
    let trace1 = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "trace1",
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now(),
    };
    let trace2 = TemporalTrace { data: "trace2", ..trace1.clone() };
    indexer.insert(trace1.clone());
    indexer.insert(trace2.clone());
    let recents = indexer.get_recent(2);
    assert_eq!(recents.len(), 2);
    assert!(recents.iter().any(|t| t.data == "trace1"));
    assert!(recents.iter().any(|t| t.data == "trace2"));
}

#[test]
fn test_temporal_indexer_buffer_overflow() {
    let mut indexer = TemporalIndexer::new(1, 3600);
    let trace1 = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "trace1",
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now(),
    };
    let trace2 = TemporalTrace { data: "trace2", ..trace1.clone() };
    indexer.insert(trace1);
    indexer.insert(trace2.clone());
    let recents = indexer.get_recent(2);
    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0].data, "trace2");
}

#[test]
fn test_temporal_indexer_decay_and_prune() {
    let mut indexer = TemporalIndexer::new(2, 1); // decay half-life 1s
    let mut trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "decay",
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now() - Duration::from_secs(60),
    };
    indexer.insert(trace.clone());
    indexer.decay_and_prune();
    let recents = indexer.get_recent(2);
    assert!(recents.is_empty(), "Trace should be pruned after decay");
}

#[test]
fn test_temporal_indexer_remove_and_get() {
    let mut indexer = TemporalIndexer::new(2, 3600);
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "remove",
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
    };
    indexer.insert(trace.clone());
    assert!(indexer.get_trace(trace.id).is_some());
    assert!(indexer.remove(trace.id));
    assert!(indexer.get_trace(trace.id).is_none());
}

#[test]
fn test_temporal_indexer_decay_factor() {
    let mut indexer = TemporalIndexer::new(2, 1);
    let fast = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "fast",
        relevance: 1.0,
        decay_factor: 2.0, // decays twice as fast
        last_access: SystemTime::now() - Duration::from_secs(60),
    };
    let slow = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "slow",
        relevance: 1.0,
        decay_factor: 0.1, // very slow decay
        last_access: SystemTime::now() - Duration::from_secs(60),
    };
    indexer.insert(fast.clone());
    indexer.insert(slow.clone());
    indexer.decay_and_prune();
    let traces: Vec<&str> = indexer.get_recent(2).iter().map(|t| t.data).collect();
    assert!(traces.contains(&"slow"));
    assert!(!traces.contains(&"fast"));
}

#[test]
fn test_procedural_cache_fsm_transitions() {
    let mut proc_cache = ProceduralCache::new();
    let trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: std::collections::HashMap::new(),
    };
    proc_cache.add_trace(trace.clone());
    proc_cache.add_transition(FSMTransition {
        from: FSMState::Start,
        to: FSMState::Observe,
        condition: None,
    });
    proc_cache.add_transition(FSMTransition {
        from: FSMState::Observe,
        to: FSMState::Reason,
        condition: Some("trigger".to_string()),
    });
    assert_eq!(
        proc_cache.advance(trace.id, None),
        Some(FSMState::Observe)
    );
    assert_eq!(
        proc_cache.advance(trace.id, Some("trigger")),
        Some(FSMState::Reason)
    );
}

#[test]
fn test_procedural_cache_fsm_invalid_transition() {
    let mut proc_cache = ProceduralCache::new();
    let trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: std::collections::HashMap::new(),
    };
    proc_cache.add_trace(trace.clone());
    proc_cache.add_transition(FSMTransition {
        from: FSMState::Start,
        to: FSMState::Observe,
        condition: None,
    });
    assert_eq!(proc_cache.advance(trace.id, Some("wrong")), None);
}

#[test]
fn test_procedural_cache_remove_and_reset() {
    let mut proc_cache = ProceduralCache::new();
    let mut trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Reason,
        memory: std::collections::HashMap::new(),
    };
    trace.memory.insert("k".into(), "v".into());
    proc_cache.add_trace(trace.clone());
    assert!(proc_cache.reset_trace(trace.id).is_some());
    let t = proc_cache.get_trace(trace.id).unwrap();
    assert_eq!(t.current_state, FSMState::Start);
    assert!(t.memory.is_empty());
    assert!(proc_cache.remove_trace(trace.id));
    assert!(proc_cache.get_trace(trace.id).is_none());
}
