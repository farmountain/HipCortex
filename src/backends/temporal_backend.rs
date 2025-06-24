use crate::procedural_cache::{FSMBackend, FSMState, FSMTransition, ProceduralTrace};
use std::collections::HashMap;
use uuid::Uuid;

/// Simple in-memory FSM backend with rollback support.
/// Transitions are stored in a `(from, condition)` map and each trace
/// keeps its history for backtracking.
pub struct TemporalFSMBackend {
    traces: HashMap<Uuid, ProceduralTrace>,
    transitions: HashMap<(FSMState, Option<String>), FSMState>,
    history: HashMap<Uuid, Vec<FSMState>>, // state history per trace
}

impl TemporalFSMBackend {
    pub fn new() -> Self {
        Self {
            traces: HashMap::new(),
            transitions: HashMap::new(),
            history: HashMap::new(),
        }
    }

    /// Store a trace and initialize its history.
    pub fn store_trace(&mut self, trace: ProceduralTrace) {
        self.history
            .entry(trace.id)
            .or_insert_with(|| vec![trace.current_state.clone()]);
        self.traces.insert(trace.id, trace);
    }

    /// Advance a trace based on the current state and optional condition.
    pub fn advance_trace(&mut self, trace_id: Uuid, cond: Option<&str>) -> Option<FSMState> {
        let trace = self.traces.get_mut(&trace_id)?;
        let key = (trace.current_state.clone(), cond.map(|c| c.to_string()));
        let next = self.transitions.get(&key)?.clone();
        trace.current_state = next.clone();
        self.history.entry(trace_id).or_default().push(next.clone());
        Some(next)
    }

    /// Roll back a trace to the previous state.
    pub fn rollback_trace(&mut self, trace_id: Uuid) -> Option<FSMState> {
        let hist = self.history.get_mut(&trace_id)?;
        if hist.len() <= 1 {
            return None;
        }
        hist.pop();
        let prev = hist.last()?.clone();
        let trace = self.traces.get_mut(&trace_id)?;
        trace.current_state = prev.clone();
        Some(prev)
    }
}

impl FSMBackend for TemporalFSMBackend {
    fn add_trace(&mut self, trace: ProceduralTrace) {
        self.store_trace(trace);
    }

    fn add_transition(&mut self, transition: FSMTransition) {
        self.transitions
            .insert((transition.from, transition.condition), transition.to);
    }

    fn advance(&mut self, trace_id: Uuid, condition: Option<&str>) -> Option<FSMState> {
        self.advance_trace(trace_id, condition)
    }

    fn advance_batch(&mut self, trace_ids: &[Uuid], condition: Option<&str>) -> Vec<Option<FSMState>> {
        trace_ids
            .iter()
            .map(|id| self.advance_trace(*id, condition))
            .collect()
    }

    fn assert_fsm_invariants(&self) {}

    fn traces(&self) -> &HashMap<Uuid, ProceduralTrace> {
        &self.traces
    }

    fn traces_mut(&mut self) -> &mut HashMap<Uuid, ProceduralTrace> {
        &mut self.traces
    }
}
