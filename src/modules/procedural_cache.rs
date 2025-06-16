/// Chain-of-Thought: event -> match transition -> new state
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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

pub struct ProceduralCache {
    traces: HashMap<Uuid, ProceduralTrace>,
    transitions: Vec<FSMTransition>,
}

impl ProceduralCache {
    pub fn new() -> Self {
        Self {
            traces: HashMap::new(),
            transitions: vec![],
        }
    }

    pub fn add_trace(&mut self, trace: ProceduralTrace) {
        self.traces.insert(trace.id, trace);
    }

    pub fn add_transition(&mut self, transition: FSMTransition) {
        self.transitions.push(transition);
    }

    pub fn remove_trace(&mut self, trace_id: Uuid) -> bool {
        self.traces.remove(&trace_id).is_some()
    }

    pub fn reset_trace(&mut self, trace_id: Uuid) -> Option<()> {
        let trace = self.traces.get_mut(&trace_id)?;
        trace.current_state = FSMState::Start;
        trace.memory.clear();
        Some(())
    }

    pub fn advance(&mut self, trace_id: Uuid, condition: Option<&str>) -> Option<FSMState> {
        let trace = self.traces.get_mut(&trace_id)?;
        for trans in &self.transitions {
            if &trace.current_state == &trans.from {
                if let Some(c) = &trans.condition {
                    if let Some(cond) = condition {
                        if c == cond {
                            trace.current_state = trans.to.clone();
                            return Some(trace.current_state.clone());
                        }
                    }
                } else if condition.is_none() {
                    trace.current_state = trans.to.clone();
                    return Some(trace.current_state.clone());
                }
            }
        }
        None
    }

    pub fn advance_batch<'a, I>(
        &mut self,
        traces: I,
        condition: Option<&'a str>,
    ) -> Vec<(Uuid, FSMState)>
    where
        I: IntoIterator<Item = Uuid>,
    {
        let mut results = Vec::new();
        for id in traces {
            if let Some(state) = self.advance(id, condition) {
                results.push((id, state));
            }
        }
        results
    }

    pub fn get_trace(&self, trace_id: Uuid) -> Option<&ProceduralTrace> {
        self.traces.get(&trace_id)
    }

    pub fn save_checkpoint<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, &self.traces)?;
        Ok(())
    }

    pub fn load_checkpoint<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::new());
        }
        let file = std::fs::File::open(path)?;
        let traces: HashMap<Uuid, ProceduralTrace> = serde_json::from_reader(file)?;
        Ok(Self {
            traces,
            transitions: vec![],
        })
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
        let res = cache.advance_batch(vec![t1.id, t2.id], None);
        assert_eq!(res.len(), 2);
    }
}
