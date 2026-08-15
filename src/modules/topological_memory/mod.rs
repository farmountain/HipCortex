pub mod contradiction;
pub mod deconstructor;
pub mod graph;
pub mod search;

pub use contradiction::{
    analyze_proposed_edge, scan_pair_contradictions, would_contradict, ContradictionReport,
};
pub use deconstructor::{
    deconstruct, deconstruct_with_llm_hint, hyp_to_props, DeconstructedHypothesis, HypEdge,
};
pub use graph::{CausalTopoGraph, EdgeType, TopoEdge, TopoNode};
pub use search::{markov_neighbors, multi_hop_paths, ppr_search, ppr_weighted, rank_map};
