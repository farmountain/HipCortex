use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::world_model_enhanced::TransitionModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorNode {
    pub id: Uuid,
    pub state_id: String,
    pub action_taken: Option<String>,
    pub visit_count: usize,
    pub total_reward: f64,
    pub children: Vec<Uuid>,
    pub parent: Option<Uuid>,
}

impl SimulatorNode {
    pub fn new(state_id: String, action_taken: Option<String>, parent: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            state_id,
            action_taken,
            visit_count: 0,
            total_reward: 0.0,
            children: Vec::new(),
            parent,
        }
    }

    pub fn ucb1_score(&self, parent_visits: usize, exploration_constant: f64) -> f64 {
        if self.visit_count == 0 {
            return f64::INFINITY;
        }
        let exploitation = self.total_reward / (self.visit_count as f64);
        let exploration = exploration_constant * ((parent_visits as f64).ln() / (self.visit_count as f64)).sqrt();
        exploitation + exploration
    }
}

pub struct MctsSimulator {
    pub nodes: HashMap<Uuid, SimulatorNode>,
    pub root_id: Uuid,
    pub exploration_constant: f64,
}

impl MctsSimulator {
    pub fn new(root_state: String, exploration_constant: f64) -> Self {
        let root = SimulatorNode::new(root_state, None, None);
        let root_id = root.id;
        let mut nodes = HashMap::new();
        nodes.insert(root_id, root);
        Self {
            nodes,
            root_id,
            exploration_constant,
        }
    }

    /// Search trajectory rollout engine using UCB1 branch selection and `TransitionModel::predict()`.
    pub fn search(
        &mut self,
        transition_model: &TransitionModel,
        available_actions: &[String],
        iterations: usize,
        max_depth: usize,
        reward_fn: impl Fn(&str) -> f64,
    ) -> Result<String, String> {
        if available_actions.is_empty() {
            return Err("No actions available for MCTS search".to_string());
        }

        for _ in 0..iterations {
            // 1. Selection & Expansion
            let mut current_id = self.root_id;
            let mut depth = 0;

            loop {
                if depth >= max_depth {
                    break;
                }
                depth += 1;

                let (children_ids, is_leaf) = {
                    let node = self.nodes.get(&current_id).ok_or("Node not found")?;
                    (node.children.clone(), node.children.is_empty())
                };

                if is_leaf {
                    let node_state = self.nodes.get(&current_id).unwrap().state_id.clone();
                    for action in available_actions {
                        let pred = transition_model.predict(&node_state, action);
                        let next_state = if let Ok(p) = pred {
                            p.probabilities
                                .iter()
                                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                                .map(|(k, _)| k.clone())
                                .unwrap_or_else(|| format!("{}_next", node_state))
                        } else {
                            format!("{}_after_{}", node_state, action)
                        };

                        let child = SimulatorNode::new(next_state, Some(action.clone()), Some(current_id));
                        let child_id = child.id;
                        self.nodes.get_mut(&current_id).unwrap().children.push(child_id);
                        self.nodes.insert(child_id, child);
                    }
                    break;
                } else {
                    let parent_visits = self.nodes.get(&current_id).unwrap().visit_count.max(1);
                    let mut best_child = current_id;
                    let mut max_ucb = f64::NEG_INFINITY;
                    for child_id in &children_ids {
                        if let Some(child) = self.nodes.get(child_id) {
                            let score = child.ucb1_score(parent_visits, self.exploration_constant);
                            if score > max_ucb {
                                max_ucb = score;
                                best_child = *child_id;
                            }
                        }
                    }
                    if best_child == current_id {
                        break;
                    }
                    current_id = best_child;
                }
            }

            // 2. Rollout (Simulate trajectory)
            let mut sim_state = self.nodes.get(&current_id).unwrap().state_id.clone();
            let mut total_rollout_reward = reward_fn(&sim_state);
            for _ in depth..max_depth {
                let action = &available_actions[0];
                if let Ok(pred) = transition_model.predict(&sim_state, action) {
                    if let Some((next_s, _)) = pred
                        .probabilities
                        .iter()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    {
                        sim_state = next_s.clone();
                    }
                }
                total_rollout_reward += reward_fn(&sim_state);
            }

            // 3. Backpropagation
            let mut backprop_id = Some(current_id);
            while let Some(nid) = backprop_id {
                if let Some(node) = self.nodes.get_mut(&nid) {
                    node.visit_count += 1;
                    node.total_reward += total_rollout_reward;
                    backprop_id = node.parent;
                } else {
                    break;
                }
            }
        }

        let root = self.nodes.get(&self.root_id).ok_or("Root node lost")?;
        let mut best_action = None;
        let mut max_visits = 0;
        for child_id in &root.children {
            if let Some(child) = self.nodes.get(child_id) {
                if child.visit_count >= max_visits {
                    max_visits = child.visit_count;
                    best_action = child.action_taken.clone();
                }
            }
        }

        best_action.ok_or_else(|| "No best action found".to_string())
    }
}
