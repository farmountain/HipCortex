//! DigitalTwin — named façade over SimulationFork + ContinuousDynamics.
//!
//! Chain-of-thought: SimulationFork provides discrete rollouts; ContinuousDynamics provides
//! residual continuous flow between discrete steps. DigitalTwin composes both into a single
//! handle with sync policy. It does NOT import cognitive_state to avoid circular deps;
//! callers construct DigitalTwin from a fork obtained via CognitiveHandle::fork().

use crate::continuous_dynamics::{ContinuousDynamics, DynamicsContext};
use crate::persistence::MemoryBackend;
use crate::simulation_fork::{HybridRolloutResult, SimulationFork};
use crate::cognitive_state::CognitiveError;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SyncPolicy {
    /// Fork is read-only; no sync back to parent store.
    ReadOnly,
    /// Mutations write through to parent (future use).
    WriteThrough,
    /// Fork is fully isolated; must be explicitly merged.
    Isolated,
}

pub struct DigitalTwin<B: MemoryBackend + Send + Sync + 'static> {
    pub id: Uuid,
    pub fork: SimulationFork<B>,
    pub dynamics: ContinuousDynamics,
    pub sync_policy: SyncPolicy,
    pub created_at_tx: u64,
    trajectory: Vec<Vec<f64>>,
    t: f64,
    interventions: std::collections::HashMap<String, f64>,
}

impl<B: MemoryBackend + Send + Sync + 'static> DigitalTwin<B> {
    pub fn new(
        fork: SimulationFork<B>,
        dynamics: ContinuousDynamics,
        sync_policy: SyncPolicy,
        created_at_tx: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            fork,
            dynamics,
            sync_policy,
            created_at_tx,
            trajectory: Vec::new(),
            t: 0.0,
            interventions: std::collections::HashMap::new(),
        }
    }

    /// Apply a do(var=value) intervention on this twin's dynamics. Returns &mut self for chaining.
    pub fn fork_under_intervention(&mut self, var: &str, value: f64) -> &mut Self {
        self.interventions.insert(var.to_string(), value);
        self
    }

    pub fn pinned_interventions(&self) -> &std::collections::HashMap<String, f64> {
        &self.interventions
    }

    /// Advance twin by one action: record discrete step + integrate continuous dynamics.
    pub fn step(&mut self, action: &str) -> Result<Vec<f64>, CognitiveError> {
        self.fork.step(action)?;
        let prev_state = self.trajectory.last().cloned().unwrap_or_else(|| {
            vec![0.0; self.dynamics.dim()]
        });
        let ctx = DynamicsContext {
            entity_states: &[],
            resource_vec: &[],
            tx_cursor: 0,
        };
        let next = self.dynamics.step(self.t, &prev_state, &ctx)
            .map_err(|e| CognitiveError::StoreError(e))?;
        self.t += self.dynamics.dt;
        self.trajectory.push(next.clone());
        Ok(next)
    }

    /// Run a hybrid rollout over `actions`. Uses a clone of dynamics so twin state is not mutated.
    pub fn rollout(&mut self, actions: Vec<String>) -> Result<HybridRolloutResult, CognitiveError> {
        let dyn_clone = self.dynamics.clone();
        self.fork.rollout_hybrid(actions, 1.0, Some(dyn_clone))
    }

    /// All state vectors accumulated via step().
    pub fn trajectory(&self) -> &[Vec<f64>] {
        &self.trajectory
    }

    /// All records in the fork's isolated store.
    pub fn records(&self) -> Vec<crate::memory_record::MemoryRecord> {
        self.fork.all_records()
    }
}
