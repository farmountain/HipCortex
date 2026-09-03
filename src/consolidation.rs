//! Online hierarchical memory compactor.
//!
//! Chain-of-thought: Provides three APIs:
//! 1. Legacy/bulk: `consolidate()` + `compute_pressure()` + `ConsolidationConfig` —
//!    used by REST endpoints and the web_server background loop.
//! 2. Phase-4 graph layer: `detect_communities()` + `contract_community()` —
//!    used by CognitiveDelta::Consolidate via CognitiveHandle::consolidate_memory().
//! 3. Phase-2 causal motif mining: `mine_causal_motifs()` + `mine_and_consolidate()` —
//!    automatic Skill/Belief induction from recurring derived_from chains.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::archive_store::ArchiveStore;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::payloads::{BeliefPayload, EpistemicStatus, SkillPayload};
use crate::persistence::MemoryBackend;
use crate::symbolic_store::{GraphDatabase, SymbolicStore};
use crate::tx_log::{TxKind, TxLog};
use crate::world_model_enhanced::WorldModelEnhanced;

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

// ── Phase-2 causal motif mining API ──────────────────────────────────────────

/// A recurring causal pattern: a sequence of `action` strings that appears in
/// `derived_from` chains at least `frequency` times.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CausalMotif {
    /// Ordered sequence of `action` field values (causal order, oldest first).
    pub action_sequence: Vec<String>,
    /// All record IDs that participated in any instance of this motif.
    pub member_ids: Vec<Uuid>,
    /// Number of distinct chain instances with this action sequence.
    pub frequency: usize,
}

/// Report returned by `mine_and_consolidate`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct MiningReport {
    pub motifs_found: usize,
    pub skills_induced: usize,
    pub beliefs_induced: usize,
    pub source_ids_archived: Vec<Uuid>,
}

/// Detect a cycle in the `derived_from` provenance chain starting from `id`.
/// Uses tortoise-and-hare via a visited set with depth cap to avoid unbounded traversal.
pub fn has_derived_from_cycle<B: MemoryBackend>(id: uuid::Uuid, store: &MemoryStore<B>) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut current = id;
    loop {
        if !visited.insert(current) {
            return true; // visited twice → cycle
        }
        match store.find_by_id(current).and_then(|r| r.derived_from) {
            Some(parent) => current = parent,
            None => return false,
        }
        if visited.len() > 1024 {
            return false; // depth cap — treat as acyclic
        }
    }
}

/// Mine recurring causal chains from `derived_from` links in Temporal records.
///
/// Returns motifs with `frequency >= min_frequency` and chain length in `[min_len, max_len]`.
pub fn mine_causal_motifs<B: MemoryBackend>(
    store: &MemoryStore<B>,
    min_frequency: usize,
    min_len: usize,
    max_len: usize,
) -> Vec<CausalMotif> {
    // Build id → record lookup for chain walking
    let all: Vec<_> = store.all().into_iter().cloned().collect();
    let by_id: HashMap<Uuid, &MemoryRecord> = all.iter().map(|r| (r.id, r)).collect();

    // Walk derived_from chain backward for every Temporal record
    // signature → (frequency, member_ids)
    let mut groups: HashMap<String, (usize, Vec<Uuid>)> = HashMap::new();

    for rec in &all {
        if rec.record_type != MemoryType::Temporal || rec.status != "active" {
            continue;
        }
        if rec.derived_from.is_none() {
            continue;
        }

        // Build chain walking backward from current record
        let mut chain: Vec<String> = vec![rec.action.clone()];
        let mut chain_ids: Vec<Uuid> = vec![rec.id];
        let mut cursor = rec.derived_from;

        while let Some(parent_id) = cursor {
            if chain.len() >= max_len {
                break;
            }
            match by_id.get(&parent_id) {
                Some(parent) if parent.record_type == MemoryType::Temporal => {
                    chain.push(parent.action.clone());
                    chain_ids.push(parent.id);
                    cursor = parent.derived_from;
                }
                _ => break,
            }
        }

        // Reverse to get oldest-first causal order
        chain.reverse();
        chain_ids.reverse();

        if chain.len() < min_len {
            continue;
        }

        let sig = chain.join("\x00");
        let entry = groups.entry(sig).or_insert((0, Vec::new()));
        entry.0 += 1;
        // Collect unique member IDs across all instances
        for id in &chain_ids {
            if !entry.1.contains(id) {
                entry.1.push(*id);
            }
        }
    }

    // Keep only groups that meet the frequency threshold
    let mut motifs: Vec<CausalMotif> = groups
        .into_iter()
        .filter(|(_, (freq, _))| *freq >= min_frequency)
        .map(|(sig, (freq, ids))| {
            let action_sequence = sig.split('\x00').map(|s| s.to_string()).collect();
            CausalMotif { action_sequence, member_ids: ids, frequency: freq }
        })
        .collect();

    // Stable order by frequency desc, then by sequence for determinism
    motifs.sort_by(|a, b| b.frequency.cmp(&a.frequency).then(a.action_sequence.cmp(&b.action_sequence)));
    motifs
}

/// Induce a `Skill` record from a causal motif.
pub fn induce_skill_record(motif: &CausalMotif, actor: &str) -> MemoryRecord {
    let procedure = motif.action_sequence.join(" → ");
    let payload = SkillPayload {
        procedure: procedure.clone(),
        preconditions: vec![],
        expected_outcomes: vec![format!("pattern repeats {} times", motif.frequency)],
    };
    let meta = serde_json::to_value(&payload).unwrap_or_default();
    let mut rec = MemoryRecord::new(MemoryType::Skill, actor.into(), "induced".into(), procedure, meta);
    rec.evidence = motif.member_ids.clone();
    rec
}

/// Induce a high-confidence `Belief` record from a causal motif, linked to the given Skill.
pub fn induce_belief_record(motif: &CausalMotif, skill_id: Uuid, actor: &str) -> MemoryRecord {
    let proposition = format!("causal pattern: {}", motif.action_sequence.join(" → "));
    let payload = BeliefPayload {
        proposition: proposition.clone(),
        confidence: 0.85,
        epistemic_status: EpistemicStatus::Deduced,
        causal_source_ids: motif.member_ids.clone(),
        ..Default::default()
    };
    let meta = serde_json::to_value(&payload).unwrap_or_default();
    let mut rec = MemoryRecord::new(MemoryType::Belief, actor.into(), "induced".into(), proposition, meta);
    rec.derived_from = Some(skill_id);
    rec.evidence = motif.member_ids.clone();
    rec
}

/// Mine motifs, induce Skill+Belief records, archive source episodes.
///
/// Returns source IDs that were archived (to be committed to cold store by the caller).
pub fn mine_and_consolidate<B: MemoryBackend>(
    store: &mut MemoryStore<B>,
    mut archive: Option<&mut ArchiveStore>,
    wm: Option<&WorldModelEnhanced>,
    log: Option<&TxLog>,
    min_frequency: usize,
    actor: &str,
) -> Result<MiningReport, String> {
    let motifs = mine_causal_motifs(store, min_frequency, 2, 5);

    if motifs.is_empty() {
        return Ok(MiningReport::default());
    }

    let mut skills_induced = 0usize;
    let mut beliefs_induced = 0usize;
    let mut all_source_ids: HashSet<Uuid> = HashSet::new();

    for motif in &motifs {
        // Cycle guard: skip motifs whose provenance chain contains a cycle.
        let has_cycle = motif.member_ids.iter().any(|&id| has_derived_from_cycle(id, store));
        if has_cycle {
            continue;
        }

        // Causal validity check: if WM is provided and has transition data, skip motifs
        // whose first action has no plausible support in the world model.
        if let (Some(wm_ref), Some(first_action)) = (wm, motif.action_sequence.first()) {
            if let Ok(pred) = wm_ref.predict_next_state(first_action, "motif") {
                if !pred.probabilities.is_empty() {
                    // WM has an opinion — ensure at least one predicted state is non-zero
                    let max_p = pred.probabilities.values().cloned().fold(0.0f64, f64::max);
                    if max_p < 1e-6 {
                        continue; // WM says this action leads nowhere plausible
                    }
                }
            }
        }

        // AbstractionGate: require min evidence + temporal grounding before inducing.
        {
            use std::collections::HashSet;
            let gate = crate::abstraction_gate::AbstractionGate::validate(
                &motif.member_ids,
                "",
                &HashSet::new(),
                store,
            );
            if !gate.valid {
                continue;
            }
        }

        let skill = induce_skill_record(motif, actor);
        let skill_id = skill.id;
        store.add(skill).map_err(|e| format!("skill add: {e}"))?;
        if let Some(tx) = log {
            tx.append(TxKind::Consolidate, vec![skill_id], actor);
        }
        skills_induced += 1;

        let belief = induce_belief_record(motif, skill_id, actor);
        store.add(belief).map_err(|e| format!("belief add: {e}"))?;
        beliefs_induced += 1;

        for &id in &motif.member_ids {
            all_source_ids.insert(id);
        }
    }

    // Archive source records to Cold Store, then remove from Hot Store.
    let source_ids_archived: Vec<Uuid> = all_source_ids.into_iter().collect();
    for &id in &source_ids_archived {
        if let Some(arc) = archive.as_deref_mut() {
            if let Some(rec) = store.find_by_id(id).cloned() {
                let _ = arc.append(rec);
            }
        }
        store.delete_by_id(id);
    }

    Ok(MiningReport {
        motifs_found: motifs.len(),
        skills_induced,
        beliefs_induced,
        source_ids_archived,
    })
}
