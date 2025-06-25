/// Chain-of-Thought: event -> match transition -> new state
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::backends::rustfsm_backend::RustFSMBackend;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FSMState {
    Start,
    Observe,
    Reason,
    Act,
    Reflexion,
    End,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FSMTransition {
    pub from: FSMState,
    pub to: FSMState,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralTrace {
    pub id: Uuid,
    pub current_state: FSMState,
    pub memory: HashMap<String, String>,
}

pub trait FSMBackend {
    fn add_trace(&mut self, trace: ProceduralTrace);
    fn add_transition(&mut self, transition: FSMTransition);
    fn advance(&mut self, trace_id: Uuid, condition: Option<&str>) -> Option<FSMState>;
    fn advance_batch(
        &mut self,
        trace_ids: &[Uuid],
        condition: Option<&str>,
    ) -> Vec<Option<FSMState>>;
    fn assert_fsm_invariants(&self);
    fn traces(&self) -> &HashMap<Uuid, ProceduralTrace>;
    fn traces_mut(&mut self) -> &mut HashMap<Uuid, ProceduralTrace>;
}

use crate::latent_map::LatentMapVersion;
/// Procedural cache storing FSM traces and versioned latent maps.
pub struct ProceduralCache<B: FSMBackend = RustFSMBackend> {
    backend: B,
    maps: HashMap<Uuid, Vec<LatentMapVersion>>,
    snapshot: HashMap<Uuid, Vec<LatentMapVersion>>, // immutable rollback
}

impl ProceduralCache<RustFSMBackend> {
    pub fn new() -> Self {
        Self {
            backend: RustFSMBackend::new(),
            maps: HashMap::new(),
            snapshot: HashMap::new(),
        }
    }

    pub fn load_checkpoint<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::new());
        }
        let file = std::fs::File::open(path)?;
        let traces: HashMap<Uuid, ProceduralTrace> = serde_json::from_reader(file)?;
        let mut backend = RustFSMBackend::new();
        for t in traces.values() {
            backend.add_trace(t.clone());
        }
        Ok(Self {
            backend,
            maps: HashMap::new(),
            snapshot: HashMap::new(),
        })
    }
}

impl<B: FSMBackend> ProceduralCache<B> {
    pub fn from_backend(backend: B) -> Self {
        Self {
            backend,
            maps: HashMap::new(),
            snapshot: HashMap::new(),
        }
    }

    pub fn add_trace(&mut self, trace: ProceduralTrace) {
        self.backend.add_trace(trace);
    }

    pub fn add_transition(&mut self, transition: FSMTransition) {
        self.backend.add_transition(transition);
    }

    pub fn remove_trace(&mut self, trace_id: Uuid) -> bool {
        self.backend.traces_mut().remove(&trace_id).is_some()
    }

    pub fn reset_trace(&mut self, trace_id: Uuid) -> Option<()> {
        let trace = self.backend.traces_mut().get_mut(&trace_id)?;
        trace.current_state = FSMState::Start;
        trace.memory.clear();
        Some(())
    }

    pub fn advance(&mut self, trace_id: Uuid, condition: Option<&str>) -> Option<FSMState> {
        self.backend.advance(trace_id, condition)
    }

    pub fn advance_batch(
        &mut self,
        trace_ids: &[Uuid],
        condition: Option<&str>,
    ) -> Vec<Option<FSMState>> {
        self.backend.advance_batch(trace_ids, condition)
    }

    pub fn get_trace(&self, trace_id: Uuid) -> Option<&ProceduralTrace> {
        self.backend.traces().get(&trace_id)
    }

    pub fn save_checkpoint<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self.backend.traces())?;
        Ok(())
    }

    pub fn assert_fsm_invariants(&self) {
        self.backend.assert_fsm_invariants();
    }

    /// Add a new latent map version for a trace with guardrail checks.
    pub fn add_map_version(&mut self, trace_id: Uuid, map: serde_json::Value, confidence: f32) {
        if crate::safety_guardrail::SAFETY_GUARDRAIL
            .lock()
            .unwrap()
            .check_precondition("map_update")
            .is_err()
        {
            return;
        }
        self.snapshot.insert(trace_id, self.maps.get(&trace_id).cloned().unwrap_or_default());
        self.maps
            .entry(trace_id)
            .or_default()
            .push(LatentMapVersion::new(map, confidence));
    }

    /// Retrieve the latest map version for a trace.
    pub fn latest_map(&self, trace_id: Uuid) -> Option<&LatentMapVersion> {
        self.maps.get(&trace_id).and_then(|v| v.last())
    }

    /// Revert to the last snapshot for the given trace.
    pub fn rollback_map(&mut self, trace_id: Uuid) {
        if let Some(prev) = self.snapshot.get(&trace_id).cloned() {
            self.maps.insert(trace_id, prev);
        }
    }

    /// Collapse map versions to the most confident one using the evaluator.
    pub fn collapse_maps(&mut self, trace_id: Uuid, threshold: f32) -> Option<LatentMapVersion> {
        use crate::latent_map_evaluator::LatentMapEvaluator;
        if let Some(list) = self.maps.get_mut(&trace_id) {
            LatentMapEvaluator::collapse(list, threshold)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_fsm_transition() {
        let mut cache = ProceduralCache::new();
        let trace = ProceduralTrace {
            id: Uuid::new_v4(),
            current_state: FSMState::Start,
            memory: HashMap::new(),
        };
        cache.add_trace(trace.clone());
        cache.add_transition(FSMTransition {
            from: FSMState::Start,
            to: FSMState::Observe,
            condition: None,
        });
        let new_state = cache.advance(trace.id, None);
        assert_eq!(new_state, Some(FSMState::Observe));
    }

    #[test]
    fn batch_transition() {
        let mut cache = ProceduralCache::new();
        let t1 = ProceduralTrace {
            id: Uuid::new_v4(),
            current_state: FSMState::Start,
            memory: HashMap::new(),
        };
        let t2 = ProceduralTrace {
            id: Uuid::new_v4(),
            current_state: FSMState::Start,
            memory: HashMap::new(),
        };
        cache.add_trace(t1.clone());
        cache.add_trace(t2.clone());
        cache.add_transition(FSMTransition {
            from: FSMState::Start,
            to: FSMState::Observe,
            condition: None,
        });
        let res = cache.advance_batch(&[t1.id, t2.id], None);
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn invariant_check() {
        let mut cache = ProceduralCache::new();
        cache.add_transition(FSMTransition {
            from: FSMState::Start,
            to: FSMState::End,
            condition: None,
        });
        cache.assert_fsm_invariants();
    }
}
