pub mod graph;
pub mod search;
pub mod contradiction;
pub mod deconstructor;

pub use graph::{CausalTopoGraph, TopoNode, TopoEdge, EdgeType};
pub use search::{ppr_search, ppr_weighted, multi_hop_paths, markov_neighbors, rank_map};
pub use contradiction::{would_contradict, analyze_proposed_edge, scan_pair_contradictions, ContradictionReport};
pub use deconstructor::{
    deconstruct, deconstruct_with_llm_hint, hyp_to_props, DeconstructedHypothesis, HypEdge,
};
