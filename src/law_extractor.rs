//! LawExtractor — clusters surprising Intent records into reusable structural Laws.
//! Self-prompts via route_uncertainty before writing when the causal pattern is ambiguous.
//!
//! Chain-of-Thought:
//!   1. Collect Intent records for `actor` flagged `was_surprising == true`.
//!   2. Cluster them by `target_entity`; a cluster with >= MIN_SURPRISING_FOR_LAW members
//!      is a candidate structural regularity worth generalising.
//!   3. Extract candidate causal variables from repeated action tokens; if too few,
//!      self-prompt (T0) by mining `content_excerpt`, then route_uncertainty as the last gate.
//!   4. Build a structural equation `effect ~ action(vars)` and write it as a Law record
//!      (through MemoryStore::add so SafetyGuardrail / audit / integrity stay intact).
//!   5. Idempotency: never write a Law whose equation already exists in the store.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::agent_guidance::ClarifyRoute;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::payloads::LawPayload;
use crate::persistence::MemoryBackend;

/// Minimum number of surprising Intents (per target-entity cluster) before a Law is coined.
pub const MIN_SURPRISING_FOR_LAW: usize = 3;
/// Cap on how many causal variables a single Law equation may name.
const MAX_CAUSAL_VARS: usize = 4;

pub struct LawExtractor;

impl LawExtractor {
    /// Scan surprising Intent records for `actor`; extract Laws for each target-entity
    /// cluster of size >= MIN_SURPRISING_FOR_LAW. Returns the IDs of newly written Law
    /// records (empty if nothing qualified or every candidate was a duplicate).
    pub fn attempt_extract<B: MemoryBackend>(
        store: &mut MemoryStore<B>,
        actor: &str,
    ) -> Vec<Uuid> {
        // Collect owned Intent snapshots up front so no immutable borrow of `store`
        // is alive when we later call `store.add` (a mutable borrow).
        let intents = Self::collect_surprising(store, actor);
        if intents.is_empty() {
            return Vec::new();
        }

        // Cluster by target_entity.
        let mut clusters: HashMap<String, Vec<MemoryRecord>> = HashMap::new();
        for rec in intents {
            let entity = rec
                .metadata
                .get("target_entity")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            clusters.entry(entity).or_default().push(rec);
        }

        // Snapshot existing Law equations for idempotency (owned Strings, borrow released).
        let existing_equations = Self::existing_law_equations(store);
        let mut written = Vec::new();

        // Deterministic cluster order so equal-sized runs behave reproducibly.
        let mut cluster_keys: Vec<String> = clusters.keys().cloned().collect();
        cluster_keys.sort();

        for entity in cluster_keys {
            let cluster = match clusters.get(&entity) {
                Some(c) => c,
                None => continue,
            };
            if cluster.len() < MIN_SURPRISING_FOR_LAW {
                continue;
            }

            // Candidate causal variables := action tokens repeated across the cluster.
            let mut action_freq: HashMap<String, usize> = HashMap::new();
            for rec in cluster {
                for token in rec.action.split_whitespace() {
                    let t = token.to_lowercase();
                    if t.len() > 2 {
                        *action_freq.entry(t).or_default() += 1;
                    }
                }
            }
            let mut causal_vars: Vec<String> = action_freq
                .into_iter()
                .filter(|(_, count)| *count >= 2)
                .map(|(token, _)| token)
                .take(MAX_CAUSAL_VARS)
                .collect();
            causal_vars.sort();

            // Self-prompt (T0): too few variables — mine content_excerpt for more tokens.
            if causal_vars.len() < 2 {
                let from_content: Vec<String> = cluster
                    .iter()
                    .filter_map(|r| {
                        r.metadata
                            .get("content_excerpt")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    })
                    .flat_map(|c| {
                        c.split_whitespace()
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .filter(|t| t.len() > 3)
                    .collect();
                for token in from_content {
                    let t = token.to_lowercase();
                    if !causal_vars.contains(&t) {
                        causal_vars.push(t);
                        if causal_vars.len() >= 2 {
                            break;
                        }
                    }
                }
            }

            // Still ambiguous after the T0 self-prompt — consult route_uncertainty.
            if causal_vars.len() < 2 {
                let route = crate::agent_guidance::route_uncertainty(true, 0, false, 0.8);
                match route {
                    ClarifyRoute::SelfPrompt { .. } => {
                        // T0 fallback: use the entity name itself as a causal anchor.
                        causal_vars.push(entity.clone());
                    }
                    ClarifyRoute::AskUser { .. } | ClarifyRoute::DeclineAsk { .. } => {
                        // Genuinely ambiguous and human attention isn't warranted here — skip.
                        continue;
                    }
                    ClarifyRoute::NoClarification { .. } => {}
                }
            }

            let effect = entity.clone();
            let primary_action = cluster[0]
                .action
                .split_whitespace()
                .next()
                .unwrap_or("unknown");
            let eq = format!("{} ~ {}({})", effect, primary_action, causal_vars.join(", "));

            // Idempotency: skip if this exact equation was already coined.
            if existing_equations.contains(&eq) {
                continue;
            }

            let evidence_ids: Vec<Uuid> = cluster.iter().map(|r| r.id).collect();
            // Crude MDL proxy: more evidence per unit equation length ⇒ better compression.
            let mdl_score = evidence_ids.len() as f64 / (eq.len() as f64 / 20.0 + 1.0);

            let payload = LawPayload {
                equation: eq.clone(),
                causal_variables: causal_vars,
                effect_variable: effect,
                evidence_ids,
                mdl_score,
                domain: None,
                holds_under_intervention: true,
            };

            let metadata = serde_json::to_value(&payload).unwrap_or_default();
            let rec = MemoryRecord::new(
                MemoryType::Law,
                actor.to_string(),
                "extracted".to_string(),
                "law-store".to_string(),
                metadata,
            );

            // MemoryStore::add returns Result<()>, so capture the id before the move.
            let rec_id = rec.id;
            if store.add(rec).is_ok() {
                written.push(rec_id);
            }
        }

        written
    }

    /// Owned snapshots of this actor's surprising Intent records.
    fn collect_surprising<B: MemoryBackend>(
        store: &MemoryStore<B>,
        actor: &str,
    ) -> Vec<MemoryRecord> {
        store
            .all_by_type(MemoryType::Intent)
            .into_iter()
            .filter(|r| {
                r.actor == actor
                    && r.metadata
                        .get("was_surprising")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Set of equation strings already present on Law records (for idempotency).
    fn existing_law_equations<B: MemoryBackend>(store: &MemoryStore<B>) -> HashSet<String> {
        store
            .all_by_type(MemoryType::Law)
            .into_iter()
            .filter_map(|r| {
                r.metadata
                    .get("equation")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect()
    }
}
