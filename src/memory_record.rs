use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ValueEnum, Hash)]
pub enum MemoryType {
    Temporal,
    Symbolic,
    Procedural,
    Reflexion,
    Perception,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: Uuid,
    pub record_type: MemoryType,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub target: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub integrity: Option<String>,
    // Optimization fields
    #[serde(default)]
    pub access_count: u32,
    #[serde(default)]
    pub last_accessed: DateTime<Utc>,
    #[serde(default = "default_relevance")]
    pub relevance_score: f64,
    #[serde(default)]
    pub content_hash: Option<String>,
    /// Unix timestamp (seconds) when this record expires. None = never expires.
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// Confidence score [0.0, 1.0] — how reliable is this memory? Default 1.0.
    /// Lower values signal uncertain or unverified information.
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    /// Source identifier — who or what wrote this memory.
    /// Examples: "user-input", "claude-3-7", "system", "sensor-array-1"
    #[serde(default)]
    pub source: Option<String>,
    /// Version counter — increments on every in-place update.
    /// Version 0 = original write.
    #[serde(default)]
    pub version: u32,
    /// Tags for categorization and RAG filtering (e.g. ["bug", "architecture", "decision"])
    #[serde(default)]
    pub tags: Vec<String>,
    /// Memory priority: "pinned" bypasses decay and always appears in search.
    /// Values: "pinned" | "high" | "normal" | "low". Default "normal".
    #[serde(default = "default_priority")]
    pub priority: String,
}

fn default_relevance() -> f64 {
    1.0
}

fn default_confidence() -> f32 {
    1.0
}

fn default_priority() -> String {
    "normal".to_string()
}

impl MemoryRecord {
    pub fn new(
        record_type: MemoryType,
        actor: String,
        action: String,
        target: String,
        metadata: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        let mut rec = Self {
            id: Uuid::new_v4(),
            record_type,
            timestamp: now,
            actor,
            action,
            target,
            metadata,
            integrity: None,
            access_count: 0,
            last_accessed: now,
            relevance_score: 1.0,
            content_hash: None,
            expires_at: None,
            confidence: 1.0,
            source: None,
            version: 0,
            tags: Vec::new(),
            priority: "normal".to_string(),
        };
        let hash = rec.compute_hash();
        rec.integrity = Some(hash.clone());
        rec.content_hash = Some(hash);
        rec
    }

    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut clone = self.clone();
        clone.integrity = None;
        clone.content_hash = None;
        clone.access_count = 0;  // Exclude access tracking from hash
        clone.last_accessed = self.timestamp;  // Use original timestamp for consistency
        let data = serde_json::to_vec(&clone).unwrap();
        let hash = Sha256::digest(&data);
        hex::encode(hash)
    }

    /// Mark this memory as accessed, updating access tracking
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }

    /// Update relevance score with bounds checking
    pub fn update_relevance_score(&mut self, new_score: f64) {
        self.relevance_score = new_score.clamp(0.0, 1.0);
    }

    /// Calculate decay factor based on access patterns and age
    pub fn calculate_decay_factor(&self, base_decay: f64) -> f64 {
        let now = Utc::now();
        let age_seconds = (now - self.timestamp).num_seconds() as f64;
        
        // Slower decay for frequently accessed memories
        let access_factor = 1.0 + (self.access_count as f64 * 0.1);
        let time_factor = (-age_seconds / 86400.0 * base_decay).exp(); // Daily decay
        
        (time_factor * access_factor * self.relevance_score).clamp(0.0, 1.0)
    }

    /// Get typed metadata field
    pub fn get_metadata_field<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.metadata.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set metadata field with type safety
    pub fn set_metadata_field<T>(&mut self, key: &str, value: T) -> Result<(), serde_json::Error>
    where
        T: serde::Serialize,
    {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key.to_string(), serde_json::to_value(value)?);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), serde_json::to_value(value)?);
            self.metadata = serde_json::Value::Object(map);
        }
        
        // Recompute hash after metadata change
        let new_hash = self.compute_hash();
        self.integrity = Some(new_hash.clone());
        self.content_hash = Some(new_hash);
        
        Ok(())
    }

    /// Check if this memory should be pruned based on access patterns
    pub fn should_prune(&self, min_access_threshold: u32, max_age_days: i64) -> bool {
        let now = Utc::now();
        let age_days = (now - self.timestamp).num_days();
        let last_access_days = (now - self.last_accessed).num_days();
        
        (self.access_count < min_access_threshold && age_days > max_age_days) ||
        (last_access_days > max_age_days * 2) ||
        (self.relevance_score < 0.1)
    }

    /// Calculate memory similarity based on content
    pub fn similarity_score(&self, other: &MemoryRecord) -> f64 {
        let mut score = 0.0;
        let mut factors = 0.0;

        // Actor similarity
        if self.actor == other.actor {
            score += 0.3;
        }
        factors += 0.3;

        // Action similarity
        if self.action == other.action {
            score += 0.4;
        }
        factors += 0.4;

        // Target similarity
        if self.target == other.target {
            score += 0.3;
        }
        factors += 0.3;

        // Type similarity
        if self.record_type == other.record_type {
            score += 0.2;
        }
        factors += 0.2;

        score / factors
    }
}
