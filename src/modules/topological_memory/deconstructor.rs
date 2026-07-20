//! Lightweight hypothesis deconstructor — rule-based parse of free text into
//! candidate causal edges / nodes (no LLM required).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HypEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeconstructedHypothesis {
    pub nodes: Vec<String>,
    pub edges: Vec<HypEdge>,
    pub raw: String,
}

/// Parse free-text hypothesis into nodes + directed edges using cue patterns.
/// Patterns: "A causes B", "A → B", "A leads to B", "A because B" (B→A).
pub fn deconstruct(text: &str) -> DeconstructedHypothesis {
    let raw = text.trim().to_string();
    let lower = raw.to_lowercase();
    let mut edges = Vec::new();
    let mut nodes: Vec<String> = Vec::new();

    let patterns: &[(&str, bool)] = &[
        (" causes ", false),
        (" cause ", false),
        (" leads to ", false),
        (" results in ", false),
        (" implies ", false),
        (" -> ", false),
        (" → ", false),
        (" because ", true),  // reversed
        (" due to ", true),
    ];

    for (cue, reverse) in patterns {
        if let Some(idx) = lower.find(cue) {
            let left = raw[..idx].trim();
            let right = raw[idx + cue.len()..].trim();
            if left.is_empty() || right.is_empty() {
                continue;
            }
            let (from, to) = if *reverse {
                (right.to_string(), left.to_string())
            } else {
                (left.to_string(), right.to_string())
            };
            // strip trailing punctuation
            let from = from.trim_end_matches(['.', ',', ';', '!']).to_string();
            let to = to.trim_end_matches(['.', ',', ';', '!']).to_string();
            if !nodes.contains(&from) {
                nodes.push(from.clone());
            }
            if !nodes.contains(&to) {
                nodes.push(to.clone());
            }
            edges.push(HypEdge {
                from,
                to,
                relation: cue.trim().to_string(),
                confidence: 0.7,
            });
            break; // first cue wins
        }
    }

    if nodes.is_empty() && !raw.is_empty() {
        // Single node fallback
        nodes.push(raw.clone());
    }

    DeconstructedHypothesis { nodes, edges, raw }
}

/// Apply deconstructed edges onto a property bag (for topo node props).
pub fn hyp_to_props(hyp: &DeconstructedHypothesis) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("hyp_raw".into(), hyp.raw.clone());
    m.insert("hyp_nodes".into(), hyp.nodes.join("|"));
    m.insert(
        "hyp_edges".into(),
        hyp.edges
            .iter()
            .map(|e| format!("{}>{}", e.from, e.to))
            .collect::<Vec<_>>()
            .join(";"),
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_causes() {
        let h = deconstruct("rain causes floods");
        assert_eq!(h.edges.len(), 1);
        assert_eq!(h.edges[0].from, "rain");
        assert_eq!(h.edges[0].to, "floods");
    }

    #[test]
    fn parse_because_reversed() {
        let h = deconstruct("floods because rain");
        assert_eq!(h.edges.len(), 1);
        assert_eq!(h.edges[0].from, "rain");
        assert_eq!(h.edges[0].to, "floods");
    }

    #[test]
    fn arrow() {
        let h = deconstruct("X -> Y");
        assert_eq!(h.edges[0].from, "X");
        assert_eq!(h.edges[0].to, "Y");
    }
}
