#[cfg(feature = "web-server")]
mod tests {
    use hipcortex::web_server::check_rollout_depth;
    use hipcortex::world_model_enhanced::WorldModelEnhanced;

    #[test]
    fn six_actions_returns_max_depth_error() {
        let actions: Vec<String> = (0..6).map(|i| format!("a{i}")).collect();
        let err = check_rollout_depth(&actions, None).unwrap_err();
        assert!(
            err.contains("max_depth"),
            "error must mention 'max_depth': {err}"
        );
    }

    #[test]
    fn five_actions_ok() {
        let actions: Vec<String> = (0..5).map(|i| format!("a{i}")).collect();
        assert!(check_rollout_depth(&actions, Some(5)).is_ok());
    }

    #[test]
    fn max_depth_six_returns_error() {
        let actions: Vec<String> = (0..3).map(|i| format!("a{i}")).collect();
        let err = check_rollout_depth(&actions, Some(6)).unwrap_err();
        assert!(
            err.contains("max_depth"),
            "error must mention 'max_depth': {err}"
        );
    }

    #[test]
    fn dirichlet_rollout_confidence_is_finite() {
        let wm = WorldModelEnhanced::new();
        let _ = wm.observe_transition(
            "state_a".to_string(),
            "move".to_string(),
            "state_b".to_string(),
        );
        let actions = vec!["move".to_string(), "stop".to_string()];
        if let Ok(pred) = wm.rollout_dirichlet("state_a".to_string(), actions.clone()) {
            assert!(
                pred.confidence.is_finite(),
                "confidence must be finite: {}",
                pred.confidence
            );
            let uncertainty = 1.0 - pred.confidence;
            assert!(
                uncertainty.is_finite(),
                "uncertainty must be finite: {uncertainty}"
            );
        }
    }
}
