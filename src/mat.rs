//! Memoized Arbitration Table — caches AttributionReport keyed by ConflictSignature.
use crate::world_model_enhanced::causal::AttributionReport;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConflictSignature {
    hash: u64,
    raw: String,
}

impl ConflictSignature {
    pub fn from_raw(s: &str) -> Self {
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        Self { hash: h.finish(), raw: s.to_string() }
    }
}

pub struct AttributionCache {
    entries: HashMap<ConflictSignature, AttributionReport>,
    capacity: usize,
}

impl AttributionCache {
    pub fn new() -> Self {
        Self { entries: HashMap::new(), capacity: 256 }
    }

    pub fn insert(&mut self, sig: ConflictSignature, report: AttributionReport) {
        if self.entries.len() >= self.capacity {
            if let Some(k) = self.entries.keys().next().cloned() {
                self.entries.remove(&k);
            }
        }
        self.entries.insert(sig, report);
    }

    pub fn get(&self, sig: &ConflictSignature) -> Option<&AttributionReport> {
        self.entries.get(sig)
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    pub fn recent(&self, limit: usize) -> Vec<&AttributionReport> {
        let mut v: Vec<&AttributionReport> = self.entries.values().collect();
        v.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(limit);
        v
    }
}

impl Default for AttributionCache {
    fn default() -> Self { Self::new() }
}
