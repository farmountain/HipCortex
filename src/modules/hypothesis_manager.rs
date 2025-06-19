use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;

/// Simple hypothesis node with associated probability.
#[derive(Clone)]
pub struct Hypothesis<T> {
    pub state: T,
    pub probability: f32, // 0.0..=1.0
}

/// Maintains a forest of hypotheses with Bayesian updates and pruning.
#[derive(Default)]
pub struct HypothesisManager<T> {
    nodes: HashMap<usize, Hypothesis<T>>, // id -> hypothesis
    parent: HashMap<usize, usize>,        // child -> parent
    counter: usize,
}

impl<T: Clone> HypothesisManager<T> {
    /// Create a new empty manager.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            parent: HashMap::new(),
            counter: 0,
        }
    }

    /// Standard Bayesian probability update.
    ///
    /// `prior` is the prior probability of the hypothesis,
    /// `likelihood` is the probability of the evidence assuming the hypothesis.
    /// The complement probability P(E|¬H) is assumed to be `1 - likelihood`.
    pub fn update_probability(&self, prior: f32, likelihood: f32) -> f32 {
        (prior * likelihood) / ((prior * likelihood) + ((1.0 - prior) * (1.0 - likelihood)))
    }

    /// Add a root hypothesis and return its unique id.
    pub fn add_root(&mut self, state: T, probability: f32) -> usize {
        self.counter += 1;
        self.nodes
            .insert(self.counter, Hypothesis { state, probability });
        self.counter
    }

    /// Add a child hypothesis refining a parent given new evidence likelihood.
    pub fn add_child(&mut self, parent: usize, state: T, likelihood: f32) -> usize {
        let parent_prob = self
            .nodes
            .get(&parent)
            .expect("parent not found")
            .probability;
        self.counter += 1;
        let prob = self.update_probability(parent_prob, likelihood);
        self.nodes.insert(
            self.counter,
            Hypothesis {
                state,
                probability: prob,
            },
        );
        self.parent.insert(self.counter, parent);
        self.normalize_children(parent);
        self.counter
    }

    /// Retrieve hypothesis by id.
    pub fn get(&self, id: usize) -> Option<&Hypothesis<T>> {
        self.nodes.get(&id)
    }

    /// Return ids of all children for a parent.
    fn children_of(&self, parent: usize) -> Vec<usize> {
        self.parent
            .iter()
            .filter_map(|(c, p)| if *p == parent { Some(*c) } else { None })
            .collect()
    }

    /// Normalize sibling probabilities so they sum to the parent probability.
    fn normalize_children(&mut self, parent: usize) {
        let child_ids = self.children_of(parent);
        if child_ids.is_empty() {
            return;
        }
        let sum: f32 = child_ids
            .iter()
            .map(|id| self.nodes.get(id).unwrap().probability)
            .sum();
        if sum == 0.0 {
            return;
        }
        let parent_prob = self.nodes.get(&parent).unwrap().probability;
        for id in child_ids {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.probability = node.probability / sum * parent_prob;
            }
        }
    }

    /// Backtrack from a hypothesis to its root, returning the path of nodes.
    pub fn backtrack(&self, id: usize) -> Vec<&Hypothesis<T>> {
        let mut cur = id;
        let mut path = Vec::new();
        while let Some(node) = self.nodes.get(&cur) {
            path.push(node);
            if let Some(p) = self.parent.get(&cur).cloned() {
                cur = p;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }

    /// Remove a node and all of its descendants.
    fn remove_subtree(&mut self, id: usize) {
        let children = self.children_of(id);
        for c in children {
            self.remove_subtree(c);
        }
        self.nodes.remove(&id);
        self.parent.remove(&id);
    }

    /// Prune all hypotheses with probability below `threshold`.
    pub fn prune_low_probability(&mut self, threshold: f32) {
        let ids: Vec<usize> = self
            .nodes
            .iter()
            .filter_map(|(id, h)| {
                if h.probability < threshold {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        let mut affected = HashSet::new();
        for id in ids {
            if let Some(p) = self.parent.get(&id).cloned() {
                affected.insert(p);
            }
            self.remove_subtree(id);
        }
        for p in affected {
            self.normalize_children(p);
        }
    }

    fn is_leaf(&self, id: usize) -> bool {
        !self.parent.values().any(|p| *p == id)
    }

    /// Find path ending in the leaf with highest probability.
    pub fn best_path(&self) -> Vec<&Hypothesis<T>> {
        let leaf = self
            .nodes
            .keys()
            .filter(|id| self.is_leaf(**id))
            .max_by(|a, b| {
                self.nodes[a]
                    .probability
                    .partial_cmp(&self.nodes[b].probability)
                    .unwrap()
            });
        if let Some(id) = leaf {
            self.backtrack(*id)
        } else {
            Vec::new()
        }
    }

    /// Return the top `n` leaf paths ordered by probability.
    pub fn top_n_paths(&self, n: usize) -> Vec<Vec<&Hypothesis<T>>> {
        let mut leaves: Vec<(usize, f32)> = self
            .nodes
            .iter()
            .filter(|(id, _)| self.is_leaf(**id))
            .map(|(id, h)| (*id, h.probability))
            .collect();
        leaves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        leaves
            .into_iter()
            .take(n)
            .map(|(id, _)| self.backtrack(id))
            .collect()
    }

    /// Export the hypothesis tree in Graphviz `.dot` format.
    pub fn export_dot(&self, filename: &str) {
        let mut file = File::create(filename).expect("cannot create dot file");
        writeln!(file, "digraph Hypotheses {{").unwrap();
        for (id, node) in &self.nodes {
            writeln!(
                file,
                "    {} [label=\"{}:{:.3}\"];",
                id, id, node.probability
            )
            .unwrap();
        }
        for (child, parent) in &self.parent {
            writeln!(file, "    {} -> {};", parent, child).unwrap();
        }
        writeln!(file, "}}").unwrap();
    }

    /// Ensure every parent exists and the structure contains no cycles.
    pub fn assert_tree_validity(&self) {
        for (&_child, &parent) in &self.parent {
            assert!(self.nodes.contains_key(&parent));
            let mut seen = HashSet::new();
            let mut cur = parent;
            while let Some(p) = self.parent.get(&cur).cloned() {
                assert!(seen.insert(cur), "cycle detected");
                cur = p;
            }
        }
    }
}
