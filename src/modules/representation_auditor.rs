use crate::latent_map::LatentMapVersion;
use crate::latent_map_evaluator::LatentMapEvaluator;
use crate::world_model::WorldModel;

/// Periodically checks that all modules agree on the current world model.
pub struct RepresentationAuditor;

impl RepresentationAuditor {
    /// Returns true if the latent map aligns with the exported world model.
    pub fn is_aligned(map: &LatentMapVersion, model: &WorldModel) -> bool {
        let (nodes, _) = model.export();
        let val = serde_json::json!({"nodes": nodes});
        LatentMapEvaluator::score(&map.map, &val) > 0.5
    }
}
