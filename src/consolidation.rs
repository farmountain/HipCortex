//! Online hierarchical memory compactor.
//!
//! Chain-of-thought: Provides two APIs:
//! 1. Legacy/bulk: `consolidate()` + `compute_pressure()` + `ConsolidationConfig` —
//!    used by REST endpoints and the web_server background loop.
//! 2. Phase-4 graph layer: `detect_communities()` + `contract_community()` —
//!    used by CognitiveDelta::Consolidate via CognitiveHandle::consolidate_memory().

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::archive_store::ArchiveStore;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::persistence::MemoryBackend;
use crate::symbolic_store::{GraphDatabase, SymbolicStore};
use crate::tx_log::{TxKind, TxLog};

// ── Legacy bulk API ───────────────────────────────────────────────────────────

/// Configuration knobs for the bulk consolidation pass.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsolidationConfig {
    /// Minimum number of records in a group before consolidation fires.
    pub min_group_size: usize,
    /// Record count at which pressure = 1.0.
    pub capacity_limit: usize,
    /// Fraction [0, 1] above which a background consolidation pass is triggered.
    pub pressure_threshold: f32,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            min_group_size: 3,
            capacity_limit: 1000,
            pressure_threshold: 0.7,
        }
    }
}

/// Report returned by `consolidate()`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsolidationReport {
    pub groups_consolidated: usize,
    pub records_archived: usize,
    pub summary_records_created: usize,
    pub archived_ids: Vec<Uuid>,
}

/// Compute memory pressure as `record_count / capacity_limit`, clamped to [0, 1].
pub fn compute_pressure<B: MemoryBackend>(
    store: &MemoryStore<B>,
    config: &ConsolidationConfig,
) -> f32 {
    if config.capacity_limit == 0 {
        return 1.0;
    }
    (store.record_count() as f32 / config.capacity_limit as f32).min(1.0)
}

/// Bulk consolidation pass.
///
/// Groups active Temporal records by `(actor, sorted-tags)`. Groups with
/// `>= min_group_size` records are consolidated: originals are moved to the
/// `ArchiveStore`, one summary record is inserted into the hot store, and a
/// symbolic graph node is added for the summary.
pub fn consolidate<B: MemoryBackend, S: GraphDatabase>(
    store: &mut MemoryStore<B>,
    archive: &mut ArchiveStore,
    graph: &mut SymbolicStore<S>,
    log: &TxLog,
    config: &ConsolidationConfig,
) -> Result<ConsolidationReport, String> {
    // Group active Temporal records by (actor, sorted tags)
    let groups = group_temporal_records(store, config.min_group_size);

    let mut groups_consolidated = 0usize;
    let mut records_archived = 0usize;
    let mut summary_records_created = 0usize;
    let mut archived_ids: Vec<Uuid> = Vec::new();

    for (key, ids) in groups {
        // Archive originals
        for id in &ids {
            if let Some(rec) = store.find_by_id(*id).cloned() {
                archive
                    .append(rec)
                    .map_err(|e| format!("archive error: {e}"))?;
                archived_ids.push(*id);
                records_archived += 1;
            }
            store.delete_by_id(*id);
        }

        // Insert summary record
        let summary = MemoryRecord::new(
            MemoryType::Temporal,
            key.0.clone(),
            "consolidated".into(),
            format!("summary:{}", key.1),
            serde_json::json!({ "source_count": ids.len(), "tags": key.1 }),
        );
        let summary_id = summary.id;
        store.add(summary).map_err(|e| format!("store error: {e}"))?;
        log.append(TxKind::Consolidate, vec![summary_id], &key.0);

        // Add graph node for summary
        let mut props = HashMap::new();
        props.insert("actor".into(), key.0.clone());
        props.insert("type".into(), "consolidated_summary".into());
        graph.add_node(&format!("summary:{}", key.0), props);

        groups_consolidated += 1;
        summary_records_created += 1;
    }

    Ok(ConsolidationReport {
        groups_consolidated,
        records_archived,
        summary_records_created,
        archived_ids,
    })
}

/// Group active Temporal records by `(actor, sorted-tags-joined)`.
/// Returns only groups with >= `min_size` members.
fn group_temporal_records<B: MemoryBackend>(
    store: &MemoryStore<B>,
    min_size: usize,
) -> HashMap<(String, String), Vec<Uuid>> {
    let mut groups: HashMap<(String, String), Vec<Uuid>> = HashMap::new();
    for rec in store.all() {
        if rec.record_type != MemoryType::Temporal || rec.status != "active" {
            continue;
        }
        let mut sorted_tags = rec.tags.clone();
        sorted_tags.sort();
        let tag_key = sorted_tags.join(",");
        groups
            .entry((rec.actor.clone(), tag_key))
            .or_default()
            .push(rec.id);
    }
    groups.retain(|_, ids| ids.len() >= min_size);
    groups
}

// ── Phase-4 graph layer API ───────────────────────────────────────────────────

/// Greedy single-pass Louvain community detection on a SymbolicStore subgraph.
///
/// Only edges whose both endpoints are in `node_ids` are considered. Isolated
/// nodes each form a singleton community. Returns `Vec<Vec<Uuid>>`.
pub fn detect_communities<B: GraphDatabase>(
    store: &SymbolicStore<B>,
    node_ids: &[Uuid],
) -> Vec<Vec<Uuid>> {
    if node_ids.is_empty() {
        return vec![];
    }

    let id_set: HashSet<Uuid> = node_ids.iter().copied().collect();

    // Build undirected adjacency map restricted to the subgraph
    let mut adj: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for &id in node_ids {
        for edge in store.edges_from(id, None) {
            if id_set.contains(&edge.to) {
                adj.entry(id).or_default().push(edge.to);
                adj.entry(edge.to).or_default().push(id);
            }
        }
    }

    // Greedy single-pass: join first already-assigned neighbor's community
    let mut community_of: HashMap<Uuid, usize> = HashMap::new();
    let mut next_id: usize = 0;

    for &node in node_ids {
        if community_of.contains_key(&node) {
            continue;
        }
        let assigned = adj
            .get(&node)
            .and_then(|nb| nb.iter().find_map(|n| community_of.get(n).copied()));
        let cid = assigned.unwrap_or_else(|| {
            let id = next_id;
            next_id += 1;
            id
        });
        community_of.insert(node, cid);
    }

    let mut groups: HashMap<usize, Vec<Uuid>> = HashMap::new();
    for (node, cid) in community_of {
        groups.entry(cid).or_default().push(node);
    }
    groups.into_values().collect()
}

/// Contract one community: redirect outgoing cross-community edges to `summary_id`,
/// then remove each original community node.
///
/// `summary_id` must already exist in `store`. Intra-community edges are dropped.
pub fn contract_community<B: GraphDatabase>(
    store: &mut SymbolicStore<B>,
    community: &[Uuid],
    summary_id: Uuid,
) -> Result<(), String> {
    if community.is_empty() {
        return Ok(());
    }
    let community_set: HashSet<Uuid> = community.iter().copied().collect();

    // Collect cross-community outgoing edges before removing nodes
    let mut edges_to_add: Vec<(Uuid, Uuid, String)> = Vec::new();
    for &node in community {
        for edge in store.edges_from(node, None) {
            if !community_set.contains(&edge.to) {
                edges_to_add.push((summary_id, edge.to, edge.relation.clone()));
            }
        }
    }

    // Remove community nodes (drops all their stored edges)
    for &node in community {
        if node != summary_id {
            store.remove_node(node);
        }
    }

    // Re-add cross-community edges from summary
    for (from, to, rel) in edges_to_add {
        store.add_edge(from, to, &rel);
    }

    Ok(())
}
