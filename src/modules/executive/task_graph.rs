use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    Active,
    Completed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: Uuid,
    pub summary: String,
    pub preconditions: HashMap<String, String>,
    pub effects: HashMap<String, String>,
    pub state: TaskState,
    pub cost: f64,
}

#[derive(Debug)]
pub struct TaskGraph {
    pub graph: DiGraph<TaskNode, ()>,
    pub node_index_map: HashMap<Uuid, NodeIndex>,
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct PlanningState {
    current_beliefs: HashMap<String, String>,
    path_cost: f64,
    unmet_goals_count: usize,
    planned_actions: Vec<TaskNode>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index_map: HashMap::new(),
        }
    }

    /// Solve state-space using A* search from current state to goal state.
    pub fn solve_planning_problem(
        current_state: HashMap<String, String>,
        goal_state: HashMap<String, String>,
        available_actions: &[TaskNode],
    ) -> Result<Self, String> {
        let initial_unmet = goal_state.iter().filter(|(gk, gv)| {
            current_state.get(*gk) != Some(*gv)
        }).count();

        let mut open_set = vec![PlanningState {
            current_beliefs: current_state.clone(),
            path_cost: 0.0,
            unmet_goals_count: initial_unmet,
            planned_actions: Vec::new(),
        }];

        let mut max_iterations = 1000;

        while let Some(best_idx) = open_set.iter().enumerate().min_by(|(_, a), (_, b)| {
            let score_a = a.path_cost + a.unmet_goals_count as f64;
            let score_b = b.path_cost + b.unmet_goals_count as f64;
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        }).map(|(i, _)| i) {
            if max_iterations == 0 {
                return Err("Planning iteration limit exceeded".to_string());
            }
            max_iterations -= 1;

            let current = open_set.remove(best_idx);

            // Check goal condition
            let mut is_goal = true;
            for (gk, gv) in &goal_state {
                if current.current_beliefs.get(gk) != Some(gv) {
                    is_goal = false;
                    break;
                }
            }

            if is_goal {
                let mut task_graph = TaskGraph::new();
                let mut prev_idx: Option<NodeIndex> = None;
                for task in current.planned_actions {
                    let idx = task_graph.graph.add_node(task.clone());
                    task_graph.node_index_map.insert(task.id, idx);
                    if let Some(p) = prev_idx {
                        task_graph.graph.add_edge(p, idx, ());
                    }
                    prev_idx = Some(idx);
                }
                return Ok(task_graph);
            }

            // Expand states
            for action in available_actions {
                let mut preconditions_met = true;
                for (pk, pv) in &action.preconditions {
                    if current.current_beliefs.get(pk) != Some(pv) {
                        preconditions_met = false;
                        break;
                    }
                }

                if preconditions_met {
                    let mut next_beliefs = current.current_beliefs.clone();
                    for (ek, ev) in &action.effects {
                        next_beliefs.insert(ek.clone(), ev.clone());
                    }

                    let mut unmet = 0;
                    for (gk, gv) in &goal_state {
                        if next_beliefs.get(gk) != Some(gv) {
                            unmet += 1;
                        }
                    }

                    let mut next_actions = current.planned_actions.clone();
                    next_actions.push(action.clone());

                    open_set.push(PlanningState {
                        current_beliefs: next_beliefs,
                        path_cost: current.path_cost + action.cost,
                        unmet_goals_count: unmet,
                        planned_actions: next_actions,
                    });
                }
            }
        }

        Err("No planning path found".to_string())
    }
}
