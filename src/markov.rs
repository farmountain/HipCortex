use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// Simple first-order Markov chain for discrete states.
/// Transition counts are kept for the last `capacity` observations.
#[derive(Default)]
pub struct MarkovChain<T> {
    transitions: HashMap<T, HashMap<T, usize>>,
    order: VecDeque<(T, T)>,
    capacity: usize,
}

impl<T> MarkovChain<T>
where
    T: Eq + Hash + Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            transitions: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    /// Record a state transition from `current` to `next`.
    pub fn record_transition(&mut self, current: &T, next: &T) {
        self.order.push_back((current.clone(), next.clone()));
        let entry = self
            .transitions
            .entry(current.clone())
            .or_insert_with(HashMap::new);
        *entry.entry(next.clone()).or_insert(0) += 1;

        if self.order.len() > self.capacity {
            if let Some((c, n)) = self.order.pop_front() {
                if let Some(map) = self.transitions.get_mut(&c) {
                    if let Some(cnt) = map.get_mut(&n) {
                        *cnt -= 1;
                        if *cnt == 0 {
                            map.remove(&n);
                        }
                    }
                    if map.is_empty() {
                        self.transitions.remove(&c);
                    }
                }
            }
        }
    }

    /// Predict the most likely next state given the current state.
    pub fn predict_next(&self, current: &T) -> Option<T> {
        let map = self.transitions.get(current)?;
        map.iter()
            .max_by_key(|(_, v)| *v)
            .map(|(k, _)| k.clone())
    }
}
