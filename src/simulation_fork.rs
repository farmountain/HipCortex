//! Phase-2 stub. Real copy-on-write digital-twin semantics ship with the
//! DigitalTwin spec. All methods return NotImplemented until then.

use crate::cognitive_state::CognitiveError;
use crate::persistence::MemoryBackend;

pub struct SimulationFork<B: MemoryBackend + Send + Sync + 'static> {
    _marker: std::marker::PhantomData<B>,
}

impl<B: MemoryBackend + Send + Sync + 'static> SimulationFork<B> {
    pub(crate) fn new_stub() -> Self {
        Self { _marker: std::marker::PhantomData }
    }

    pub fn step(&self, _action: &str) -> Result<(), CognitiveError> {
        Err(CognitiveError::NotImplemented("SimulationFork::step (Phase 2)".into()))
    }

    pub fn rollout(&self, _steps: usize) -> Result<Vec<String>, CognitiveError> {
        Err(CognitiveError::NotImplemented("SimulationFork::rollout (Phase 2)".into()))
    }
}
