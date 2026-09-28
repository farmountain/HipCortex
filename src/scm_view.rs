//! ScmView — the causal SCM as a serialisable read model (v3.18.0).
//!
//! Chain-of-thought:
//! 1. A consumer that plans against the world model needs the SCM as data: the nodes,
//!    each node's parents, its equation's weights, and the interventions pinned now.
//! 2. `CausalGraph` keeps all of this in `HashMap`s whose iteration order changes from
//!    process to process, so the view sorts every list: identical graphs give
//!    byte-identical JSON.
//! 3. `parents` come from `CausalGraph::get_parents` (ascending id), the order that
//!    `weights[i]` binds to, so an equation can be evaluated from the view alone.
//! 4. Weights are read through the defaulted `StructuralEquation::weights` /
//!    `dir_weights` accessors; an equation that does not expose them shows `null`.

use crate::world_model_enhanced::causal::{CausalGraph, SeShape, StructuralEquation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmView {
    /// TxLog cursor read before the graph (0 without a TxLog): the view reflects at
    /// least every transaction up to here.
    pub as_of_tx: u64,
    pub acyclic: bool,
    /// Ascending by `id`.
    pub nodes: Vec<ScmNodeView>,
    /// Ascending by `(from, to)`.
    pub edges: Vec<ScmEdgeView>,
    /// Ascending by `var`.
    pub interventions: Vec<InterventionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmNodeView {
    pub id: String,
    /// Canonical (ascending) order: `equation.weights[i]` binds to `parents[i]`.
    pub parents: Vec<String>,
    pub equation: Option<EquationView>,
    pub noise_var: f64,
    pub noise_var_vector: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquationView {
    /// "scalar" | "directional"
    pub shape: String,
    /// Direction dimension of a directional equation.
    pub dim: Option<usize>,
    /// `None` when the equation does not expose its weights.
    pub weights: Option<Vec<f64>>,
    pub dir_weights: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmEdgeView {
    pub from: String,
    pub to: String,
    pub strength: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionView {
    pub var: String,
    /// The pinned value; the norm of a directional intervention.
    pub value: f64,
    /// Unit direction of a directional intervention.
    pub direction: Option<Vec<f64>>,
}

impl EquationView {
    fn of(eq: &dyn StructuralEquation) -> Self {
        let (shape, dim) = match eq.shape() {
            SeShape::Scalar => ("scalar", None),
            SeShape::Directional { dim } => ("directional", Some(dim)),
        };
        Self {
            shape: shape.to_string(),
            dim,
            weights: eq.weights().map(<[f64]>::to_vec),
            dir_weights: eq.dir_weights().map(<[f64]>::to_vec),
        }
    }
}

impl ScmView {
    pub fn from_graph(graph: &CausalGraph, as_of_tx: u64) -> Self {
        let mut ids = graph.node_ids();
        ids.sort();
        let nodes = ids
            .iter()
            .filter_map(|id| graph.node(id))
            .map(|n| ScmNodeView {
                id: n.id.clone(),
                parents: graph.get_parents(&n.id),
                equation: n.equation.as_deref().map(EquationView::of),
                noise_var: n.noise_var,
                noise_var_vector: n.noise_var_vector.clone(),
            })
            .collect();

        let mut edges: Vec<ScmEdgeView> = graph
            .all_edges()
            .into_iter()
            .map(|e| ScmEdgeView { from: e.from, to: e.to, strength: e.strength })
            .collect();
        edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));

        let interventions = graph
            .pinned_vars()
            .into_iter()
            .filter_map(|var| {
                let pin = graph.pinned_intervention_value(&var)?;
                Some(InterventionView {
                    value: pin.norm(),
                    direction: pin.direction().map(<[f64]>::to_vec),
                    var,
                })
            })
            .collect();

        Self { as_of_tx, acyclic: graph.is_acyclic(), nodes, edges, interventions }
    }
}
