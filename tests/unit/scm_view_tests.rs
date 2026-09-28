//! v3.18.0 loop-ready contract — canonical parent order and the SCM read model.
//!
//! `weights[i]` of a structural equation binds to the i-th parent in ascending id order.
//! Every graph below is built fresh, so each one gets its own `HashMap` seed: a binding that
//! followed map iteration order would differ between iterations of the same loop.

use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_store::MemoryStore;
use hipcortex::scm_view::{EquationView, InterventionView, ScmView};
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::tx_log::{TxKind, TxLog};
use hipcortex::world_model_enhanced::causal::{
    CausalGraph, DirectionalSE, InterventionValue, LinearSE, StructuralEquation,
};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::InMemoryBackend;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

const FRESH_GRAPHS: usize = 32;

/// `a → y ← b` with `y = 1·a + 10·b + u`, nodes and edges inserted in reverse id order.
fn two_parent_graph() -> CausalGraph {
    let mut g = CausalGraph::new();
    for id in ["y", "b", "a"] {
        g.add_node(id.to_string()).unwrap();
    }
    g.add_edge("b".to_string(), "y".to_string()).unwrap();
    g.add_edge("a".to_string(), "y".to_string()).unwrap();
    g.node_mut("y").unwrap().equation = Some(Arc::new(LinearSE { weights: vec![1.0, 10.0] }));
    g
}

#[test]
fn parents_come_back_in_canonical_order() {
    let expected = vec!["a".to_string(), "b".to_string()];
    for _ in 0..FRESH_GRAPHS {
        let g = two_parent_graph();
        assert_eq!(g.get_parents("y"), expected);
        assert_eq!(g.parents_of("y"), expected);
    }
}

#[test]
fn equation_weights_bind_to_parents_in_canonical_order() {
    // Observed a=2, b=3, y=32 abducts u_y = 0; do(a=4) gives y = 1·4 + 10·3 = 34.
    // Binding the weights the other way round abducts u_y = 9 and answers 52.
    let observed: HashMap<String, f64> = [("a", 2.0), ("b", 3.0), ("y", 32.0)]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    for _ in 0..FRESH_GRAPHS {
        let g = two_parent_graph();
        let cf = g
            .compute_scm_counterfactual_directional(&observed, "a", &InterventionValue::Scalar(4.0))
            .unwrap();
        assert!((cf["y"] - 34.0).abs() < 1e-9, "y_cf = {}, expected 34", cf["y"]);
    }
}

// ── ScmView: the SCM as data ─────────────────────────────────────────────────

/// `two_parent_graph` plus `b → z` (directional), an isolated noisy `c`, a scalar pin
/// on `a` and a directional pin on `b`.
fn pinned_graph() -> CausalGraph {
    let mut g = two_parent_graph();
    g.add_edge("b".to_string(), "z".to_string()).unwrap();
    g.node_mut("z").unwrap().equation =
        Some(Arc::new(DirectionalSE::new(vec![2.0], vec![0.5, -0.5])));
    g.add_node("c".to_string()).unwrap();
    let c = g.node_mut("c").unwrap();
    c.noise_var = 0.25;
    c.noise_var_vector = Some(vec![0.1, 0.2]);
    g.apply_intervention("a", 4.0);
    g.apply_directional_intervention("b", 3.0, &[0.6, 0.8]).unwrap();
    g
}

fn node<'v>(view: &'v ScmView, id: &str) -> &'v hipcortex::scm_view::ScmNodeView {
    view.nodes.iter().find(|n| n.id == id).unwrap()
}

#[test]
fn scm_view_json_is_identical_across_fresh_builds() {
    let first = serde_json::to_string(&ScmView::from_graph(&pinned_graph(), 7)).unwrap();
    for _ in 1..FRESH_GRAPHS {
        let again = serde_json::to_string(&ScmView::from_graph(&pinned_graph(), 7)).unwrap();
        assert_eq!(again, first);
    }
}

#[test]
fn scm_view_lists_nodes_and_edges_in_id_order() {
    let view = ScmView::from_graph(&pinned_graph(), 7);
    assert_eq!(view.as_of_tx, 7);
    let ids: Vec<&str> = view.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c", "y", "z"]);
    let edges: Vec<(&str, &str)> =
        view.edges.iter().map(|e| (e.from.as_str(), e.to.as_str())).collect();
    assert_eq!(edges, [("a", "y"), ("b", "y"), ("b", "z")]);
    assert!(view.edges.iter().all(|e| e.strength == 1.0));
}

#[test]
fn scm_view_parents_align_with_equation_weights() {
    let view = ScmView::from_graph(&pinned_graph(), 0);
    let y = node(&view, "y");
    assert_eq!(y.parents, ["a", "b"]);
    assert_eq!(
        y.equation,
        Some(EquationView {
            shape: "scalar".into(),
            dim: None,
            weights: Some(vec![1.0, 10.0]),
            dir_weights: None,
        })
    );
    let z = node(&view, "z");
    assert_eq!(z.parents, ["b"]);
    assert_eq!(
        z.equation,
        Some(EquationView {
            shape: "directional".into(),
            dim: Some(2),
            weights: Some(vec![2.0]),
            dir_weights: Some(vec![0.5, -0.5]),
        })
    );
}

#[test]
fn scm_view_carries_node_noise() {
    let view = ScmView::from_graph(&pinned_graph(), 0);
    let c = node(&view, "c");
    assert_eq!(c.equation, None);
    assert_eq!(c.noise_var, 0.25);
    assert_eq!(c.noise_var_vector, Some(vec![0.1, 0.2]));
}

#[test]
fn scm_view_lists_scalar_and_directional_pins() {
    let view = ScmView::from_graph(&pinned_graph(), 0);
    assert_eq!(
        view.interventions,
        vec![
            InterventionView { var: "a".into(), value: 4.0, direction: None },
            InterventionView { var: "b".into(), value: 3.0, direction: Some(vec![0.6, 0.8]) },
        ]
    );
}

#[test]
fn scm_view_reports_acyclicity() {
    // `add_edge` rejects cycles, so every graph the API can build is acyclic.
    let mut g = pinned_graph();
    assert!(g.add_edge("y".to_string(), "a".to_string()).is_err());
    assert!(g.is_acyclic());
    assert!(ScmView::from_graph(&g, 0).acyclic);
}

/// A v3.17-shaped equation: only the two required methods.
#[derive(Debug)]
struct Doubling;

impl StructuralEquation for Doubling {
    fn evaluate(&self, parents: &[f64], u: f64) -> f64 {
        2.0 * parents.iter().sum::<f64>() + u
    }
    fn invert_for_u(&self, parents: &[f64], observed: f64) -> f64 {
        observed - 2.0 * parents.iter().sum::<f64>()
    }
}

#[test]
fn equation_without_weight_accessors_shows_null_weights() {
    let mut g = two_parent_graph();
    g.node_mut("y").unwrap().equation = Some(Arc::new(Doubling));
    let view = ScmView::from_graph(&g, 0);
    let y = node(&view, "y");
    assert_eq!(
        y.equation,
        Some(EquationView { shape: "scalar".into(), dim: None, weights: None, dir_weights: None })
    );
    let json = serde_json::to_value(y).unwrap();
    assert!(json["equation"]["weights"].is_null());
}

#[test]
fn world_model_view_reflects_its_causal_graph() {
    let wm = WorldModelEnhanced::new();
    wm.add_causal_edge("a".to_string(), "y".to_string()).unwrap();
    wm.rewrite_structural_equation("y", vec![3.0]).unwrap();
    wm.apply_intervention("a", 1.5).unwrap();
    let view = wm.scm_view(11).unwrap();
    assert_eq!(view.as_of_tx, 11);
    assert_eq!(node(&view, "y").equation.as_ref().unwrap().weights, Some(vec![3.0]));
    assert_eq!(view.interventions, vec![InterventionView { var: "a".into(), value: 1.5, direction: None }]);
}

fn handle_over(
    world: Arc<RwLock<WorldModelEnhanced>>,
    tx_log: Option<Arc<TxLog>>,
) -> CognitiveHandle<InMemoryBackend> {
    CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        world,
        Arc::new(SelfModel::new()),
        tx_log,
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    )
}

#[test]
fn handle_view_is_stamped_with_the_tx_log_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let log = Arc::new(TxLog::open(dir.path().join("tx.jsonl")).unwrap());
    log.append(TxKind::MemoryAdd, vec![], "agent");
    log.append(TxKind::MemoryAdd, vec![], "agent");
    let world = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    world.read().unwrap().add_causal_edge("a".to_string(), "y".to_string()).unwrap();

    let view = handle_over(world, Some(log)).scm_view().unwrap();
    assert_eq!(view.as_of_tx, 2);
    assert_eq!(node(&view, "y").parents, ["a"]);
}

#[test]
fn handle_view_without_a_tx_log_is_as_of_zero() {
    let world = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let view = handle_over(world, None).scm_view().unwrap();
    assert_eq!(view.as_of_tx, 0);
    assert!(view.nodes.is_empty());
}
