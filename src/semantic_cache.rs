use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticCache {
    capacity: usize,
    map: HashMap<String, Vec<f32>>,
    order: VecDeque<String>,
}

impl SemanticCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn load<P: AsRef<Path>>(path: P, capacity: usize) -> Self {
        if let Ok(file) = File::open(&path) {
            if let Ok(cache) = serde_json::from_reader(file) {
                return cache;
            }
        }
        Self::new(capacity)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn put_embedding(&mut self, key: String, embedding: Vec<f32>) {
        let ctx = if embedding.is_empty() {
            "invalid".to_string()
        } else {
            format!("cache_put:{}", embedding.len())
        };
        if crate::safety_guardrail::SAFETY_GUARDRAIL
            .lock()
            .unwrap()
            .check_precondition(&ctx)
            .is_err()
        {
            return;
        }
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else if self.map.len() == self.capacity {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            }
        }
        self.map.insert(key.clone(), embedding);
        self.order.push_back(key);
    }

    pub fn get_nearest(&mut self, query: &[f32]) -> Option<(String, Vec<f32>)> {
        let mut best: Option<(String, f32)> = None;
        for (k, v) in &self.map {
            let sim = cosine_similarity(query, v);
            if best.as_ref().map_or(true, |(_, b)| sim > *b) {
                best = Some((k.clone(), sim));
            }
        }
        if let Some((key, _)) = best {
            if let Some(vec) = self.map.get(&key).cloned() {
                self.order.retain(|k| k != &key);
                self.order.push_back(key.clone());
                return Some((key, vec));
            }
        }
        None
    }

    pub fn invalidate(&mut self, key: &str) -> bool {
        let existed = self.map.remove(key).is_some();
        if existed {
            self.order.retain(|k| k != key);
        }
        existed
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_put_get() {
        let mut cache = SemanticCache::new(2);
        cache.put_embedding("a".into(), vec![1.0, 0.0]);
        let res = cache.get_nearest(&[1.0, 0.0]).unwrap();
        assert_eq!(res.0, "a");
    }

    #[test]
    fn capacity_eviction() {
        let mut cache = SemanticCache::new(1);
        cache.put_embedding("a".into(), vec![1.0, 0.0]);
        cache.put_embedding("b".into(), vec![0.0, 1.0]);
        assert!(cache.get_nearest(&[1.0, 0.0]).unwrap().0 == "b");
        assert_eq!(cache.len(), 1);
    }
}
