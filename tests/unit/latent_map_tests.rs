use hipcortex::latent_map::LatentMapVersion;
use hipcortex::latent_map_evaluator::LatentMapEvaluator;
use serde_json::json;

#[test]
fn score_and_collapse() {
    let mut maps = vec![
        LatentMapVersion::new(json!({"a":1}), 0.2),
        LatentMapVersion::new(json!({"a":1}), 0.8),
    ];
    let winner = LatentMapEvaluator::collapse(&mut maps, 0.5).unwrap();
    assert!(winner.confidence >= 0.8);
    let score = LatentMapEvaluator::score(&winner.map, &json!({"a":1}));
    assert!((score - 1.0).abs() < 1e-6);
}
