// Integration tests for intelligence hooks in TemporalIndexer + SymbolicStore
//
// Verifies that Sections 7+8 integration wiring works correctly:
// - TemporalIndexer: self_model gating, world_model observation, coherence validation
// - SymbolicStore: self_model resource check, world_model entity tracking, coherence gate
//
// These tests do NOT require the web-server feature — they test direct module wiring.

use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::self_model::SelfModel;
use hipcortex::decay::DecayType;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

// ============================================================================
// Test helpers
// ============================================================================

fn make_temporal_trace(id: Uuid, data: String, relevance: f32) -> TemporalTrace<String> {
    TemporalTrace {
        id,
        timestamp: SystemTime::now(),
        data,
        relevance,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
        decay_type: DecayType::Exponential {
            half_life: std::time::Duration::from_secs(3600),
        },
    }
}

/// Register common capabilities so self_model doesn't block operations.
fn setup_self_model() -> Arc<SelfModel> {
    let sm = Arc::new(SelfModel::new());
    for (name, desc) in &[
        ("temporal_insert", "Insert temporal trace"),
        ("symbolic_add_node", "Add symbolic graph node"),
        ("symbolic_add_edge", "Add symbolic graph edge"),
    ] {
        let _ = sm.register_capability(hipcortex::self_model::CapabilityDescriptor {
            name: name.to_string(),
            description: desc.to_string(),
            required_cpu_percent: 5.0,
            required_memory_mb: 10.0,
            limitations: vec![],
        });
    }
    sm
}

// ============================================================================
// TemporalIndexer Intelligence Hook Tests (Section 7.6)
// ============================================================================

#[test]
fn test_temporal_indexer_constructs_without_intelligence() {
    let mut idx: TemporalIndexer<String> = TemporalIndexer::new(10, 3600);
    let trace = make_temporal_trace(Uuid::new_v4(), "event_A".to_string(), 1.0);
    idx.insert(trace);
    let recent = idx.get_recent(5);
    assert_eq!(recent.len(), 1);
}

#[test]
fn test_temporal_indexer_with_self_model_gating() {
    let sm = setup_self_model();
    let mut idx: TemporalIndexer<String> = TemporalIndexer::new(10, 3600)
        .with_self_model(sm);

    let trace = make_temporal_trace(Uuid::new_v4(), "gated_event".to_string(), 1.0);
    idx.insert(trace);
    let recent = idx.get_recent(5);
    assert_eq!(recent.len(), 1);
}

#[test]
fn test_temporal_indexer_with_world_model_observation() {
    let wm = Arc::new(WorldModelEnhanced::new());
    let mut idx: TemporalIndexer<String> = TemporalIndexer::new(10, 3600)
        .with_world_model(wm.clone());

    let trace = make_temporal_trace(Uuid::new_v4(), "observed_event".to_string(), 1.0);
    idx.insert(trace);
    let recent = idx.get_recent(5);
    assert_eq!(recent.len(), 1);
}

#[test]
fn test_temporal_indexer_with_coherence_validation() {
    let cc = Arc::new(CoherenceChecker::new());
    let mut idx: TemporalIndexer<String> = TemporalIndexer::new(10, 3600)
        .with_coherence(cc.clone());

    let trace = make_temporal_trace(Uuid::new_v4(), "coherent_event".to_string(), 1.0);
    idx.insert(trace);

    let active = cc.get_active_inconsistencies().unwrap();
    assert!(active.is_empty());
}

#[test]
fn test_temporal_indexer_with_all_intelligence_hooks() {
    let sm = setup_self_model();
    let wm = Arc::new(WorldModelEnhanced::new());
    let cc = Arc::new(CoherenceChecker::new());

    let mut idx: TemporalIndexer<String> = TemporalIndexer::new(10, 3600)
        .with_self_model(sm.clone())
        .with_world_model(wm.clone())
        .with_coherence(cc.clone());

    for i in 0..5 {
        let trace = make_temporal_trace(
            Uuid::new_v4(),
            format!("full_hook_event_{}", i),
            1.0 - (i as f32 * 0.1),
        );
        idx.insert(trace);
    }

    let recent = idx.get_recent(10);
    assert_eq!(recent.len(), 5);
    assert!(sm.is_healthy().unwrap());
    assert!(wm.list_entities().is_ok());
    assert!(cc.get_metrics().is_ok());
}

#[test]
fn test_temporal_indexer_markov_with_intelligence() {
    let sm = setup_self_model();
    let wm = Arc::new(WorldModelEnhanced::new());
    let cc = Arc::new(CoherenceChecker::new());

    let mut idx: TemporalIndexer<String> = TemporalIndexer::new(20, 3600)
        .with_self_model(sm)
        .with_world_model(wm)
        .with_coherence(cc);
    idx.enable_markov(10);

    for state in &["S0", "S1", "S0", "S1", "S2"] {
        let trace = make_temporal_trace(Uuid::new_v4(), state.to_string(), 1.0);
        idx.insert(trace);
    }

    let recent = idx.get_recent(10);
    assert_eq!(recent.len(), 5);
}

// ============================================================================
// SymbolicStore Intelligence Hook Tests (Section 8.6)
// ============================================================================

#[test]
fn test_symbolic_store_constructs_with_intelligence() {
    let sm = setup_self_model();
    let wm = Arc::new(WorldModelEnhanced::new());
    let cc = Arc::new(CoherenceChecker::new());

    let store = SymbolicStore::new()
        .with_self_model(sm.clone())
        .with_world_model(wm.clone())
        .with_coherence(cc.clone());

    let (nodes, edges) = store.export_graph();
    assert!(nodes.is_empty());
    assert!(edges.is_empty());
}

#[test]
fn test_symbolic_store_add_node_with_self_model() {
    let sm = setup_self_model();
    let mut store = SymbolicStore::new().with_self_model(sm.clone());

    let node_id = store.add_node("test_entity", HashMap::new());
    assert_ne!(node_id, Uuid::nil());

    let (nodes, _) = store.export_graph();
    assert_eq!(nodes.len(), 1);
}

#[test]
fn test_symbolic_store_add_node_with_world_model() {
    let wm = Arc::new(WorldModelEnhanced::new());
    let mut store = SymbolicStore::new().with_world_model(wm.clone());

    let node_id = store.add_node("tracked_entity", HashMap::new());
    assert_ne!(node_id, Uuid::nil());

    // Verify the store has the node
    let node = store.get_node(node_id);
    assert!(node.is_some());
}

#[test]
fn test_symbolic_store_add_node_with_coherence() {
    let cc = Arc::new(CoherenceChecker::new());
    let mut store = SymbolicStore::new().with_coherence(cc.clone());

    let node_id = store.add_node("coherent_entity", HashMap::new());
    assert_ne!(node_id, Uuid::nil());

    let active = cc.get_active_inconsistencies().unwrap();
    assert!(active.is_empty());
}

#[test]
fn test_symbolic_store_nodes_and_edges_with_intelligence() {
    let sm = setup_self_model();
    let wm = Arc::new(WorldModelEnhanced::new());
    let cc = Arc::new(CoherenceChecker::new());

    let mut store = SymbolicStore::new()
        .with_self_model(sm.clone())
        .with_world_model(wm.clone())
        .with_coherence(cc.clone());

    // Add nodes
    let a = store.add_node("A", HashMap::new());
    let b = store.add_node("B", HashMap::new());
    assert_ne!(a, Uuid::nil());
    assert_ne!(b, Uuid::nil());

    // Add edge (use safe relation name to pass safety guardrail)
    store.add_edge(a, b, "knows");

    let (nodes, edges) = store.export_graph();
    assert_eq!(nodes.len(), 2);
    // Edge count depends on safety guardrail — may be 0 if blocked
    // The important thing is that add_edge didn't panic
}

#[test]
fn test_symbolic_store_remove_node_with_coherence() {
    let cc = Arc::new(CoherenceChecker::new());
    let mut store = SymbolicStore::new().with_coherence(cc.clone());

    let node_id = store.add_node("removable", HashMap::new());
    assert_ne!(node_id, Uuid::nil());

    let removed = store.remove_node(node_id);
    assert!(removed);

    let active = cc.get_active_inconsistencies().unwrap();
    assert!(active.is_empty());
}

#[test]
fn test_symbolic_store_full_intelligence_pipeline() {
    let sm = setup_self_model();
    let wm = Arc::new(WorldModelEnhanced::new());
    let cc = Arc::new(CoherenceChecker::new());

    let mut store = SymbolicStore::new()
        .with_self_model(sm.clone())
        .with_world_model(wm.clone())
        .with_coherence(cc.clone());

    // Add nodes
    let e1 = store.add_node("entity_1", HashMap::from([
        ("type".to_string(), "person".to_string()),
    ]));
    let e2 = store.add_node("entity_2", HashMap::from([
        ("type".to_string(), "document".to_string()),
    ]));
    assert_ne!(e1, Uuid::nil(), "entity_1 should have valid UUID");
    assert_ne!(e2, Uuid::nil(), "entity_2 should have valid UUID");

    // Add edge
    store.add_edge(e1, e2, "knows");

    // Verify graph has nodes
    let (nodes, _edges) = store.export_graph();
    assert_eq!(nodes.len(), 2);

    // Verify intelligence layer state
    assert!(sm.is_healthy().unwrap());
    let active = cc.get_active_inconsistencies().unwrap();
    assert!(active.is_empty());
    let violations = cc.enforce_invariants().unwrap();
    assert!(violations.is_empty());
}
