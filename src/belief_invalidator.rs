//! BeliefInvalidator — decays Belief confidence when contradicting evidence arrives.
//!
//! Called after every Temporal/Reflexion write in ReactEngine::run().
//! Uses token-overlap + negation-keyword heuristic (zero external deps).
//! When confidence < 0.2: writes a "belief_invalidated" Temporal marker.

use crate::memory_record::{MemoryRecord, MemoryType};
use crate::payloads::BeliefPayload;
use crate::persistence::MemoryBackend;
use uuid::Uuid;

const NEGATION_KEYWORDS: &[&str] = &[
    "not", "failed", "incorrect", "false", "error", "invalid", "wrong", "broken", "never",
    "no", "cannot", "fail", "failure",
];
const CONTRADICTION_THRESHOLD: f64 = 0.3;
const DECAY_FACTOR: f64 = 0.3;
const ARCHIVE_THRESHOLD: f32 = 0.2;

pub struct BeliefInvalidator;

impl BeliefInvalidator {
    /// Check `new_record` against all active Beliefs in `store`.
    /// Decays confidence for contradicted beliefs; returns IDs of beliefs that fell below threshold.
    pub fn process<B: MemoryBackend>(
        new_record: &MemoryRecord,
        store: &mut crate::memory_store::MemoryStore<B>,
    ) -> Vec<Uuid> {
        let content = format!(
            "{} {} {}",
            new_record.action, new_record.target,
            new_record.metadata.as_str().unwrap_or("")
        )
        .to_lowercase();

        let has_negation = NEGATION_KEYWORDS.iter().any(|kw| content.contains(kw));
        let content_tokens: std::collections::HashSet<&str> =
            content.split_whitespace().collect();

        // Collect snapshot to avoid borrow conflict on mutable store later.
        let beliefs: Vec<(Uuid, f32, String)> = store
            .all_by_type(MemoryType::Belief)
            .into_iter()
            .map(|r| {
                let proposition = serde_json::from_value::<BeliefPayload>(r.metadata.clone())
                    .map(|p| p.proposition)
                    .unwrap_or_default();
                (r.id, r.confidence, proposition)
            })
            .collect();

        let mut invalidated = Vec::new();

        for (belief_id, current_conf, proposition) in beliefs {
            let prop_lower = proposition.to_lowercase();
            let prop_tokens: std::collections::HashSet<&str> =
                prop_lower.split_whitespace().collect();

            let overlap = content_tokens.intersection(&prop_tokens).count();
            if overlap == 0 || !has_negation {
                continue;
            }

            let contradiction_score = (0.5 + (overlap as f64 / 10.0).min(0.5)).min(1.0);
            if contradiction_score < CONTRADICTION_THRESHOLD {
                continue;
            }

            let new_conf = ((current_conf as f64) - contradiction_score * DECAY_FACTOR)
                .max(0.0) as f32;

            let _ = store.update_record(belief_id, None, None, Some(new_conf), None, None);

            if new_conf < ARCHIVE_THRESHOLD {
                invalidated.push(belief_id);
                let marker = MemoryRecord::new(
                    MemoryType::Temporal,
                    "belief_invalidator".to_string(),
                    "belief_invalidated".to_string(),
                    proposition.clone(),
                    serde_json::json!({
                        "invalidated_belief_id": belief_id.to_string(),
                        "final_confidence": new_conf,
                        "contradiction_score": contradiction_score,
                    }),
                );
                let mut marker = marker;
                marker.derived_from = Some(belief_id);
                let _ = store.add(marker);
            }
        }

        invalidated
    }
}
