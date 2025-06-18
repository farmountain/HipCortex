/// Chain-of-Thought: append trace -> decay -> predict next state
use crate::segmented_buffer::SegmentedRingBuffer;
use crate::decay::{apply_decay, DecayType};
use crate::markov::MarkovChain;
use crate::poisson::PoissonBurst;
use std::time::{Duration, SystemTime};

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
    decay_half_life: Duration,
    markov: Option<MarkovChain<T>>,
    poisson: PoissonBurst,
    last_state: Option<T>,
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
        }
    }

    pub fn insert(&mut self, trace: TemporalTrace<T>)
    where
        T: Eq + std::hash::Hash + Clone,
    {
        if let Some(m) = self.markov.as_mut() {
            if let Some(prev) = self.last_state.as_ref() {
                m.record_transition(prev, &trace.data);
            }
            self.last_state = Some(trace.data.clone());
        }
        self.poisson.record_event();
        self.buffer.push_back(trace);
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
