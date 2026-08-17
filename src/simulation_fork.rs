//! Phase-2: Real Copy-on-Write SimulationFork.
//! Holds an isolated MemoryStore<InMemoryBackend> snapshot of the parent at fork time.
//! Mutations to the fork never touch the parent.

use crate::cognitive_state::{
    BeliefDistribution, BeliefSummary, CognitiveDelta, CognitiveError, CognitiveSnapshot,
    GoalSnapshot, ProvenanceSummary, SelfStateView, SkillSnapshot, TemporalView, WorldStateView,
};
use crate::memory_record::MemoryType;
use crate::memory_store::MemoryStore;
use crate::payloads::{BeliefPayload, GoalPayload, SkillPayload};
use crate::persistence::{InMemoryBackend, MemoryBackend};
use crate::tx_log::{TxKind, TxLog};
use uuid::Uuid;

pub struct SimulationFork<B: MemoryBackend + Send + Sync + 'static> {
    pub id: Uuid,
    pub base_tx: u64,
    pub created_at: std::time::Instant,
    store: MemoryStore<InMemoryBackend>,
    tx_log: TxLog,
    steps: Vec<String>,
    _marker: std::marker::PhantomData<B>,
}

impl<B: MemoryBackend + Send + Sync + 'static> SimulationFork<B> {
    /// Copy parent's live records into an isolated in-memory store.
    pub fn from_handle(
        handle: &crate::cognitive_state::CognitiveHandle<B>,
        base_tx: u64,
    ) -> Result<Self, CognitiveError> {
        let records = {
            let ms = handle.memory.lock().map_err(|_| CognitiveError::LockError)?;
            ms.all().to_vec()
        };
        let mut fork_store = MemoryStore::<InMemoryBackend>::new_in_memory();
        for r in records {
            fork_store
                .add(r)
                .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
        }
        let tmp = std::env::temp_dir().join(format!("hc-fork-{}.jsonl", Uuid::new_v4()));
        let tx_log = TxLog::open(tmp).map_err(CognitiveError::StoreError)?;
        Ok(Self {
            id: Uuid::new_v4(),
            base_tx,
            created_at: std::time::Instant::now(),
            store: fork_store,
            tx_log,
            steps: Vec::new(),
            _marker: std::marker::PhantomData,
        })
    }

    /// Record one action string in the fork's local tx log.
    pub fn step(&mut self, action: &str) -> Result<u64, CognitiveError> {
        if action.is_empty() {
            return Err(CognitiveError::DeltaInvalid("action required".into()));
        }
        self.steps.push(action.to_string());
        Ok(self.tx_log.append(TxKind::WorldModelObserve, vec![], "fork"))
    }

    /// Apply a CognitiveDelta to the fork's isolated store (AddMemory only in Phase 2).
    pub fn apply_delta(
        &mut self,
        delta: CognitiveDelta,
        actor: &str,
    ) -> Result<u64, CognitiveError> {
        match &delta {
            CognitiveDelta::AddMemory(r) => {
                self.store
                    .add(r.clone())
                    .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
            }
            _ => {
                return Err(CognitiveError::NotImplemented(format!(
                    "fork apply_delta {}",
                    delta.label()
                )));
            }
        }
        Ok(self.tx_log.append(TxKind::MemoryAdd, vec![], actor))
    }

    /// Build CognitiveSnapshot from fork's local records.
    /// World/self views zeroed — Phase 2 forks don't inherit parent WM state.
    pub fn snapshot(&self, actor: &str) -> Result<CognitiveSnapshot, CognitiveError> {
        let all = self.store.all();

        let temporal_recs: Vec<_> = all
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Temporal
                    && r.status == "active"
                    && (actor.is_empty() || r.actor == actor)
            })
            .collect();
        let temporal_span_ms = match temporal_recs.as_slice() {
            [] | [_] => 0,
            recs => {
                let oldest = recs.first().unwrap().timestamp;
                let newest = recs.last().unwrap().timestamp;
                (newest - oldest).num_milliseconds().max(0) as u64
            }
        };
        let temporal = TemporalView {
            record_count: temporal_recs.len(),
            recent_actions: temporal_recs
                .iter()
                .rev()
                .take(5)
                .map(|r| r.action.clone())
                .collect(),
            temporal_span_ms,
        };

        let goals: Vec<GoalSnapshot> = all
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Goal
                    && r.status == "active"
                    && (actor.is_empty() || r.actor == actor)
            })
            .filter_map(|r| {
                serde_json::from_value::<GoalPayload>(r.metadata.clone())
                    .ok()
                    .map(|p| GoalSnapshot {
                        id: r.id,
                        target_state: p.target_state,
                        status: p.status,
                        iteration: p.current_iteration,
                    })
            })
            .collect();

        let skills: Vec<SkillSnapshot> = all
            .iter()
            .filter(|r| r.record_type == MemoryType::Skill)
            .filter_map(|r| {
                serde_json::from_value::<SkillPayload>(r.metadata.clone())
                    .ok()
                    .map(|p| SkillSnapshot { id: r.id, procedure: p.procedure })
            })
            .collect();

        let belief_summaries: Vec<BeliefSummary> = all
            .iter()
            .filter(|r| {
                r.record_type == MemoryType::Belief
                    && r.status == "active"
                    && (actor.is_empty() || r.actor == actor)
            })
            .filter_map(|r| {
                serde_json::from_value::<BeliefPayload>(r.metadata.clone())
                    .ok()
                    .map(|p| BeliefSummary {
                        id: r.id,
                        proposition: p.proposition,
                        confidence: p.confidence,
                        epistemic_status: p.epistemic_status,
                    })
            })
            .collect();
        let confs: Vec<f32> = belief_summaries.iter().map(|b| b.confidence).collect();
        let mean_confidence = if confs.is_empty() {
            0.0
        } else {
            confs.iter().sum::<f32>() / confs.len() as f32
        };
        let beliefs = BeliefDistribution {
            count: belief_summaries.len(),
            mean_confidence,
            epistemic_entropy: 0.0,
            beliefs: belief_summaries,
        };

        let provenance = ProvenanceSummary {
            merkle_root_hex: self.store.merkle_root_hex(),
            record_count: self.store.record_count(),
            evidence_edge_count: self.store.evidence_edge_count(),
        };

        Ok(CognitiveSnapshot {
            id: Uuid::new_v4(),
            tx_cursor: self.tx_log.current_tx(),
            actor: actor.to_string(),
            temporal,
            world: WorldStateView { node_count: 0, edge_count: 0, dag_verified: true },
            self_model: SelfStateView {
                calibration_score: 1.0,
                prediction_error_ewma: 0.0,
                consolidation_pressure: 0.0,
                epistemic_entropy: 0.0,
                healthy: true,
            },
            goals,
            skills,
            beliefs,
            provenance,
        })
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > 60
    }

    pub fn fork_tx(&self) -> u64 {
        self.tx_log.current_tx()
    }

    pub fn steps_taken(&self) -> usize {
        self.steps.len()
    }
}
