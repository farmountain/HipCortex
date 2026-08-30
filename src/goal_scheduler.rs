//! GoalScheduler — picks the highest-priority active Goal from the store.
//!
//! Score = urgency / estimated_cost. Stateless: re-scores on every call.

use crate::memory_record::MemoryType;
use crate::payloads::{GoalPayload, GoalStatus};
use crate::persistence::MemoryBackend;
use uuid::Uuid;

pub struct GoalScheduler;

impl GoalScheduler {
    /// Return the Uuid of the highest-priority Pending or InProgress Goal for `actor`.
    pub fn next<B: MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        actor: &str,
    ) -> Option<Uuid> {
        store
            .all_by_type(MemoryType::Goal)
            .into_iter()
            .filter(|r| r.actor == actor)
            .filter_map(|r| {
                let p: GoalPayload = serde_json::from_value(r.metadata.clone()).ok()?;
                if p.status != GoalStatus::Pending && p.status != GoalStatus::InProgress {
                    return None;
                }
                let cost = if p.estimated_cost > 0.0 { p.estimated_cost } else { 1.0 };
                let score = p.urgency / cost;
                Some((r.id, score))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id)
    }

    /// Return all active Goals sorted by priority (highest first).
    pub fn ranked<B: MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        actor: &str,
    ) -> Vec<(Uuid, f64)> {
        let mut scored: Vec<(Uuid, f64)> = store
            .all_by_type(MemoryType::Goal)
            .into_iter()
            .filter(|r| r.actor == actor)
            .filter_map(|r| {
                let p: GoalPayload = serde_json::from_value(r.metadata.clone()).ok()?;
                if p.status != GoalStatus::Pending && p.status != GoalStatus::InProgress {
                    return None;
                }
                let cost = if p.estimated_cost > 0.0 { p.estimated_cost } else { 1.0 };
                Some((r.id, p.urgency / cost))
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }
}
