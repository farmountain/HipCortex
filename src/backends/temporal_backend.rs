#[cfg(feature = "temporal_backend")]
use crate::procedural_cache::{FSMBackend, FSMState, FSMTransition, ProceduralTrace};
#[cfg(feature = "temporal_backend")]
use std::collections::HashMap;
#[cfg(feature = "temporal_backend")]
use temporalio::WorkflowClient;
#[cfg(feature = "temporal_backend")]
use uuid::Uuid;

#[cfg(feature = "temporal_backend")]
pub struct TemporalFSMBackend {
    client: WorkflowClient,
    traces: HashMap<Uuid, ProceduralTrace>,
}

#[cfg(feature = "temporal_backend")]
impl TemporalFSMBackend {
    pub fn new(client: WorkflowClient) -> Self {
        Self {
            client,
            traces: HashMap::new(),
        }
    }
}

#[cfg(feature = "temporal_backend")]
impl FSMBackend for TemporalFSMBackend {
    fn add_trace(&mut self, trace: ProceduralTrace) {
        self.traces.insert(trace.id, trace);
    }
    fn add_transition(&mut self, _transition: FSMTransition) { /* transitions handled by workflow */
    }
    fn advance(&mut self, _trace_id: Uuid, _condition: Option<&str>) -> Option<FSMState> {
        None
    }
    fn advance_batch(
        &mut self,
        _trace_ids: &[Uuid],
        _condition: Option<&str>,
    ) -> Vec<Option<FSMState>> {
        Vec::new()
    }
    fn assert_fsm_invariants(&self) {}
    fn traces(&self) -> &HashMap<Uuid, ProceduralTrace> {
        &self.traces
    }
    fn traces_mut(&mut self) -> &mut HashMap<Uuid, ProceduralTrace> {
        &mut self.traces
    }
}
