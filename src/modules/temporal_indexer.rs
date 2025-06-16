/// Chain-of-Thought: append trace -> decay -> predict next state
use crate::segmented_buffer::SegmentedRingBuffer;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct TemporalTrace<T> {
    pub id: uuid::Uuid,
    pub timestamp: SystemTime,
    pub data: T,
    pub relevance: f32,
    pub decay_factor: f32,
    pub last_access: SystemTime,
}

pub struct TemporalIndexer<T> {
    buffer: SegmentedRingBuffer<TemporalTrace<T>>,
    _capacity: usize,
    decay_half_life: Duration,
}

impl<T> TemporalIndexer<T> {
    pub fn new(capacity: usize, decay_half_life_secs: u64) -> Self {
        Self {
            buffer: SegmentedRingBuffer::new(capacity, 64),
            _capacity: capacity,
            decay_half_life: Duration::from_secs(decay_half_life_secs),
        }
    }

    pub fn insert(&mut self, trace: TemporalTrace<T>) {
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
            let half_life = self.decay_half_life.mul_f32(1.0 / factor);
            let decay = (-((elapsed.as_secs_f32() / half_life.as_secs_f32())
                * std::f32::consts::LN_2))
                .exp2();
            let decayed_relevance = trace.relevance * decay;
            decayed_relevance > 0.01
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
            });
        }
        assert_eq!(idx.get_recent(5).len(), 3);
    }
}
