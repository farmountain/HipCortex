use hipcortex::consolidation::{
    contract_community, detect_communities, induce_belief_record, induce_skill_record,
    mine_and_consolidate, mine_causal_motifs,
};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use std::collections::HashMap;
use uuid::Uuid;

// ── Phase-2 causal motif mining ───────────────────────────────────────────────

fn mem_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

/// Add a Temporal record with optional parent.
fn add_temporal(store: &mut MemoryStore<InMemoryBackend>, action: &str, parent: Option<Uuid>) -> Uuid {
    let mut rec = MemoryRecord::new(
        MemoryType::Temporal,
        "agent".into(),
        action.into(),
        action.into(),
        serde_json::json!({}),
    );
    rec.derived_from = parent;
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

#[test]
fn mine_motifs_finds_repeated_chain() {
    let mut store = mem_store();

    // Create 3 identical 3-step chains: observe → reason → act
    for _ in 0..3 {
        let a = add_temporal(&mut store, "observe", None);
        let b = add_temporal(&mut store, "reason", Some(a));
        let _c = add_temporal(&mut store, "act", Some(b));
    }

    let motifs = mine_causal_motifs(&store, 3, 2, 5);
    assert!(!motifs.is_empty(), "should find at least one motif");
    let top = &motifs[0];
    assert_eq!(top.frequency, 3);
    // Action sequence should contain "observe", "reason", "act"
    assert!(top.action_sequence.contains(&"observe".to_string()) || top.action_sequence.len() >= 2);
}

#[test]
fn mine_motifs_min_frequency_filters() {
    let mut store = mem_store();

    // 2 chains — below min_frequency of 3
    for _ in 0..2 {
        let a = add_temporal(&mut store, "plan", None);
        let _b = add_temporal(&mut store, "exec", Some(a));
    }

    let motifs = mine_causal_motifs(&store, 3, 2, 5);
    assert!(motifs.is_empty(), "should not find motifs below min_frequency");
}

#[test]
fn mine_motifs_min_len_filters_single_step() {
    let mut store = mem_store();

    // 4 orphan Temporal records (no derived_from) → no chains of len >= 2
    for _ in 0..4 {
        add_temporal(&mut store, "solo", None);
    }

    let motifs = mine_causal_motifs(&store, 2, 2, 5);
    assert!(motifs.is_empty(), "single-step records have no derived_from, no chain");
}

#[test]
fn induce_records_have_correct_types() {
    let mut store = mem_store();
    for _ in 0..3 {
        let a = add_temporal(&mut store, "sense", None);
        let _b = add_temporal(&mut store, "respond", Some(a));
    }
    let motifs = mine_causal_motifs(&store, 3, 2, 5);
    assert!(!motifs.is_empty());
    let skill = induce_skill_record(&motifs[0], "test-agent");
    assert_eq!(skill.record_type, MemoryType::Skill);
    let belief = induce_belief_record(&motifs[0], skill.id, "test-agent");
    assert_eq!(belief.record_type, MemoryType::Belief);
    assert_eq!(belief.derived_from, Some(skill.id));
}

#[test]
fn mine_and_consolidate_induces_and_archives() {
    let mut store = mem_store();
    let mut source_ids = Vec::new();
    for _ in 0..3 {
        let a = add_temporal(&mut store, "fetch", None);
        let b = add_temporal(&mut store, "process", Some(a));
        source_ids.push(a);
        source_ids.push(b);
    }
    let initial_count = store.record_count();

    let report = mine_and_consolidate(&mut store, None, None, None, 3, "agent").unwrap();

    assert!(report.motifs_found > 0, "should find motifs");
    assert!(report.skills_induced > 0, "should induce skills");
    assert!(report.beliefs_induced > 0, "should induce beliefs");
    // Source records removed; Skill+Belief added → net change = (inducted) - (archived)
    let final_count = store.record_count();
    assert!(
        final_count < initial_count,
        "hot store should shrink: {} → {}",
        initial_count,
        final_count
    );
    // Verify source records are gone
    for id in &report.source_ids_archived {
        assert!(store.find_by_id(*id).is_none(), "source {id} must be removed from hot store");
    }
}

fn make_store() -> SymbolicStore<InMemoryGraph> {
    SymbolicStore::new()
}

fn add_node(store: &mut SymbolicStore<InMemoryGraph>, label: &str) -> Uuid {
    store.add_node(label, HashMap::new())
}

// ── detect_communities ────────────────────────────────────────────────────────

#[test]
fn detect_communities_empty_returns_empty() {
    let store = make_store();
    let result = detect_communities(&store, &[]);
    assert!(result.is_empty());
}

#[test]
fn detect_communities_isolated_nodes_each_own_community() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let c = add_node(&mut store, "c");
    let communities = detect_communities(&store, &[a, b, c]);
    // 3 isolated nodes → 3 singleton communities
    assert_eq!(communities.len(), 3);
    for comm in &communities {
        assert_eq!(comm.len(), 1);
    }
}

#[test]
fn detect_communities_connected_nodes_same_community() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let c = add_node(&mut store, "c");
    store.add_edge(a, b, "related");
    store.add_edge(b, c, "related");
    let communities = detect_communities(&store, &[a, b, c]);
    // All connected → should form at most 2 communities (greedy single-pass)
    // At minimum: a+b together (or b+c), never all isolated
    let total_nodes: usize = communities.iter().map(|c| c.len()).sum();
    assert_eq!(total_nodes, 3, "all nodes must be assigned");
    // At least one community has > 1 node
    assert!(communities.iter().any(|c| c.len() > 1));
}

#[test]
fn detect_communities_ignores_external_edges() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let external = add_node(&mut store, "external");
    store.add_edge(a, external, "cross");
    // Only pass [a, b] — external node not in node_ids
    let communities = detect_communities(&store, &[a, b]);
    let total: usize = communities.iter().map(|c| c.len()).sum();
    assert_eq!(total, 2);
    // a and b are isolated w.r.t. each other → 2 singletons
    assert_eq!(communities.len(), 2);
}

// ── contract_community ────────────────────────────────────────────────────────

#[test]
fn contract_community_empty_is_noop() {
    let mut store = make_store();
    let summary = add_node(&mut store, "summary");
    let result = contract_community(&mut store, &[], summary);
    assert!(result.is_ok());
}

#[test]
fn contract_community_removes_community_nodes() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let summary = add_node(&mut store, "summary");
    store.add_edge(a, b, "related");
    contract_community(&mut store, &[a, b], summary).unwrap();
    // a and b should be gone
    assert!(store.get_node(a).is_none(), "a must be removed");
    assert!(store.get_node(b).is_none(), "b must be removed");
    // summary survives
    assert!(store.get_node(summary).is_some(), "summary must survive");
}

#[test]
fn contract_community_redirects_cross_edges_to_summary() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let external = add_node(&mut store, "external");
    let summary = add_node(&mut store, "summary");
    // a → external (cross-community), a → b (intra)
    store.add_edge(a, external, "depends");
    store.add_edge(a, b, "intra");
    contract_community(&mut store, &[a, b], summary).unwrap();
    // summary → external must exist
    let out = store.edges_from(summary, None);
    assert!(
        out.iter().any(|e| e.to == external && e.relation == "depends"),
        "cross-community edge must be redirected to summary"
    );
    // intra edge must be gone (b removed)
    assert!(store.get_node(b).is_none());
}
