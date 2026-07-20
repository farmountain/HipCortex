//! Contradiction detection over CausalTopoGraph (cycle / reverse causal).

use super::graph::{CausalTopoGraph, EdgeType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContradictionReport {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub reason: String,
    pub reverse_paths: Vec<Vec<String>>,
}

/// Would adding from→to with edge type create a contradiction?
pub fn would_contradict(
    graph: &CausalTopoGraph,
    from: &str,
    to: &str,
    et: EdgeType,
) -> bool {
    graph.detect_contradiction(from.to_string(), to.to_string(), et)
}

/// Detailed contradiction report for a proposed edge.
pub fn analyze_proposed_edge(
    graph: &CausalTopoGraph,
    from: &str,
    to: &str,
    et: EdgeType,
) -> Option<ContradictionReport> {
    if !graph.detect_contradiction(from.to_string(), to.to_string(), et.clone()) {
        return None;
    }
    let reverse_paths = graph.find_multi_hop_paths(to, from, 4);
    let reason = if from == to {
        "self-loop".to_string()
    } else if !reverse_paths.is_empty() {
        format!("cycle: path exists from {} back to {}", to, from)
    } else {
        "high-strength reverse causal edge".to_string()
    };
    Some(ContradictionReport {
        from: from.to_string(),
        to: to.to_string(),
        edge_type: format!("{:?}", et),
        reason,
        reverse_paths,
    })
}

/// Scan all directed pairs among nodes for mutual causal contradictions (O(n²) sample).
pub fn scan_pair_contradictions(graph: &CausalTopoGraph, limit: usize) -> Vec<ContradictionReport> {
    let ranks = graph.personalized_pagerank(&[], 0.85, 2);
    let mut ids: Vec<String> = ranks.keys().cloned().collect();
    ids.sort();
    let mut out = Vec::new();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            if out.len() >= limit {
                return out;
            }
            let a = &ids[i];
            let b = &ids[j];
            if let Some(r) = analyze_proposed_edge(graph, a, b, EdgeType::Causal) {
                out.push(r);
            }
            if out.len() >= limit {
                return out;
            }
            if let Some(r) = analyze_proposed_edge(graph, b, a, EdgeType::Causal) {
                out.push(r);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn detects_cycle_edge() {
        let mut g = CausalTopoGraph::new();
        g.add_node("x".into(), [0.0; 128], HashMap::new()).unwrap();
        g.add_node("y".into(), [0.0; 128], HashMap::new()).unwrap();
        g.add_edge("x".into(), "y".into(), EdgeType::Causal, 0.9, 0.9)
            .unwrap();
        assert!(would_contradict(&g, "y", "x", EdgeType::Causal));
        let rep = analyze_proposed_edge(&g, "y", "x", EdgeType::Causal).unwrap();
        assert!(rep.reason.contains("cycle") || !rep.reverse_paths.is_empty());
    }
}
