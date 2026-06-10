/// Chain-of-Thought: append trace -> decay -> predict next state
use crate::segmented_buffer::SegmentedRingBuffer;
use crate::decay::{apply_decay, DecayType};
use crate::markov::MarkovChain;
use crate::poisson::PoissonBurst;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Debug)]
pub struct TemporalTrace<T> {
    pub id: uuid::Uuid,
    pub timestamp: SystemTime,
    pub data: T,
    pub relevance: f32,
    pub decay_factor: f32,
    pub last_access: SystemTime,
    pub decay_type: DecayType,
}

pub struct TemporalIndexer<T> {
    buffer: SegmentedRingBuffer<TemporalTrace<T>>,
    _capacity: usize,
    #[allow(dead_code)]  // Field reserved for future decay calculations
    decay_half_life: Duration,
    markov: Option<MarkovChain<T>>,
    poisson: PoissonBurst,
    last_state: Option<T>,
    // Intelligence layer hooks (optional, set via with_* builders)
    self_model: Option<Arc<crate::self_model::SelfModel>>,
    world_model: Option<Arc<crate::world_model_enhanced::WorldModelEnhanced>>,
    coherence: Option<Arc<crate::coherence::CoherenceChecker>>,
    // Health tracking
    op_count: u64,
    error_count: u64,
    total_latency_ms: f64,
}

impl<T: Clone> TemporalIndexer<T> {
    pub fn new(capacity: usize, decay_half_life_secs: u64) -> Self {
        Self {
            buffer: SegmentedRingBuffer::new(capacity, 64),
            _capacity: capacity,
            decay_half_life: Duration::from_secs(decay_half_life_secs),
            markov: None,
            poisson: PoissonBurst::new(0.1),
            last_state: None,
            self_model: None,
            world_model: None,
            coherence: None,
            op_count: 0,
            error_count: 0,
            total_latency_ms: 0.0,
        }
    }

    /// Attach self-model for operation gating and resource reporting.
    pub fn with_self_model(mut self, sm: Arc<crate::self_model::SelfModel>) -> Self {
        // Auto-register temporal operations so the self-model doesn't
        // reject them on first use.
        let _ = sm.register_capability(crate::self_model::CapabilityDescriptor {
            name: "temporal_insert".to_string(),
            description: "Insert temporal trace into index".to_string(),
            required_cpu_percent: 5.0,
            required_memory_mb: 10.0,
            limitations: vec![],
        });
        self.self_model = Some(sm);
        self
    }

    /// Attach world-model for transition observation.
    pub fn with_world_model(mut self, wm: Arc<crate::world_model_enhanced::WorldModelEnhanced>) -> Self {
        self.world_model = Some(wm);
        self
    }

    /// Attach coherence checker for entity validation.
    pub fn with_coherence(mut self, cc: Arc<crate::coherence::CoherenceChecker>) -> Self {
        self.coherence = Some(cc);
        self
    }

    pub fn insert(&mut self, trace: TemporalTrace<T>)
    where
        T: Eq + std::hash::Hash + Clone + std::fmt::Debug,
    {
        // 7.2: Self-model operation gating (allow if capability not yet registered)
        if let Some(ref sm) = self.self_model {
            let context = crate::self_model::DecisionContext::default_context();
            match sm.can_execute("temporal_insert", context) {
                Ok(decision) if !decision.should_execute => {
                    // Operation explicitly rejected by self-model; skip insert
                    return;
                }
                Err(_) => {
                    // Capability not registered — auto-register on first use and proceed
                    let _ = sm.register_capability(
                        crate::self_model::CapabilityDescriptor {
                            name: "temporal_insert".to_string(),
                            description: "Insert temporal trace".to_string(),
                            required_cpu_percent: 5.0,
                            required_memory_mb: 10.0,
                            limitations: vec![],
                        }
                    );
                }
                _ => {}
            }
        }

        let start = Instant::now();
        let prev_state = self.last_state.clone();
        let trace_id = trace.id;
        let trace_data = trace.data.clone();

        if let Some(m) = self.markov.as_mut() {
            if let Some(ref prev) = prev_state {
                m.record_transition(prev, &trace.data);
            }
            self.last_state = Some(trace.data.clone());
        }
        self.poisson.record_event();
        self.buffer.push_back(trace);

        let elapsed = start.elapsed();
        self.op_count += 1;
        self.total_latency_ms += elapsed.as_secs_f64() * 1000.0;

        // 7.3: Resource usage reporting
        if let Some(ref sm) = self.self_model {
            let usage = crate::self_model::ResourceUsage {
                cpu_percent: 0.0,
                memory_mb: self.buffer.len() as f64 / 1000.0,
                disk_io_mbps: 0.0,
                network_io_mbps: 0.0,
                timestamp: Instant::now(),
            };
            let _ = sm.record_resource_usage("temporal_insert", usage);
        }

        // 7.4: World-model transition observation
        if let Some(ref wm) = self.world_model {
            let from_state = prev_state
                .as_ref()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "none".to_string());
            let to_state = format!("{:?}", &trace_data);
            let _ = wm.observe_transition(from_state, "temporal_insert".to_string(), to_state);
        }

        // 7.5: Coherence entity validation (using trace UUID as entity ID)
        if let Some(ref cc) = self.coherence {
            let entity_id = trace_id.to_string();
            let _ = cc.check_entity(&entity_id);
        }
    }

    pub fn decay_and_prune(&mut self) {
        let now = SystemTime::now();
        self.buffer.retain(|trace| {
            let elapsed = now
                .duration_since(trace.last_access)
                .unwrap_or(Duration::ZERO);
            // Each trace can have its own decay factor which adjusts the global
            // half-life. This allows different memory types to fade at
            // different rates.
            let factor = if trace.decay_factor <= 0.0 {
                1.0
            } else {
                trace.decay_factor
            };
            let profile = match trace.decay_type {
                DecayType::Exponential { half_life } => {
                    DecayType::Exponential {
                        half_life: half_life.mul_f32(1.0 / factor),
                    }
                }
                DecayType::Linear { duration } => DecayType::Linear {
                    duration: duration.mul_f32(1.0 / factor),
                },
                DecayType::Custom(f) => DecayType::Custom(f),
            };
            let decayed = apply_decay(trace.relevance, elapsed, &profile);
            decayed > 0.01
        });
    }

    pub fn remove(&mut self, id: uuid::Uuid) -> bool {
        if let Some((seg, pos)) = self.buffer.position(|t| t.id == id) {
            self.buffer.remove_at(seg, pos);
            true
        } else {
            false
        }
    }

    pub fn get_trace(&self, id: uuid::Uuid) -> Option<&TemporalTrace<T>> {
        self.buffer.iter().find(|t| t.id == id)
    }

    pub fn access(&mut self, id: uuid::Uuid) -> Option<&mut TemporalTrace<T>> {
        let now = SystemTime::now();
        for trace in self.buffer.iter_mut() {
            if trace.id == id {
                trace.last_access = now;
                return Some(trace);
            }
        }
        None
    }

    pub fn get_recent(&self, n: usize) -> Vec<&TemporalTrace<T>> {
        let vec: Vec<&TemporalTrace<T>> = self.buffer.iter().collect();
        vec.into_iter().rev().take(n).collect()
    }

    pub fn enable_markov(&mut self, capacity: usize)
    where
        T: Eq + std::hash::Hash + Clone,
    {
        self.markov = Some(MarkovChain::new(capacity));
    }

    pub fn predict_next(&self, current: &T) -> Option<T>
    where
        T: Eq + std::hash::Hash + Clone,
    {
        self.markov.as_ref()?.predict_next(current)
    }

    pub fn is_bursty(&self) -> bool {
        self.poisson.is_bursty()
    }
}

impl<T> crate::health_reporter::HealthReporter for TemporalIndexer<T> {
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
        let buffer_fullness = self.buffer.len() as f64 / self._capacity.max(1) as f64;
        crate::self_model::ModuleHealth {
            latency_ms: avg_latency_ms,
            error_rate,
            resource_usage: buffer_fullness.min(1.0),
        }
    }

    fn health_module_name(&self) -> &'static str {
        "temporal_indexer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_insert_and_recent() {
        let mut idx = TemporalIndexer::new(1, 10);
        let trace = TemporalTrace {
            id: uuid::Uuid::new_v4(),
            timestamp: SystemTime::now(),
            data: 42u32,
            relevance: 1.0,
            decay_factor: 1.0,
            last_access: SystemTime::now(),
            decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
        };
        idx.insert(trace);
        assert_eq!(idx.get_recent(1).len(), 1);
    }

    #[test]
    fn segmented_buffer_wraps() {
        let mut idx = TemporalIndexer::new(3, 10);
        for _ in 0..5 {
            idx.insert(TemporalTrace {
                id: uuid::Uuid::new_v4(),
                timestamp: SystemTime::now(),
                data: 0u8,
                relevance: 1.0,
                decay_factor: 1.0,
                last_access: SystemTime::now(),
                decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
            });
        }
        assert_eq!(idx.get_recent(5).len(), 3);
    }
}
