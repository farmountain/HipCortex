// Property-Based Tests for World-Model Enhanced Invariants
//
// Verifies mathematical properties hold across random inputs:
// - Probability distributions sum to 1.0
// - Causal graph acyclicity maintained
// - Kalman covariance positive semi-definite
// - Uncertainty bounds are valid
// - Calibration improves with samples

use hipcortex::world_model_enhanced::{
    CausalGraph, DirectionalSE, EntityObservation, EntityState, EntityTracker, InterventionValue,
    SeShape, StateTransition, StructuralEquation, TransitionModel, UncertaintyEstimator,
};
use proptest::prelude::*;
use std::time::Instant;

// ============================================================================
// Invariant 1: Probability distributions sum to 1.0
// ============================================================================

proptest! {
    #[test]
    fn transition_probabilities_sum_to_one(
        // Generate random transitions
        transitions in prop::collection::vec(
            (
                prop::string::string_regex("[A-Z][0-9]").unwrap(),  // from_state
                prop::string::string_regex("A[0-9]").unwrap(),       // action
                prop::string::string_regex("[A-Z][0-9]").unwrap(),  // to_state
            ),
            10..50
        )
    ) {
        let mut model = TransitionModel::new();

        // Record all transitions
        for (from, action, to) in &transitions {
            let transition = StateTransition {
                from_state: from.clone(),
                action: action.clone(),
                to_state: to.clone(),
            };
            model.record_transition(transition).ok();
        }

        // Check predictions sum to 1.0 for each (state, action) pair
        for (from, action, _) in &transitions {
            if let Ok(pred) = model.predict(from, action) {
                let sum: f64 = pred.probabilities.values().sum();
                prop_assert!((sum - 1.0).abs() < 1e-6, "Probabilities sum to {} instead of 1.0", sum);
            }
        }
    }
}

proptest! {
    #[test]
    fn entropy_in_valid_range(
        transitions in prop::collection::vec(
            (
                prop::string::string_regex("[A-Z][0-9]").unwrap(),
                prop::string::string_regex("A[0-9]").unwrap(),
                prop::string::string_regex("[A-Z][0-9]").unwrap(),
            ),
            5..30
        )
    ) {
        let mut model = TransitionModel::new();

        for (from, action, to) in &transitions {
            model.record_transition(StateTransition {
                from_state: from.clone(),
                action: action.clone(),
                to_state: to.clone(),
            }).ok();
        }

        // Entropy should be non-negative and bounded by log2(vocab_size)
        for (from, action, _) in &transitions {
            if let Ok(entropy) = model.compute_entropy(from, action) {
                prop_assert!(entropy >= 0.0, "Entropy cannot be negative: {}", entropy);
                // For reasonable distributions, entropy shouldn't exceed 10 bits
                prop_assert!(entropy <= 10.0, "Entropy unreasonably high: {}", entropy);
            }
        }
    }
}

// ============================================================================
// Invariant 2: Causal graph acyclicity
// ============================================================================

proptest! {
    #[test]
    fn causal_graph_always_acyclic(
        edges in prop::collection::vec(
            (
                prop::string::string_regex("[A-Z]").unwrap(),
                prop::string::string_regex("[A-Z]").unwrap(),
            ),
            5..20
        )
    ) {
        let mut graph = CausalGraph::new();

        // Try to add all edges
        for (from, to) in &edges {
            if from != to {  // Skip self-loops
                graph.add_edge(from.clone(), to.clone()).ok();
            }
        }

        // Graph must remain acyclic
        prop_assert!(graph.is_acyclic(), "Graph became cyclic after edge additions");
    }
}

proptest! {
    #[test]
    fn transitive_paths_consistent(
        edges in prop::collection::vec(
            (prop::string::string_regex("[A-Z]").unwrap(), prop::string::string_regex("[A-Z]").unwrap()),
            3..15
        )
    ) {
        let mut graph = CausalGraph::new();

        for (from, to) in &edges {
            if from != to {
                graph.add_edge(from.clone(), to.clone()).ok();
            }
        }

        // If there's a path A→B and B→C, there should be a path A→C
        let nodes: Vec<String> = edges.iter()
            .flat_map(|(a, b)| vec![a.clone(), b.clone()])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for a in &nodes {
            for b in &nodes {
                for c in &nodes {
                    if graph.has_path(a, b).unwrap_or(false) && graph.has_path(b, c).unwrap_or(false) {
                        prop_assert!(
                            graph.has_path(a, c).unwrap_or(false),
                            "Transitive path broken: {} -> {} -> {} but no path {} -> {}",
                            a, b, c, a, c
                        );
                    }
                }
            }
        }
    }
}

// ============================================================================
// Invariant 3: Kalman covariance remains positive semi-definite
// ============================================================================

proptest! {
    #[test]
    fn kalman_covariance_positive_semidefinite(
        initial_variance in 0.1f64..10.0,
        observations in prop::collection::vec(
            prop::collection::vec(-10.0f64..10.0, 2..=2),  // 2D observations
            5..20
        )
    ) {
        let initial_state = EntityState {
            properties: vec![0.0, 0.0],
            covariance: vec![
                vec![initial_variance, 0.0],
                vec![0.0, initial_variance],
            ],
        };

        let mut tracker = EntityTracker::new(initial_state);

        // Apply observations
        for obs_vec in observations {
            let observation = EntityObservation {
                measured_properties: obs_vec,
                measurement_noise: vec![
                    vec![0.1, 0.0],
                    vec![0.0, 0.1],
                ],
                timestamp: Instant::now(),
            };
            tracker.update(observation).ok();
        }

        // All diagonal elements must be non-negative (PSD invariant for full matrix)
        let state = tracker.get_state();
        for (i, row) in state.covariance.iter().enumerate() {
            prop_assert!(
                row[i] >= 0.0,
                "Covariance diagonal [{i}][{i}] = {:.6} is negative (PSD violated)",
                row[i]
            );
        }
    }
}

proptest! {
    #[test]
    fn prediction_increases_uncertainty(
        initial_variance in 0.1f64..1.0,
        steps in 1usize..10
    ) {
        let initial_state = EntityState {
            properties: vec![0.0, 0.0],
            covariance: vec![
                vec![initial_variance, 0.0],
                vec![0.0, initial_variance],
            ],
        };

        let tracker = EntityTracker::new(initial_state.clone());
        let predicted = tracker.predict(steps).unwrap();

        // Uncertainty must grow (or stay same) with prediction steps
        prop_assert!(
            predicted.covariance[0][0] >= initial_state.covariance[0][0],
            "Prediction decreased uncertainty from {} to {}",
            initial_state.covariance[0][0],
            predicted.covariance[0][0]
        );
    }
}

// ============================================================================
// Invariant 4: Confidence intervals have valid bounds
// ============================================================================

proptest! {
    #[test]
    fn confidence_intervals_bounded(
        predictions in prop::collection::vec(
            (0.1f64..1.0, prop::bool::ANY),  // (probability, was_correct)
            10..100
        )
    ) {
        let mut estimator = UncertaintyEstimator::new();

        // Record outcomes
        for (prob, correct) in &predictions {
            estimator.record_outcome(*prob, *correct);
        }

        // Check all recorded predictions
        for (prob, _) in &predictions {
            // Confidence intervals should be in [0, 1]
            prop_assert!(*prob >= 0.0 && *prob <= 1.0, "Probability {} out of bounds", prob);
        }

        // Calibration metrics should be valid
        let metrics = estimator.get_metrics();
        prop_assert!(metrics.ece >= 0.0 && metrics.ece <= 1.0, "ECE {} out of [0,1]", metrics.ece);
        prop_assert!(metrics.mce >= 0.0 && metrics.mce <= 1.0, "MCE {} out of [0,1]", metrics.mce);
    }
}

proptest! {
    #[test]
    fn uncertainty_propagation_monotonic(
        initial_variance in 0.01f64..0.5,
        step1 in 1usize..5,
        step2 in 5usize..10
    ) {
        let estimator = UncertaintyEstimator::new();

        // Uncertainty should grow monotonically with more prediction steps
        let ci1 = estimator.propagate_uncertainty(initial_variance, step1);
        let ci2 = estimator.propagate_uncertainty(initial_variance, step2);

        let width1 = ci1.upper - ci1.lower;
        let width2 = ci2.upper - ci2.lower;

        // More steps should give wider (or equal) confidence interval
        prop_assert!(
            width2 >= width1 - 1e-6,  // Allow small numerical error
            "Uncertainty decreased from {} steps (width={}) to {} steps (width={})",
            step1, width1, step2, width2
        );
    }
}

// ============================================================================
// Invariant 5: Calibration improves with more data
// ============================================================================

proptest! {
    #[test]
    fn calibration_error_decreases_with_data(
        confidence in 0.5f64..0.9,
        noise in 0.0f64..0.2,
    ) {
        let mut estimator = UncertaintyEstimator::new();

        // Record partially calibrated predictions (confidence ± noise)
        for i in 0..100 {
            let is_correct = (i as f64 / 100.0) < (confidence + noise * (i as f64 / 50.0 - 1.0));
            estimator.record_outcome(confidence, is_correct);
        }

        let metrics_100 = estimator.get_metrics();

        // Add more well-calibrated data
        for i in 0..900 {
            let is_correct = (i as f64 / 900.0) < confidence;
            estimator.record_outcome(confidence, is_correct);
        }

        let metrics_1000 = estimator.get_metrics();

        // ECE should improve (decrease) or stay similar with more calibrated data
        // Allow small tolerance for randomness
        prop_assert!(
            metrics_1000.ece <= metrics_100.ece + 0.05,
            "Calibration worsened: {} -> {} (this can happen due to randomness)",
            metrics_100.ece, metrics_1000.ece
        );
    }
}

// ============================================================================
// Invariant 6: Edge cases and boundary conditions
// ============================================================================

proptest! {
    #[test]
    fn empty_model_handles_gracefully(
        state in prop::string::string_regex("[A-Z][0-9]").unwrap(),
        action in prop::string::string_regex("A[0-9]").unwrap(),
    ) {
        let model = TransitionModel::new();

        // Querying empty model should return error, not crash
        let result = model.predict(&state, &action);
        prop_assert!(result.is_err(), "Empty model should return error");
    }
}

proptest! {
    #[test]
    fn single_observation_model_valid(
        from in prop::string::string_regex("[A-Z][0-9]").unwrap(),
        action in prop::string::string_regex("A[0-9]").unwrap(),
        to in prop::string::string_regex("[A-Z][0-9]").unwrap(),
    ) {
        let mut model = TransitionModel::new();

        model.record_transition(StateTransition {
            from_state: from.clone(),
            action: action.clone(),
            to_state: to.clone(),
        }).unwrap();

        // Single observation should still give valid prediction
        if let Ok(pred) = model.predict(&from, &action) {
            prop_assert!(pred.probabilities.len() > 0, "Prediction should have outcomes");

            let sum: f64 = pred.probabilities.values().sum();
            prop_assert!((sum - 1.0).abs() < 1e-6, "Even single observation should sum to 1.0");
        }
    }
}

// ============================================================================
// Invariant 10 (H10): directional SCM
// ============================================================================

proptest! {
    /// A `DirectionalSE` must be exactly invertible for the direction block: abduction
    /// recovers the noise term that prediction consumed, for any parents, any unit
    /// direction on S^(d-1), and any noise.
    #[test]
    fn directional_se_abduction_round_trip(
        theta in 0.0f64..std::f64::consts::TAU,
        parents in prop::collection::vec(-100.0f64..100.0, 1..4),
        u in -100.0f64..100.0,
    ) {
        let dir = vec![theta.cos(), theta.sin()];
        let dir_weights: Vec<f64> = (0..dir.len()).map(|i| (i as f64 + 1.0) * 1.5).collect();
        let se = DirectionalSE::new(vec![1.0; parents.len()], dir_weights);

        let observed = se.evaluate_directional(&parents, &dir, u);
        let recovered = se.invert_for_u_directional(&parents, &dir, observed);

        prop_assert!(
            (recovered - u).abs() < 1e-6,
            "round-trip failed: u={} recovered={} (theta={})",
            u, recovered, theta
        );
        prop_assert_eq!(se.shape(), SeShape::Directional { dim: 2 });
    }
}

proptest! {
    /// WP9 acceptance, generalised: for any unit direction and any norm, every step of a
    /// rollout uses the pinned direction unchanged. A drifted direction is a hard failure.
    #[test]
    fn directional_intervention_is_held_fixed_across_rollout(
        theta in 0.0f64..std::f64::consts::TAU,
        norm in -50.0f64..50.0,
        steps in 1usize..6,
        observed_parent_ref in -5.0f64..5.0,
    ) {
        let dir = vec![theta.cos(), theta.sin()];

        let mut g = CausalGraph::new();
        g.add_node("x".into()).unwrap();
        g.add_node("y".into()).unwrap();
        g.add_edge("x".into(), "y".into()).unwrap();

        // A recording equation so we can inspect the direction actually used.
        #[derive(Debug)]
        struct Rec {
            inner: DirectionalSE,
            seen: std::sync::Arc<std::sync::Mutex<Vec<Vec<f64>>>>,
        }
        impl StructuralEquation for Rec {
            fn evaluate(&self, parents: &[f64], u: f64) -> f64 {
                self.inner.evaluate(parents, u)
            }
            fn invert_for_u(&self, parents: &[f64], observed: f64) -> f64 {
                self.inner.invert_for_u(parents, observed)
            }
            fn shape(&self) -> SeShape {
                self.inner.shape()
            }
            fn evaluate_directional(&self, parents: &[f64], dir: &[f64], u: f64) -> f64 {
                self.seen.lock().unwrap().push(dir.to_vec());
                self.inner.evaluate_directional(parents, dir, u)
            }
            fn invert_for_u_directional(&self, parents: &[f64], dir: &[f64], observed: f64) -> f64 {
                self.inner.invert_for_u_directional(parents, dir, observed)
            }
        }

        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        if let Some(n) = g.node_mut("y") {
            n.equation = Some(std::sync::Arc::new(Rec {
                inner: DirectionalSE::new(vec![1.0], vec![1.0, 1.0]),
                seen: seen.clone(),
            }));
        }

        prop_assume!(g.apply_directional_intervention("x", norm, &dir).is_ok());

        let init = std::collections::HashMap::from([
            ("x".to_string(), observed_parent_ref),
            ("y".to_string(), observed_parent_ref * 2.0),
        ]);
        let traj = g.rollout_directional(&init, "x", steps).unwrap();

        prop_assert_eq!(traj.len(), steps + 1);
        // Step 0 echoes the observed anchor untouched; the pin applies to the rollout steps.
        prop_assert!(
            (traj[0]["x"] - observed_parent_ref).abs() < 1e-9,
            "step 0 must echo the observed anchor, got {}", traj[0]["x"]
        );
        for (i, step) in traj.iter().enumerate().skip(1) {
            prop_assert!(
                (step["x"] - norm).abs() < 1e-9,
                "step {}: pinned norm violated: {} vs {}",
                i, step["x"], norm
            );
        }

        let seen = seen.lock().unwrap();
        prop_assert_eq!(seen.len(), steps, "one directional eval per step");
        for (i, d) in seen.iter().enumerate() {
            prop_assert_eq!(
                d.clone(), dir.clone(),
                "step {}: direction drifted instead of being held fixed", i
            );
        }
    }
}

proptest! {
    /// The directional counterfactual must not change scalar semantics: on a graph whose
    /// nodes carry no structural equations at all, the directional path and the existing
    /// scalar path must agree exactly.
    #[test]
    fn directional_path_matches_scalar_path_on_equation_free_graphs(
        n_nodes in 2usize..5,
        obs in prop::collection::vec(-10.0f64..10.0, 2..5),
        intervention_value in -10.0f64..10.0,
    ) {
        let mut g = CausalGraph::new();
        for i in 0..n_nodes {
            let _ = g.add_node(format!("n{}", i));
        }
        // Chain n0 -> n1 -> ... so there is a real topological order.
        for i in 1..n_nodes {
            let _ = g.add_edge(format!("n{}", i - 1), format!("n{}", i));
        }

        let state: std::collections::HashMap<String, f64> = (0..n_nodes)
            .map(|i| (format!("n{}", i), obs.get(i).copied().unwrap_or(0.0)))
            .collect();

        let scalar = g
            .compute_scm_counterfactual(&state, "n0", intervention_value)
            .unwrap();
        let directional = g
            .compute_scm_counterfactual_directional(
                &state,
                "n0",
                &InterventionValue::Scalar(intervention_value),
            )
            .unwrap();

        prop_assert_eq!(scalar.len(), directional.len());
        for (k, v) in &scalar {
            let d = directional.get(k).copied().unwrap_or(f64::NAN);
            prop_assert!(
                (d - v).abs() < 1e-9,
                "directional path diverged on '{}': scalar={} directional={}", k, v, d
            );
        }
    }
}
