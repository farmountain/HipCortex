use std::collections::VecDeque;
use std::time::{SystemTime, Duration};

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
    buffer: VecDeque<TemporalTrace<T>>,
    capacity: usize,
    decay_half_life: Duration,
}

impl<T> TemporalIndexer<T> {
    pub fn new(capacity: usize, decay_half_life_secs: u64) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            decay_half_life: Duration::from_secs(decay_half_life_secs),
        }
    }

    pub fn insert(&mut self, trace: TemporalTrace<T>) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(trace);
    }

    pub fn decay_and_prune(&mut self) {
        let now = SystemTime::now();
        self.buffer.retain(|trace| {
            let elapsed = now.duration_since(trace.last_access).unwrap_or(Duration::ZERO);
            let decay = (-((elapsed.as_secs_f32() / self.decay_half_life.as_secs_f32()) * std::f32::consts::LN_2)).exp2();
            let decayed_relevance = trace.relevance * decay;
            decayed_relevance > 0.01
        });
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
        self.buffer.iter().rev().take(n).collect()
    }
}
