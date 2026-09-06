//! GoalScheduler — picks the highest-priority active Goal from the store.
//!
//! Score = urgency / estimated_cost. Stateless: re-scores on every call.
//! v2.8.0: WM-coupled planning — plan_action_sequence orders success_factors
//! by MAP probability (most grounded first); wm_ranked breaks goal ties using
//! WM coverage fraction.

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

    /// Order a goal's unsatisfied success_factors by WM MAP probability (highest first).
    /// Factors with MAP > 0 are most grounded — do them first. Satisfied factors excluded.
    pub fn plan_action_sequence(
        payload: &GoalPayload,
        wm: &crate::world_model_enhanced::WorldModelEnhanced,
    ) -> Vec<String> {
        let mut scored: Vec<(String, f64)> = payload
            .success_factors
            .iter()
            .filter(|sf| !sf.satisfied)
            .map(|sf| {
                let prob = wm
                    .predict_next_state(&sf.name, "execute")
                    .map(|pred| pred.probabilities.values().cloned().fold(0.0_f64, f64::max))
                    .unwrap_or(0.0);
                (sf.name.clone(), prob)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(name, _)| name).collect()
    }

    /// Like `ranked`, but breaks urgency/cost ties using WM coverage fraction.
    /// Coverage = fraction of success_factors that have any WM transition data (MAP > 0).
    pub fn wm_ranked<B: MemoryBackend>(
        store: &crate::memory_store::MemoryStore<B>,
        actor: &str,
        wm: &crate::world_model_enhanced::WorldModelEnhanced,
    ) -> Vec<Uuid> {
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
                let base = p.urgency / cost;
                let total = p.success_factors.len().max(1) as f64;
                let grounded = p.success_factors.iter().filter(|sf| {
                    wm.predict_next_state(&sf.name, "execute")
                        .map(|pr| pr.probabilities.values().cloned().fold(0.0_f64, f64::max) > 0.0)
                        .unwrap_or(false)
                }).count() as f64;
                let coverage = grounded / total;
                Some((r.id, base * (1.0 + coverage)))
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(id, _)| id).collect()
    }
}
