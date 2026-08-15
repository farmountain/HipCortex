use crate::coherence::CoherenceChecker;
use crate::self_model::SelfModel;
use crate::task_graph::{TaskGraph, TaskState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub task_id: Uuid,
    pub local_beliefs: HashMap<String, String>,
    pub current_step: Option<Uuid>,
}

pub struct ExecutiveScheduler {
    pub goal_stack: Vec<StackFrame>,
    pub active_graph: Option<TaskGraph>,
    pub max_depth: usize,
}

impl ExecutiveScheduler {
    pub fn new(max_depth: usize) -> Self {
        Self {
            goal_stack: Vec::new(),
            active_graph: None,
            max_depth,
        }
    }

    /// Single execution tick of the scheduler kernel
    pub fn tick(
        &mut self,
        self_model: &SelfModel,
        coherence: &CoherenceChecker,
    ) -> Result<(), String> {
        // Step 1: Health & Invariant pre-check
        if !self_model.is_healthy().unwrap_or(false) {
            // Push pre-emptive Diagnostics Goal Frame
            self.goal_stack.push(StackFrame {
                task_id: Uuid::new_v4(),
                local_beliefs: {
                    let mut b = HashMap::new();
                    b.insert(
                        "diagnostic_reason".to_string(),
                        "self_model_unhealthy".to_string(),
                    );
                    b
                },
                current_step: None,
            });
            return Err("System unhealthy: pushing Diagnostic interrupt frame".to_string());
        }

        // Step 2: Coherence Write gating check
        if let Err(e) = coherence.gate_write("executive_scheduler::tick") {
            self.goal_stack.push(StackFrame {
                task_id: Uuid::new_v4(),
                local_beliefs: {
                    let mut b = HashMap::new();
                    b.insert(
                        "diagnostic_reason".to_string(),
                        format!("coherence_gate_failed: {}", e.reason),
                    );
                    b
                },
                current_step: None,
            });
            return Err(format!(
                "Coherence gate rejected write operations: {}",
                e.reason
            ));
        }

        // Step 3: Execute active task step
        if let Some(frame) = self.goal_stack.last_mut() {
            if let Some(ref mut task_graph) = self.active_graph {
                if let Some(node_idx) = frame
                    .current_step
                    .and_then(|id| task_graph.node_index_map.get(&id))
                {
                    let task = &mut task_graph.graph[*node_idx];
                    task.state = TaskState::Active;

                    // Execute step
                    task.state = TaskState::Completed;

                    // Topologically retrieve next step
                    let mut outgoing = task_graph.graph.neighbors(*node_idx);
                    if let Some(next_idx) = outgoing.next() {
                        frame.current_step = Some(task_graph.graph[next_idx].id);
                    } else {
                        // Pop completed goal frame
                        self.goal_stack.pop();
                    }
                }
            }
        }
        Ok(())
    }
}
