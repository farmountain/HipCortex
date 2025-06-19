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

pub struct ProceduralCache<B: FSMBackend = RustFSMBackend> {
    backend: B,
}

impl ProceduralCache<RustFSMBackend> {
    pub fn new() -> Self {
        Self {
            backend: RustFSMBackend::new(),
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
        Ok(Self { backend })
    }
}

impl<B: FSMBackend> ProceduralCache<B> {
    pub fn from_backend(backend: B) -> Self {
        Self { backend }
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
