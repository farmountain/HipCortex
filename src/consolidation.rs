use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    archive_store::ArchiveStore,
    memory_record::{MemoryRecord, MemoryType},
    memory_store::MemoryStore,
    symbolic_store::{GraphDatabase, SymbolicStore},
    persistence::MemoryBackend,
    tx_log::{TxKind, TxLog},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Groups with fewer records than this are not consolidated.
    pub min_group_size: usize,
    /// Trigger auto-consolidation when hot_count / capacity >= this.
    pub pressure_threshold: f32,
    /// Approximate maximum Hot store capacity (for pressure computation).
    pub capacity_limit: usize,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            min_group_size: 3,
            pressure_threshold: 0.80,
            capacity_limit: 10_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub groups_consolidated: usize,
    pub records_archived: usize,
    pub summary_records_created: usize,
    /// IDs of all archived original records.
    pub archived_ids: Vec<Uuid>,
}

/// P_tx = hot_count / capacity_limit. Returns [0.0, ∞).
pub fn compute_pressure<B: MemoryBackend>(store: &MemoryStore<B>, config: &ConsolidationConfig) -> f32 {
    store.all().len() as f32 / config.capacity_limit as f32
}

/// Greedy tag+actor consolidation.
///
/// Groups Temporal records by `actor:sorted_tags`, collapses groups ≥ min_group_size
/// into a SummaryRecord, archives originals via ArchiveStore, and reanchors SymbolicStore
/// edges from original nodes to the summary node.
pub fn consolidate<B: MemoryBackend, G: GraphDatabase>(
    store: &mut MemoryStore<B>,
    archive: &mut ArchiveStore,
    graph: &mut SymbolicStore<G>,
    tx_log: &TxLog,
    config: &ConsolidationConfig,
) -> Result<ConsolidationReport, String> {
    // 1. Collect Temporal records.
    let temporals: Vec<MemoryRecord> = store
        .all()
        .iter()
        .filter(|r| r.record_type == MemoryType::Temporal)
        .cloned()
        .collect();

    // 2. Group by actor:sorted_tags.
    let mut groups: HashMap<String, Vec<MemoryRecord>> = HashMap::new();
    for rec in temporals {
        let mut sorted_tags = rec.tags.clone();
        sorted_tags.sort();
        let key = format!("{}:{}", rec.actor, sorted_tags.join(","));
        groups.entry(key).or_default().push(rec);
    }

    let mut archived_ids: Vec<Uuid> = Vec::new();
    let mut groups_consolidated = 0usize;
    let mut summary_records_created = 0usize;

    // 3. Consolidate eligible groups.
    for (_key, members) in groups.into_iter().filter(|(_, v)| v.len() >= config.min_group_size) {
        // 4a. Build SummaryRecord.
        let representative = &members[0];
        let summary_meta = serde_json::json!({
            "consolidated_from": members.iter().map(|r| r.id.to_string()).collect::<Vec<_>>(),
            "count": members.len(),
        });
        let summary = MemoryRecord::new(
            MemoryType::Temporal,
            representative.actor.clone(),
            "consolidate".to_string(),
            format!("summary:{}", representative.actor),
            summary_meta,
        );
        let summary_id = summary.id;

        // 4b. Add summary node to graph.
        let mut sym_props = HashMap::new();
        sym_props.insert("type".to_string(), "summary".to_string());
        sym_props.insert("actor".to_string(), representative.actor.clone());
        let sym_summary_id = graph.add_node(&summary_id.to_string(), sym_props);

        // 4c. Add summary to hot store.
        store.add(summary).map_err(|e| e.to_string())?;

        // 4d. Archive originals, reanchor edges.
        let member_ids: Vec<Uuid> = members.iter().map(|r| r.id).collect();
        for original in members {
            let orig_id = original.id;

            // Find symbolic nodes with label matching original record id.
            let orig_sym_nodes = graph.find_by_label(&orig_id.to_string());
            for sym_node in orig_sym_nodes {
                // Reanchor outgoing edges to summary node.
                let edges = graph.edges_from(sym_node.id, None);
                for edge in edges {
                    graph.add_edge(sym_summary_id, edge.to, &edge.relation);
                }
                graph.remove_node(sym_node.id);
            }

            archive.append(original).map_err(|e| e.to_string())?;
            store.delete_by_id(orig_id);
            archived_ids.push(orig_id);
        }

        // tx_log entry per group.
        tx_log.append(TxKind::Consolidate, member_ids, "system");

        groups_consolidated += 1;
        summary_records_created += 1;
    }

    Ok(ConsolidationReport {
        groups_consolidated,
        records_archived: archived_ids.len(),
        summary_records_created,
        archived_ids,
    })
}
