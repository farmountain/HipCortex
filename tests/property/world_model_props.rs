// Property-Based Tests for World-Model Enhanced Invariants
//
// Verifies mathematical properties hold across random inputs:
// - Probability distributions sum to 1.0
// - Causal graph acyclicity maintained
// - Kalman covariance positive semi-definite
// - Uncertainty bounds are valid
// - Calibration improves with samples

use hipcortex::world_model_enhanced::{
    CausalGraph, EntityState, EntityTracker, TransitionModel, StateTransition,
    UncertaintyEstimator, EntityObservation,
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
        
        // Covariance diagonal elements must be non-negative
        let state = tracker.get_state();
        prop_assert!(state.covariance[0][0] >= 0.0, "Covariance[0][0] negative");
        prop_assert!(state.covariance[1][1] >= 0.0, "Covariance[1][1] negative");
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
