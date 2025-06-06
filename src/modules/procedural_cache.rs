use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FSMState {
    Start,
    Observe,
    Reason,
    Act,
    Reflexion,
    End,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct FSMTransition {
    pub from: FSMState,
    pub to: FSMState,
    pub condition: Option<String>,
}

#[derive(Debug, Clone)]
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

    pub fn get_trace(&self, trace_id: Uuid) -> Option<&ProceduralTrace> {
        self.traces.get(&trace_id)
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
}
