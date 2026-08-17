//! Justification-Based Truth Maintenance System (JTMS).
//!
//! Minimal dependency-list propagation: a Belief is `In` if its `in_list` is
//! non-empty and every node in `in_list` is In (and every node in `out_list`
//! is Out). Retraction BFS-cascades to dependents whose entire `in_list`
//! becomes Out, marking them Out as well.
//!
//! All mutations go through `MemoryStore::update_record` — no bypass.

use uuid::Uuid;

use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::payloads::{BeliefPayload, JtmsLabel};
use crate::persistence::MemoryBackend;
use crate::tx_log::{TxKind, TxLog};

/// Load a Belief record's payload from metadata. Returns None if not a Belief.
fn load_belief(record: &MemoryRecord) -> Option<BeliefPayload> {
    if record.record_type != MemoryType::Belief {
        return None;
    }
    serde_json::from_value(record.metadata.clone()).ok()
}

/// Persist updated BeliefPayload back into a record's metadata via update_record.
fn save_belief<B: MemoryBackend>(
    store: &mut MemoryStore<B>,
    id: Uuid,
    payload: &BeliefPayload,
) -> Result<(), String> {
    let metadata = serde_json::to_value(payload).map_err(|e| e.to_string())?;
    store
        .update_record(id, None, None, None, None, Some(metadata))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Assert a justification for a belief: set its `in_list` / `out_list`,
/// recompute its label, and register back-pointers on all in_list members.
///
/// Returns the new `JtmsLabel` for the belief.
pub fn assert_justification<B: MemoryBackend>(
    store: &mut MemoryStore<B>,
    belief_id: Uuid,
    in_nodes: Vec<Uuid>,
    out_nodes: Vec<Uuid>,
) -> Result<JtmsLabel, String> {
    // Compute the new label: In if every in_node is In and every out_node is Out.
    let new_label = {
        let all_in = in_nodes.iter().all(|&dep| {
            store
                .find_by_id(dep)
                .and_then(|r| load_belief(r))
                .map(|p| p.jtms_label == JtmsLabel::In)
                .unwrap_or(false)
        });
        let all_out = out_nodes.iter().all(|&dep| {
            store
                .find_by_id(dep)
                .and_then(|r| load_belief(r))
                .map(|p| p.jtms_label == JtmsLabel::Out || p.jtms_label == JtmsLabel::Unknown)
                .unwrap_or(true)
        });
        if in_nodes.is_empty() || !all_in || !all_out {
            JtmsLabel::Out
        } else {
            JtmsLabel::In
        }
    };

    // Update the belief itself.
    let record = store.find_by_id(belief_id).ok_or("belief not found")?.clone();
    let mut payload = load_belief(&record).ok_or("not a Belief record")?;
    payload.in_list = in_nodes.clone();
    payload.out_list = out_nodes;
    payload.jtms_label = new_label.clone();
    save_belief(store, belief_id, &payload)?;

    // Register this belief as a dependent on each in_list member.
    let deps: Vec<Uuid> = in_nodes.to_vec();
    for dep_id in deps {
        if let Some(dep_rec) = store.find_by_id(dep_id).cloned() {
            if let Some(mut dep_payload) = load_belief(&dep_rec) {
                if !dep_payload.dependents.contains(&belief_id) {
                    dep_payload.dependents.push(belief_id);
                    let _ = save_belief(store, dep_id, &dep_payload);
                }
            }
        }
    }

    Ok(new_label)
}

/// Retract a belief: mark it Out, BFS-cascade to dependents whose entire
/// `in_list` is now Out, mark them Out too.
///
/// Returns IDs of every belief marked Out (including the root).
pub fn propagate_retraction<B: MemoryBackend>(
    store: &mut MemoryStore<B>,
    retracted_id: Uuid,
    tx_log: Option<&TxLog>,
    actor: &str,
) -> Vec<Uuid> {
    let mut cascaded: Vec<Uuid> = Vec::new();
    let mut queue: Vec<Uuid> = vec![retracted_id];

    while let Some(id) = queue.pop() {
        // Load belief — skip if missing or not a Belief.
        let record = match store.find_by_id(id).cloned() {
            Some(r) => r,
            None => continue,
        };
        let mut payload = match load_belief(&record) {
            Some(p) => p,
            None => continue,
        };

        // Already Out — nothing to do.
        if payload.jtms_label == JtmsLabel::Out {
            continue;
        }

        // Mark Out.
        payload.jtms_label = JtmsLabel::Out;
        if save_belief(store, id, &payload).is_ok() {
            if let Some(tx) = tx_log {
                tx.append(TxKind::BeliefRetract, vec![id], actor);
            }
            cascaded.push(id);
        }

        // Enqueue dependents whose entire in_list is now Out.
        let deps = payload.dependents.clone();
        for dep_id in deps {
            if let Some(dep_rec) = store.find_by_id(dep_id).cloned() {
                if let Some(dep_payload) = load_belief(&dep_rec) {
                    if dep_payload.jtms_label != JtmsLabel::Out {
                        // Check if all in_list members are Out.
                        let all_out = dep_payload.in_list.iter().all(|&src| {
                            store
                                .find_by_id(src)
                                .and_then(|r| load_belief(r))
                                .map(|p| p.jtms_label == JtmsLabel::Out)
                                .unwrap_or(true) // missing = treat as Out
                        });
                        if all_out {
                            queue.push(dep_id);
                        }
                    }
                }
            }
        }
    }

    cascaded
}

/// Check JTMS consistency: returns pairs of (belief_id, violation_description)
/// for any In belief with an empty in_list (except explicitly asserted priors).
pub fn check_jtms_consistency<B: MemoryBackend>(
    store: &MemoryStore<B>,
) -> Vec<(Uuid, String)> {
    store
        .all()
        .iter()
        .filter(|r| r.record_type == MemoryType::Belief)
        .filter_map(|r| {
            let payload = load_belief(r)?;
            if payload.jtms_label == JtmsLabel::In && payload.in_list.is_empty() {
                Some((r.id, "In belief has empty in_list (no justification)".to_string()))
            } else {
                None
            }
        })
        .collect()
}
