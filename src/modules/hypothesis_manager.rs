use std::collections::HashMap;

/// Maintains a quantized tree of hypothesis states for backtracking.
#[derive(Default)]
pub struct HypothesisManager<T> {
    states: HashMap<usize, T>,
    parent: HashMap<usize, usize>,
    counter: usize,
}

impl<T: Clone> HypothesisManager<T> {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            parent: HashMap::new(),
            counter: 0,
        }
    }

    /// Add a root state, returning its id.
    pub fn add_root(&mut self, state: T) -> usize {
        self.counter += 1;
        self.states.insert(self.counter, state);
        self.counter
    }

    /// Add a child state referencing a parent id.
    pub fn add_child(&mut self, parent: usize, state: T) -> usize {
        self.counter += 1;
        self.states.insert(self.counter, state);
        self.parent.insert(self.counter, parent);
        self.counter
    }

    pub fn get(&self, id: usize) -> Option<&T> {
        self.states.get(&id)
    }

    /// Backtrack from given id to root, returning the path of states.
    pub fn backtrack(&self, id: usize) -> Vec<&T> {
        let mut cur = id;
        let mut path = Vec::new();
        while let Some(state) = self.states.get(&cur) {
            path.push(state);
            if let Some(p) = self.parent.get(&cur) {
                cur = *p;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }
}
