//! AbstractionGate — validates evidence clusters before abstraction promotion.
//!
//! Rules (all must pass):
//! 1. evidence_ids.len() >= MIN_EVIDENCE (4)
//! 2. Proposition not already in existing_propositions (no duplicate)
//! 3. At least one evidence record is Temporal or Reflexion (grounded in observation)
//!
//! After validation, `elevate()` sets JtmsLabel::In + EpistemicStatus::Confirmed.

use std::collections::HashSet;
use uuid::Uuid;

use crate::memory_record::MemoryType;
use crate::memory_store::MemoryStore;
use crate::payloads::{BeliefPayload, EpistemicStatus, JtmsLabel};
use crate::persistence::MemoryBackend;

pub const MIN_EVIDENCE: usize = 4;

pub struct AbstractionResult {
    pub valid: bool,
    pub reason: String,
}

pub struct AbstractionGate;

impl AbstractionGate {
    pub fn validate<B: MemoryBackend>(
        evidence_ids: &[Uuid],
        proposition: &str,
        existing_propositions: &HashSet<String>,
        store: &MemoryStore<B>,
    ) -> AbstractionResult {
        if evidence_ids.len() < MIN_EVIDENCE {
            return AbstractionResult {
                valid: false,
                reason: format!(
                    "insufficient evidence: {} < {}",
                    evidence_ids.len(),
                    MIN_EVIDENCE
                ),
            };
        }

        if existing_propositions.contains(proposition) {
            return AbstractionResult {
                valid: false,
                reason: "duplicate proposition".into(),
            };
        }

        let has_observational_ground = evidence_ids.iter()
            .filter_map(|id| store.find_by_id(*id))
            .any(|r| {
                r.record_type == MemoryType::Temporal
                    || r.record_type == MemoryType::Reflexion
            });

        if !has_observational_ground {
            return AbstractionResult {
                valid: false,
                reason: "no Temporal/Reflexion grounding in evidence".into(),
            };
        }

        AbstractionResult {
            valid: true,
            reason: "passed all gates".into(),
        }
    }

    /// Elevate a validated belief: assert JtmsLabel::In + EpistemicStatus::Confirmed.
    pub fn elevate<B: MemoryBackend>(
        store: &mut MemoryStore<B>,
        belief_id: Uuid,
    ) -> Result<(), String> {
        let record = store.find_by_id(belief_id).ok_or("belief not found")?.clone();
        let mut payload: BeliefPayload = serde_json::from_value(record.metadata.clone())
            .map_err(|e| e.to_string())?;
        payload.jtms_label = JtmsLabel::In;
        payload.epistemic_status = EpistemicStatus::Confirmed;
        let metadata = serde_json::to_value(&payload).map_err(|e| e.to_string())?;
        store
            .update_record(belief_id, None, None, None, None, Some(metadata))
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
