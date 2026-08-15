/// Chain-of-Thought: event -> match transition -> new state
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::backends::rustfsm_backend::RustFSMBackend;

#[path = "procedural_cache/skill_compiler.rs"]
pub mod skill_compiler;
pub use skill_compiler::{SkillCompiler, SkillTemplate, SkillTemplateStep};

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
    fn get_transitions(&self) -> Vec<FSMTransition>;
}

use crate::latent_map::LatentMapVersion;
use std::sync::Arc;
use std::time::Instant;

/// Procedural cache storing FSM traces and versioned latent maps.
pub struct ProceduralCache<B: FSMBackend = RustFSMBackend> {
    backend: B,
    maps: HashMap<Uuid, Vec<LatentMapVersion>>,
    snapshot: HashMap<Uuid, Vec<LatentMapVersion>>, // immutable rollback
    // Intelligence layer hooks (optional, set via with_* builders)
    self_model: Option<Arc<crate::self_model::SelfModel>>,
    world_model: Option<Arc<crate::world_model_enhanced::WorldModelEnhanced>>,
    coherence: Option<Arc<crate::coherence::CoherenceChecker>>,
    // Health tracking
    op_count: u64,
    error_count: u64,
    total_latency_ms: f64,
}

impl ProceduralCache<RustFSMBackend> {
    pub fn new() -> Self {
        Self {
            backend: RustFSMBackend::new(),
            maps: HashMap::new(),
            snapshot: HashMap::new(),
            self_model: None,
            world_model: None,
            coherence: None,
            op_count: 0,
            error_count: 0,
            total_latency_ms: 0.0,
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
            self_model: None,
            world_model: None,
            coherence: None,
            op_count: 0,
            error_count: 0,
            total_latency_ms: 0.0,
        })
    }
}

impl<B: FSMBackend> ProceduralCache<B> {
    pub fn from_backend(backend: B) -> Self {
        Self {
            backend,
            maps: HashMap::new(),
            snapshot: HashMap::new(),
            self_model: None,
            world_model: None,
            coherence: None,
            op_count: 0,
            error_count: 0,
            total_latency_ms: 0.0,
        }
    }

    /// Attach self-model for operation gating before FSM transitions.
    pub fn with_self_model(mut self, sm: Arc<crate::self_model::SelfModel>) -> Self {
        let _ = sm.register_capability(crate::self_model::CapabilityDescriptor {
            name: "fsm_advance".to_string(),
            description: "Advance FSM trace to next state".to_string(),
            required_cpu_percent: 5.0,
            required_memory_mb: 10.0,
            limitations: vec![],
        });
        self.self_model = Some(sm);
        self
    }

    /// Attach world-model for state transition observation.
    pub fn with_world_model(
        mut self,
        wm: Arc<crate::world_model_enhanced::WorldModelEnhanced>,
    ) -> Self {
        self.world_model = Some(wm);
        self
    }

    /// Attach coherence checker for pre-transition consistency validation.
    pub fn with_coherence(mut self, cc: Arc<crate::coherence::CoherenceChecker>) -> Self {
        self.coherence = Some(cc);
        self
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
        // 9.2: Self-model operation gating
        if let Some(ref sm) = self.self_model {
            let context = crate::self_model::DecisionContext::default_context();
            match sm.can_execute("fsm_advance", context) {
                Ok(decision) if !decision.should_execute => return None,
                Err(_) => {
                    let _ = sm.register_capability(crate::self_model::CapabilityDescriptor {
                        name: "fsm_advance".to_string(),
                        description: "Advance FSM trace".to_string(),
                        required_cpu_percent: 5.0,
                        required_memory_mb: 10.0,
                        limitations: vec![],
                    });
                }
                _ => {}
            }
        }

        // 9.5: Coherence pre-transition check
        if let Some(ref cc) = self.coherence {
            let context = format!("fsm_advance:{}", trace_id);
            if cc.gate_write(&context).is_err() {
                return None;
            }
        }

        let prev_state = self
            .backend
            .traces()
            .get(&trace_id)
            .map(|t| t.current_state.clone());
        let start = Instant::now();
        let result = self.backend.advance(trace_id, condition);
        let elapsed = start.elapsed();

        self.op_count += 1;
        self.total_latency_ms += elapsed.as_secs_f64() * 1000.0;

        // 9.4: World-model transition observation
        if let Some(ref wm) = self.world_model {
            if let (Some(ref prev), Some(ref next)) = (&prev_state, &result) {
                let from = format!("{:?}", prev);
                let to = format!("{:?}", next);
                let _ = wm.observe_transition(from, format!("condition:{:?}", condition), to);
            }
        }

        result
    }

    pub fn advance_batch(
        &mut self,
        trace_ids: &[Uuid],
        condition: Option<&str>,
    ) -> Vec<Option<FSMState>> {
        // 9.3: Health-based transition filtering — skip batch if health < 0.5
        if let Some(ref sm) = self.self_model {
            if let Ok(healthy) = sm.is_healthy() {
                if !healthy {
                    return vec![None; trace_ids.len()];
                }
            }
        }

        let start = Instant::now();
        let results = self.backend.advance_batch(trace_ids, condition);
        let elapsed = start.elapsed();

        self.op_count += results.len() as u64;
        self.total_latency_ms += elapsed.as_secs_f64() * 1000.0;

        // 9.4: World-model observation for batch transitions
        if let Some(ref wm) = self.world_model {
            for (i, result) in results.iter().enumerate() {
                if let Some(ref next) = result {
                    if let Some(trace_id) = trace_ids.get(i) {
                        let to = format!("{:?}", next);
                        let _ = wm.observe_transition(
                            format!("batch_trace:{}", trace_id),
                            format!("condition:{:?}", condition),
                            to,
                        );
                    }
                }
            }
        }

        results
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

    pub fn get_transitions(&self) -> Vec<FSMTransition> {
        self.backend.get_transitions()
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
        self.snapshot.insert(
            trace_id,
            self.maps.get(&trace_id).cloned().unwrap_or_default(),
        );
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

impl<B: FSMBackend> crate::health_reporter::HealthReporter for ProceduralCache<B> {
    fn report_health(&self) -> crate::self_model::ModuleHealth {
        let avg_latency_ms = if self.op_count > 0 {
            self.total_latency_ms / self.op_count as f64
        } else {
            0.0
        };
        let error_rate = if self.op_count > 0 {
            self.error_count as f64 / self.op_count as f64
        } else {
            0.0
        };
        let trace_count = self.backend.traces().len();
        let resource_usage = (trace_count as f64 / 10_000.0).min(1.0);
        crate::self_model::ModuleHealth {
            latency_ms: avg_latency_ms,
            error_rate,
            resource_usage,
        }
    }

    fn health_module_name(&self) -> &'static str {
        "procedural_cache"
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
