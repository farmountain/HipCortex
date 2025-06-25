use serde_json::Value;

use crate::latent_map::LatentMapVersion;

/// Scores latent maps against observed state and prunes low-confidence entries.
pub struct LatentMapEvaluator;

impl LatentMapEvaluator {
    /// Compute a simple similarity score between two JSON values.
    pub fn score(expected: &Value, observed: &Value) -> f32 {
        similarity(expected, observed)
    }

    /// Remove all map versions below the confidence threshold.
    pub fn prune_low_confidence(maps: &mut Vec<LatentMapVersion>, threshold: f32) {
        maps.retain(|m| m.confidence >= threshold);
    }

    /// Collapse multiple maps to the most confident one if above threshold.
    pub fn collapse(maps: &mut Vec<LatentMapVersion>, threshold: f32) -> Option<LatentMapVersion> {
        Self::prune_low_confidence(maps, threshold);
        maps.iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .cloned()
    }
}

fn similarity(a: &Value, b: &Value) -> f32 {
    if a == b {
        return 1.0;
    }
    let a_count = count_leaves(a);
    let b_count = count_leaves(b);
    if a_count + b_count == 0 {
        return 1.0;
    }
    let inter = count_intersection(a, b);
    (2 * inter) as f32 / (a_count + b_count) as f32
}

fn count_leaves(v: &Value) -> usize {
    match v {
        Value::Object(map) => map.values().map(count_leaves).sum(),
        Value::Array(arr) => arr.iter().map(count_leaves).sum(),
        _ => 1,
    }
}

fn count_intersection(a: &Value, b: &Value) -> usize {
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => ma
            .iter()
            .filter_map(|(k, va)| mb.get(k).map(|vb| count_intersection(va, vb)))
            .sum(),
        (Value::Array(va), Value::Array(vb)) => va
            .iter()
            .zip(vb.iter())
            .map(|(va, vb)| count_intersection(va, vb))
            .sum(),
        _ => {
            if a == b {
                1
            } else {
                0
            }
        }
    }
}
