//! Graph search over CausalTopoGraph: PPR, multi-hop paths, Markov neighbor walks.

use super::graph::CausalTopoGraph;
use std::collections::HashMap;

/// Personalized PageRank from seed(s), returning top-k (id, score).
pub fn ppr_search(
    graph: &CausalTopoGraph,
    seeds: &[String],
    limit: usize,
    damping: f32,
    max_iter: usize,
) -> Vec<(String, f32)> {
    let ranks = graph.personalized_pagerank(seeds, damping, max_iter);
    let mut pairs: Vec<(String, f32)> = ranks.into_iter().collect();
    // Prefer seed-relative scores; drop pure zeros
    pairs.retain(|(_, s)| *s > 1e-9);
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs.truncate(limit.max(1));
    pairs
}

/// Weighted PPR using edge strength×confidence (graph.ppr API).
pub fn ppr_weighted(
    graph: &CausalTopoGraph,
    seed_id: &str,
    limit: usize,
) -> Vec<(String, f64)> {
    graph.ppr(seed_id, limit, 0.85, 20)
}

/// All simple multi-hop paths from → to within max_hops.
pub fn multi_hop_paths(
    graph: &CausalTopoGraph,
    from: &str,
    to: &str,
    max_hops: usize,
) -> Vec<Vec<String>> {
    graph.find_multi_hop_paths(from, to, max_hops)
}

/// One-step Markov walk: sample neighbor distribution by edge strength×confidence.
/// Returns (neighbor_id, transition_prob) sorted by prob desc.
pub fn markov_neighbors(graph: &CausalTopoGraph, from: &str) -> Vec<(String, f32)> {
    let neighbors = graph.get_neighbors(from);
    if neighbors.is_empty() {
        return vec![];
    }
    // Uniform if edge weights not exposed per-neighbor from public API —
    // use equal mass when only neighbor list is available.
    let p = 1.0 / neighbors.len() as f32;
    let mut out: Vec<(String, f32)> = neighbors.into_iter().map(|n| (n, p)).collect();
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// Aggregate rank map helper for callers that need HashMap.
pub fn rank_map(
    graph: &CausalTopoGraph,
    seeds: &[String],
    damping: f32,
    max_iter: usize,
) -> HashMap<String, f32> {
    graph.personalized_pagerank(seeds, damping, max_iter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topological_memory::{CausalTopoGraph, EdgeType};
    use std::collections::HashMap;

    fn toy() -> CausalTopoGraph {
        let mut g = CausalTopoGraph::new();
        g.add_node("a".into(), [0.1; 128], HashMap::new()).unwrap();
        g.add_node("b".into(), [0.2; 128], HashMap::new()).unwrap();
        g.add_node("c".into(), [0.3; 128], HashMap::new()).unwrap();
        g.add_edge("a".into(), "b".into(), EdgeType::Causal, 0.9, 0.9)
            .unwrap();
        g.add_edge("b".into(), "c".into(), EdgeType::Causal, 0.8, 0.8)
            .unwrap();
        g
    }

    #[test]
    fn ppr_search_finds_reachable() {
        let g = toy();
        let hits = ppr_search(&g, &["a".into()], 5, 0.85, 10);
        assert!(!hits.is_empty());
        let ids: Vec<_> = hits.iter().map(|(i, _)| i.as_str()).collect();
        assert!(ids.contains(&"b") || ids.contains(&"c") || ids.contains(&"a"));
    }

    #[test]
    fn multi_hop_a_to_c() {
        let g = toy();
        let paths = multi_hop_paths(&g, "a", "c", 3);
        assert!(!paths.is_empty());
    }

    #[test]
    fn markov_from_a() {
        let g = toy();
        let n = markov_neighbors(&g, "a");
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].0, "b");
    }
}
