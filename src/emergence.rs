//! EmergenceDetector — auto-promotes recurring patterns in Temporal records into Beliefs.
//!
//! After every TRIGGER_EVERY Temporal writes, scans the last WINDOW records.
//! A token appearing in >= DENSITY records synthesises a new Belief with evidence pointers.
//! Uses SafetyGuardrail (via store.add) — no raw writes.

use crate::memory_record::{MemoryRecord, MemoryType};
use crate::payloads::BeliefPayload;
use crate::persistence::MemoryBackend;
use uuid::Uuid;

const WINDOW: usize = 50;
const DENSITY: usize = 5;
const TRIGGER_EVERY: u32 = 10;

// Stop-words excluded from pattern matching
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "in", "of", "to", "and", "or", "for",
    "at", "on", "it", "as", "by", "be", "with", "from", "that", "this",
];

pub struct EmergenceDetector {
    pub temporal_write_count: u32,
}

impl EmergenceDetector {
    pub fn new() -> Self {
        Self { temporal_write_count: 0 }
    }

    /// Call after each Temporal write. Returns newly created Belief record IDs.
    pub fn on_temporal_write<B: MemoryBackend>(
        &mut self,
        store: &mut crate::memory_store::MemoryStore<B>,
        actor: &str,
    ) -> Vec<Uuid> {
        self.temporal_write_count += 1;
        if self.temporal_write_count % TRIGGER_EVERY != 0 {
            return Vec::new();
        }
        Self::detect(store, actor)
    }

    /// Scan the last WINDOW Temporal records; synthesise Beliefs for recurring tokens.
    pub fn detect<B: MemoryBackend>(
        store: &mut crate::memory_store::MemoryStore<B>,
        actor: &str,
    ) -> Vec<Uuid> {
        let temporals: Vec<(Uuid, String, String)> = store
            .all_by_type(MemoryType::Temporal)
            .into_iter()
            .rev()
            .take(WINDOW)
            .map(|r| {
                let content = format!(
                    "{} {}",
                    r.action,
                    r.metadata.get("thought").and_then(|v| v.as_str()).unwrap_or("")
                )
                .to_lowercase();
                (r.id, content, r.target.clone())
            })
            .collect();

        if temporals.len() < DENSITY {
            return Vec::new();
        }

        // Count token → record IDs
        let mut token_evidence: std::collections::HashMap<String, Vec<Uuid>> =
            std::collections::HashMap::new();

        for (record_id, content, _) in &temporals {
            for token in content.split_whitespace() {
                let t = token.trim_matches(|c: char| !c.is_alphanumeric());
                if t.len() < 3 || STOP_WORDS.contains(&t) {
                    continue;
                }
                token_evidence.entry(t.to_string()).or_default().push(*record_id);
            }
        }

        // Collect existing belief propositions to avoid duplicates
        let existing: std::collections::HashSet<String> = store
            .all_by_type(MemoryType::Belief)
            .into_iter()
            .filter_map(|r| {
                serde_json::from_value::<BeliefPayload>(r.metadata.clone())
                    .ok()
                    .map(|p| p.proposition)
            })
            .collect();

        let mut created = Vec::new();

        for (token, evidence_ids) in token_evidence {
            if evidence_ids.len() < DENSITY {
                continue;
            }
            let proposition = format!("pattern: {}", token);
            if existing.contains(&proposition) {
                continue;
            }

            let confidence = (evidence_ids.len() as f64 / DENSITY as f64).min(1.0) as f32;
            let payload = BeliefPayload {
                proposition: proposition.clone(),
                justification: format!(
                    "Emerged from {} Temporal records (EmergenceDetector)",
                    evidence_ids.len()
                ),
                confidence,
                causal_source_ids: evidence_ids.clone(),
                ..Default::default()
            };

            let mut belief = MemoryRecord::new(
                MemoryType::Belief,
                actor.to_string(),
                "emerge".to_string(),
                proposition.clone(),
                serde_json::to_value(&payload).unwrap_or_default(),
            );
            belief.evidence = evidence_ids;

            let id = belief.id;
            if store.add(belief).is_ok() {
                created.push(id);
            }
        }

        created
    }
}
