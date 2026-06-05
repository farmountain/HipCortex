// Broker Policy — Swappable working-set materialization strategy
//
// The Working Set Broker assembles bounded payloads for client context windows.
// What counts as "the minimal working set for task τ" differs radically across
// consumers: a coding agent wants the relevant call graph; a world-model planner
// wants the relevant state-space neighborhood; an ASI-class reasoner might want
// something we can't specify today.
//
// Making the broker's selection policy a swappable trait keeps HipCortex ONE
// engine across all consumers. The universality lives in the invariant interface
// (materialize bounded working set), and the variability is quarantined in
// policy implementations — including WASM plugins via the PluginHost.

use std::collections::HashMap;
use uuid::Uuid;

use crate::symbolic_store::{SymbolicEdge, SymbolicNode};
use crate::temporal_indexer::TemporalTrace;

/// A bounded payload assembled for a client's context window.
///
/// Contains the minimal subgraph, temporal slice, FSM position, and
/// any relevant metadata needed to resume or continue a task — all
/// within a specified token budget.
#[derive(Debug, Clone)]
pub struct BoundedPayload {
    /// The task this payload is for
    pub task_id: Uuid,
    /// The session requesting the payload
    pub session_id: Uuid,
    /// Relevant symbolic nodes (subgraph)
    pub nodes: Vec<SymbolicNode>,
    /// Relevant symbolic edges (subgraph connections)
    pub edges: Vec<SymbolicEdge>,
    /// Recent temporal traces
    pub temporal_traces: Vec<TemporalTraceSummary>,
    /// Current FSM state (if applicable)
    pub fsm_state: Option<String>,
    /// Next intended step (from checkpoint, if any)
    pub next_step: Option<String>,
    /// Total estimated token count of this payload
    pub estimated_tokens: usize,
    /// Token budget this payload was assembled under
    pub budget: usize,
    /// Policy that produced this payload
    pub policy_name: String,
    /// Additional policy-specific data
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A lightweight summary of a temporal trace for context inclusion.
#[derive(Debug, Clone)]
pub struct TemporalTraceSummary {
    pub id: Uuid,
    pub timestamp: String,
    pub relevance: f32,
    pub summary: String,
}

/// A description of a task for broker policy selection.
#[derive(Debug, Clone)]
pub struct TaskDescription {
    /// Unique task identifier
    pub task_id: Uuid,
    /// Human-readable task description
    pub description: String,
    /// Task tags/categories (e.g., "coding", "world_model", "debugging")
    pub tags: Vec<String>,
    /// Priority (0-10)
    pub priority: u8,
    /// Any task-specific context
    pub context: HashMap<String, String>,
}

/// The broker policy trait — determines what to include in a bounded working set.
///
/// Implementations answer: "Given task τ and budget b, what is the minimal
/// payload from the store that the client needs in its context window?"
pub trait BrokerPolicy {
    /// Get the policy name (for logging and payload identification).
    fn name(&self) -> &str;

    /// Select the working set for a task under a token budget.
    ///
    /// Returns a BoundedPayload whose estimated_tokens ≤ budget.
    fn select_working_set(
        &self,
        task: &TaskDescription,
        budget: usize,
        nodes: &[SymbolicNode],
        edges: &[SymbolicEdge],
        temporal_traces: &[TemporalTraceSummary],
        fsm_state: Option<&str>,
        next_step: Option<&str>,
    ) -> BoundedPayload;
}

// ============================================================================
// Built-in Policy Implementations
// ============================================================================

/// Default policy: selects the most-relevant nodes and most-recent traces.
///
/// Strategy:
/// 1. Include all nodes whose label matches task tags (up to budget)
/// 2. Include most recent temporal traces (up to budget)
/// 3. Include FSM state and next step unconditionally (fixed cost)
pub struct DefaultBrokerPolicy;

impl BrokerPolicy for DefaultBrokerPolicy {
    fn name(&self) -> &str {
        "default"
    }

    fn select_working_set(
        &self,
        task: &TaskDescription,
        budget: usize,
        nodes: &[SymbolicNode],
        edges: &[SymbolicEdge],
        temporal_traces: &[TemporalTraceSummary],
        fsm_state: Option<&str>,
        next_step: Option<&str>,
    ) -> BoundedPayload {
        const TOKENS_PER_NODE: usize = 50;
        const TOKENS_PER_EDGE: usize = 30;
        const TOKENS_PER_TRACE: usize = 40;
        const FIXED_COST: usize = 100; // task metadata, FSM state, next step

        let mut estimated = FIXED_COST;
        let mut selected_nodes = Vec::new();
        let mut selected_edges = Vec::new();
        let mut selected_traces = Vec::new();

        // Prioritize nodes matching task tags
        for node in nodes {
            if estimated + TOKENS_PER_NODE > budget {
                break;
            }
            let matches_tag = task
                .tags
                .iter()
                .any(|tag| node.label.to_lowercase().contains(&tag.to_lowercase()));
            if matches_tag || selected_nodes.is_empty() {
                selected_nodes.push(node.clone());
                estimated += TOKENS_PER_NODE;
            }
        }

        // Add edges connecting selected nodes
        let selected_ids: Vec<Uuid> = selected_nodes.iter().map(|n| n.id).collect();
        for edge in edges {
            if estimated + TOKENS_PER_EDGE > budget {
                break;
            }
            if selected_ids.contains(&edge.from) || selected_ids.contains(&edge.to) {
                selected_edges.push(edge.clone());
                estimated += TOKENS_PER_EDGE;
            }
        }

        // Add most recent temporal traces
        for trace in temporal_traces.iter().rev() {
            if estimated + TOKENS_PER_TRACE > budget {
                break;
            }
            selected_traces.push(trace.clone());
            estimated += TOKENS_PER_TRACE;
        }

        BoundedPayload {
            task_id: task.task_id,
            session_id: Uuid::nil(), // Set by caller
            nodes: selected_nodes,
            edges: selected_edges,
            temporal_traces: selected_traces,
            fsm_state: fsm_state.map(|s| s.to_string()),
            next_step: next_step.map(|s| s.to_string()),
            estimated_tokens: estimated,
            budget,
            policy_name: self.name().to_string(),
            metadata: HashMap::new(),
        }
    }
}

/// Coding-agent policy: prioritizes call-graph relevant nodes and edges.
///
/// Strategy:
/// 1. Include nodes with labels matching "function", "struct", "trait", "impl"
/// 2. Include all edges (call relationships) for those nodes
/// 3. Include recent temporal traces tagged "coding" or "debug"
pub struct CodingBrokerPolicy;

impl BrokerPolicy for CodingBrokerPolicy {
    fn name(&self) -> &str {
        "coding"
    }

    fn select_working_set(
        &self,
        task: &TaskDescription,
        budget: usize,
        nodes: &[SymbolicNode],
        edges: &[SymbolicEdge],
        temporal_traces: &[TemporalTraceSummary],
        fsm_state: Option<&str>,
        next_step: Option<&str>,
    ) -> BoundedPayload {
        const TOKENS_PER_NODE: usize = 60;
        const TOKENS_PER_EDGE: usize = 35;
        const TOKENS_PER_TRACE: usize = 40;
        const FIXED_COST: usize = 100;

        let coding_labels = [
            "function", "struct", "trait", "impl", "method", "module", "enum",
        ];
        let mut estimated = FIXED_COST;
        let mut selected_nodes = Vec::new();
        let mut selected_edges = Vec::new();
        let mut selected_traces = Vec::new();

        for node in nodes {
            if estimated + TOKENS_PER_NODE > budget {
                break;
            }
            let is_coding = coding_labels
                .iter()
                .any(|label| node.label.to_lowercase().contains(label));
            if is_coding {
                selected_nodes.push(node.clone());
                estimated += TOKENS_PER_NODE;
            }
        }

        // Include all edges connecting selected nodes
        let selected_ids: Vec<Uuid> = selected_nodes.iter().map(|n| n.id).collect();
        for edge in edges {
            if estimated + TOKENS_PER_EDGE > budget {
                break;
            }
            if selected_ids.contains(&edge.from) && selected_ids.contains(&edge.to) {
                selected_edges.push(edge.clone());
                estimated += TOKENS_PER_EDGE;
            }
        }

        // Prioritize coding-related traces
        for trace in temporal_traces.iter().rev() {
            if estimated + TOKENS_PER_TRACE > budget {
                break;
            }
            selected_traces.push(trace.clone());
            estimated += TOKENS_PER_TRACE;
        }

        BoundedPayload {
            task_id: task.task_id,
            session_id: Uuid::nil(),
            nodes: selected_nodes,
            edges: selected_edges,
            temporal_traces: selected_traces,
            fsm_state: fsm_state.map(|s| s.to_string()),
            next_step: next_step.map(|s| s.to_string()),
            estimated_tokens: estimated,
            budget,
            policy_name: self.name().to_string(),
            metadata: HashMap::new(),
        }
    }
}

/// World-model planner policy: prioritizes state-space neighborhood.
///
/// Strategy:
/// 1. Include nodes in the causal neighborhood of entities mentioned in the task
/// 2. Include causal edges (not just any edges)
/// 3. Include temporal traces related to state transitions
pub struct WorldModelBrokerPolicy;

impl BrokerPolicy for WorldModelBrokerPolicy {
    fn name(&self) -> &str {
        "world_model"
    }

    fn select_working_set(
        &self,
        task: &TaskDescription,
        budget: usize,
        nodes: &[SymbolicNode],
        edges: &[SymbolicEdge],
        temporal_traces: &[TemporalTraceSummary],
        fsm_state: Option<&str>,
        next_step: Option<&str>,
    ) -> BoundedPayload {
        const TOKENS_PER_NODE: usize = 50;
        const TOKENS_PER_EDGE: usize = 30;
        const TOKENS_PER_TRACE: usize = 40;
        const FIXED_COST: usize = 100;

        let mut estimated = FIXED_COST;
        let mut selected_nodes = Vec::new();
        let mut selected_edges = Vec::new();
        let mut selected_traces = Vec::new();

        // Select nodes that mention entities from the task
        for node in nodes {
            if estimated + TOKENS_PER_NODE > budget {
                break;
            }
            let is_relevant = task
                .context
                .values()
                .any(|v| node.label.to_lowercase().contains(&v.to_lowercase()));
            if is_relevant || selected_nodes.is_empty() {
                selected_nodes.push(node.clone());
                estimated += TOKENS_PER_NODE;
            }
        }

        // Include edges with causal relations
        for edge in edges {
            if estimated + TOKENS_PER_EDGE > budget {
                break;
            }
            if edge.relation.to_lowercase().contains("causal")
                || edge.relation.to_lowercase().contains("cause")
            {
                selected_edges.push(edge.clone());
                estimated += TOKENS_PER_EDGE;
            }
        }

        // Include recent state-transition traces
        for trace in temporal_traces.iter().rev() {
            if estimated + TOKENS_PER_TRACE > budget {
                break;
            }
            if trace.summary.to_lowercase().contains("state")
                || trace.summary.to_lowercase().contains("transition")
            {
                selected_traces.push(trace.clone());
                estimated += TOKENS_PER_TRACE;
            }
        }

        BoundedPayload {
            task_id: task.task_id,
            session_id: Uuid::nil(),
            nodes: selected_nodes,
            edges: selected_edges,
            temporal_traces: selected_traces,
            fsm_state: fsm_state.map(|s| s.to_string()),
            next_step: next_step.map(|s| s.to_string()),
            estimated_tokens: estimated,
            budget,
            policy_name: self.name().to_string(),
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_nodes() -> Vec<SymbolicNode> {
        vec![
            SymbolicNode {
                id: Uuid::new_v4(),
                label: "function_process_request".to_string(),
                properties: HashMap::new(),
            },
            SymbolicNode {
                id: Uuid::new_v4(),
                label: "struct_RequestHandler".to_string(),
                properties: HashMap::new(),
            },
            SymbolicNode {
                id: Uuid::new_v4(),
                label: "entity_RobotArm".to_string(),
                properties: HashMap::new(),
            },
        ]
    }

    fn make_test_edges(nodes: &[SymbolicNode]) -> Vec<SymbolicEdge> {
        vec![
            SymbolicEdge {
                from: nodes[0].id,
                to: nodes[1].id,
                relation: "calls".to_string(),
            },
            SymbolicEdge {
                from: nodes[2].id,
                to: nodes[2].id,
                relation: "causal_influences".to_string(),
            },
        ]
    }

    #[test]
    fn test_default_policy_respects_budget() {
        let policy = DefaultBrokerPolicy;
        let task = TaskDescription {
            task_id: Uuid::new_v4(),
            description: "Test".to_string(),
            tags: vec!["function".to_string()],
            priority: 5,
            context: HashMap::new(),
        };
        let nodes = make_test_nodes();
        let edges = make_test_edges(&nodes);
        let traces = vec![];
        let budget = 200;

        let payload = policy.select_working_set(&task, budget, &nodes, &edges, &traces, None, None);

        assert!(
            payload.estimated_tokens <= budget,
            "Payload tokens {} should not exceed budget {}",
            payload.estimated_tokens,
            budget
        );
        assert_eq!(payload.policy_name, "default");
    }

    #[test]
    fn test_coding_policy_selects_code_symbols() {
        let policy = CodingBrokerPolicy;
        let task = TaskDescription {
            task_id: Uuid::new_v4(),
            description: "Debug".to_string(),
            tags: vec!["debug".to_string()],
            priority: 8,
            context: HashMap::new(),
        };
        let nodes = make_test_nodes();
        let edges = make_test_edges(&nodes);
        let traces = vec![];
        let budget = 500;

        let payload = policy.select_working_set(&task, budget, &nodes, &edges, &traces, None, None);

        // Should select function_ and struct_ nodes
        let has_function = payload.nodes.iter().any(|n| n.label.contains("function"));
        let has_struct = payload.nodes.iter().any(|n| n.label.contains("struct"));
        assert!(
            has_function || has_struct,
            "Coding policy should select code-relevant nodes"
        );
    }

    #[test]
    fn test_world_model_policy_respects_budget() {
        let policy = WorldModelBrokerPolicy;
        let task = TaskDescription {
            task_id: Uuid::new_v4(),
            description: "Predict".to_string(),
            tags: vec!["world_model".to_string()],
            priority: 6,
            context: HashMap::new(),
        };
        let nodes = make_test_nodes();
        let edges = make_test_edges(&nodes);
        let traces = vec![];
        let budget = 200;

        let payload = policy.select_working_set(&task, budget, &nodes, &edges, &traces, None, None);

        assert!(payload.estimated_tokens <= budget);
        assert_eq!(payload.policy_name, "world_model");
    }

    #[test]
    fn test_empty_input_produces_minimal_payload() {
        let policy = DefaultBrokerPolicy;
        let task = TaskDescription {
            task_id: Uuid::new_v4(),
            description: "Empty".to_string(),
            tags: vec![],
            priority: 1,
            context: HashMap::new(),
        };

        let payload = policy.select_working_set(&task, 1000, &[], &[], &[], None, None);

        assert_eq!(payload.nodes.len(), 0);
        assert_eq!(payload.edges.len(), 0);
        assert!(payload.estimated_tokens <= 1000);
    }

    #[test]
    fn test_fsm_state_included_in_payload() {
        let policy = DefaultBrokerPolicy;
        let task = TaskDescription {
            task_id: Uuid::new_v4(),
            description: "Resume".to_string(),
            tags: vec![],
            priority: 5,
            context: HashMap::new(),
        };

        let payload =
            policy.select_working_set(&task, 500, &[], &[], &[], Some("Reason"), Some("Apply fix"));

        assert_eq!(payload.fsm_state, Some("Reason".to_string()));
        assert_eq!(payload.next_step, Some("Apply fix".to_string()));
    }
}
