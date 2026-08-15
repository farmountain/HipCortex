use crate::memory_record::{MemoryRecord, MemoryType};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub target: Option<String>,
    pub record_type: Option<MemoryType>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub min_relevance: Option<f64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<SortCriteria>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortCriteria {
    Timestamp,
    Relevance,
    AccessCount,
    LastAccessed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult {
    pub records: Vec<MemoryRecord>,
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub successful: Vec<Uuid>,
    pub failed: Vec<(MemoryRecord, String)>,
    pub total_processed: usize,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryStoreStats {
    pub total_records: usize,
    pub records_by_type: HashMap<MemoryType, usize>,
    pub avg_access_count: f64,
    pub avg_relevance_score: f64,
    pub oldest_record: Option<DateTime<Utc>>,
    pub newest_record: Option<DateTime<Utc>>,
    pub cache_hit_rate: f64,
}

pub struct OptimizedMemoryStore {
    records: Arc<RwLock<HashMap<Uuid, MemoryRecord>>>,
    indices: Arc<RwLock<MemoryIndices>>,
    cache: Arc<RwLock<QueryCache>>,
    stats: Arc<RwLock<MemoryStoreStats>>,
    config: MemoryStoreConfig,
}

#[derive(Debug)]
struct MemoryIndices {
    by_actor: HashMap<String, Vec<Uuid>>,
    by_action: HashMap<String, Vec<Uuid>>,
    by_target: HashMap<String, Vec<Uuid>>,
    by_type: HashMap<MemoryType, Vec<Uuid>>,
    by_timestamp: VecDeque<(DateTime<Utc>, Uuid)>,
    by_relevance: Vec<(f64, Uuid)>,
}

#[derive(Debug)]
struct QueryCache {
    cache: HashMap<String, (PaginatedResult, DateTime<Utc>)>,
    max_size: usize,
    ttl_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct MemoryStoreConfig {
    pub max_cache_size: usize,
    pub cache_ttl_seconds: i64,
    pub auto_prune_enabled: bool,
    pub prune_threshold_days: i64,
    pub min_access_threshold: u32,
    pub batch_size: usize,
}

impl Default for MemoryStoreConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 1000,
            cache_ttl_seconds: 300, // 5 minutes
            auto_prune_enabled: true,
            prune_threshold_days: 30,
            min_access_threshold: 2,
            batch_size: 100,
        }
    }
}

impl OptimizedMemoryStore {
    pub fn new(config: MemoryStoreConfig) -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            indices: Arc::new(RwLock::new(MemoryIndices::new())),
            cache: Arc::new(RwLock::new(QueryCache::new(
                config.max_cache_size,
                config.cache_ttl_seconds,
            ))),
            stats: Arc::new(RwLock::new(MemoryStoreStats::default())),
            config,
        }
    }

    /// Add a single memory record with automatic indexing
    pub fn add_memory(&self, mut record: MemoryRecord) -> Result<Uuid> {
        let record_id = record.id;

        // Update stats
        record.mark_accessed();

        // Add to main store
        {
            let mut records = self.records.write().unwrap();
            records.insert(record_id, record.clone());
        }

        // Update indices
        {
            let mut indices = self.indices.write().unwrap();
            indices.add_record(&record);
        }

        // Update statistics
        self.update_stats();

        // Invalidate related cache entries
        self.invalidate_cache(&record);

        Ok(record_id)
    }

    /// Add multiple memory records in a batch operation
    pub async fn add_memories_batch(
        &self,
        records: Vec<MemoryRecord>,
    ) -> Result<BatchOperationResult> {
        let start_time = std::time::Instant::now();
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let total_processed = records.len();

        for record in records {
            match self.add_memory(record.clone()) {
                Ok(id) => successful.push(id),
                Err(e) => failed.push((record, e.to_string())),
            }
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(BatchOperationResult {
            successful,
            failed,
            total_processed,
            processing_time_ms,
        })
    }

    /// Query memories with pagination and caching
    pub fn query_memories_paginated(
        &self,
        query: &MemoryQuery,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult> {
        // Check cache first
        let cache_key = self.generate_cache_key(query, page, page_size);
        if let Some(cached_result) = self.get_cached_result(&cache_key) {
            return Ok(cached_result);
        }

        // Execute query
        let all_matching_ids = self.execute_query(query)?;
        let total_count = all_matching_ids.len();
        let total_pages = (total_count + page_size - 1) / page_size;

        // Apply pagination
        let start_idx = page * page_size;
        let end_idx = (start_idx + page_size).min(total_count);
        let page_ids = &all_matching_ids[start_idx..end_idx];

        // Fetch records and mark as accessed
        let mut records = Vec::new();
        {
            let mut records_map = self.records.write().unwrap();
            for &id in page_ids {
                if let Some(record) = records_map.get_mut(&id) {
                    record.mark_accessed();
                    records.push(record.clone());
                }
            }
        }

        // Sort if requested
        if let Some(sort_criteria) = &query.sort_by {
            self.sort_records(&mut records, sort_criteria);
        }

        let result = PaginatedResult {
            records,
            total_count,
            page,
            page_size,
            total_pages,
        };

        // Cache the result
        self.cache_result(cache_key, result.clone());

        Ok(result)
    }

    /// Get memory store statistics
    pub fn get_stats(&self) -> MemoryStoreStats {
        self.stats.read().unwrap().clone()
    }

    /// Prune old and irrelevant memories
    pub fn prune_memories(&self) -> Result<usize> {
        if !self.config.auto_prune_enabled {
            return Ok(0);
        }

        let mut pruned_count = 0;
        let ids_to_remove: Vec<Uuid> = {
            let records = self.records.read().unwrap();
            records
                .values()
                .filter(|record| {
                    record.should_prune(
                        self.config.min_access_threshold,
                        self.config.prune_threshold_days,
                    )
                })
                .map(|record| record.id)
                .collect()
        };

        for id in ids_to_remove {
            if self.remove_memory(id).is_ok() {
                pruned_count += 1;
            }
        }

        Ok(pruned_count)
    }

    /// Remove a memory record
    pub fn remove_memory(&self, id: Uuid) -> Result<()> {
        let record = {
            let mut records = self.records.write().unwrap();
            records.remove(&id)
        };

        if let Some(record) = record {
            // Remove from indices
            let mut indices = self.indices.write().unwrap();
            indices.remove_record(&record);

            // Clear cache
            let mut cache = self.cache.write().unwrap();
            cache.clear();

            // Update stats
            self.update_stats();
        }

        Ok(())
    }

    /// Find similar memories to a given record
    pub fn find_similar_memories(
        &self,
        record: &MemoryRecord,
        threshold: f64,
        limit: usize,
    ) -> Result<Vec<(MemoryRecord, f64)>> {
        let records = self.records.read().unwrap();
        let mut similarities: Vec<(MemoryRecord, f64)> = records
            .values()
            .filter(|r| r.id != record.id)
            .map(|r| (r.clone(), record.similarity_score(r)))
            .filter(|(_, score)| *score >= threshold)
            .collect();

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        similarities.truncate(limit);

        Ok(similarities)
    }

    // Helper methods

    fn execute_query(&self, query: &MemoryQuery) -> Result<Vec<Uuid>> {
        let indices = self.indices.read().unwrap();
        let mut candidate_ids: Option<Vec<Uuid>> = None;

        // Apply filters progressively
        if let Some(actor) = &query.actor {
            candidate_ids = Some(self.intersect_ids(candidate_ids, indices.by_actor.get(actor)));
        }

        if let Some(action) = &query.action {
            candidate_ids = Some(self.intersect_ids(candidate_ids, indices.by_action.get(action)));
        }

        if let Some(target) = &query.target {
            candidate_ids = Some(self.intersect_ids(candidate_ids, indices.by_target.get(target)));
        }

        if let Some(record_type) = &query.record_type {
            candidate_ids =
                Some(self.intersect_ids(candidate_ids, indices.by_type.get(record_type)));
        }

        let mut result_ids =
            candidate_ids.unwrap_or_else(|| self.records.read().unwrap().keys().cloned().collect());

        // Apply additional filters
        if query.from_timestamp.is_some()
            || query.to_timestamp.is_some()
            || query.min_relevance.is_some()
        {
            let records = self.records.read().unwrap();
            result_ids.retain(|id| {
                if let Some(record) = records.get(id) {
                    self.record_matches_query(record, query)
                } else {
                    false
                }
            });
        }

        Ok(result_ids)
    }

    fn intersect_ids(&self, existing: Option<Vec<Uuid>>, new: Option<&Vec<Uuid>>) -> Vec<Uuid> {
        match (existing, new) {
            (None, None) => Vec::new(),
            (None, Some(new)) => new.clone(),
            (Some(existing), None) => existing,
            (Some(existing), Some(new)) => {
                existing.into_iter().filter(|id| new.contains(id)).collect()
            }
        }
    }

    fn record_matches_query(&self, record: &MemoryRecord, query: &MemoryQuery) -> bool {
        if let Some(from) = query.from_timestamp {
            if record.timestamp < from {
                return false;
            }
        }

        if let Some(to) = query.to_timestamp {
            if record.timestamp > to {
                return false;
            }
        }

        if let Some(min_relevance) = query.min_relevance {
            if record.relevance_score < min_relevance {
                return false;
            }
        }

        true
    }

    fn sort_records(&self, records: &mut [MemoryRecord], criteria: &SortCriteria) {
        match criteria {
            SortCriteria::Timestamp => {
                records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            }
            SortCriteria::Relevance => {
                records.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
            }
            SortCriteria::AccessCount => {
                records.sort_by(|a, b| b.access_count.cmp(&a.access_count));
            }
            SortCriteria::LastAccessed => {
                records.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
            }
        }
    }

    fn generate_cache_key(&self, query: &MemoryQuery, page: usize, page_size: usize) -> String {
        use sha2::{Digest, Sha256};
        let query_str = format!("{:?}:{}:{}", query, page, page_size);
        let hash = Sha256::digest(query_str.as_bytes());
        hex::encode(hash)
    }

    fn get_cached_result(&self, cache_key: &str) -> Option<PaginatedResult> {
        let cache = self.cache.read().unwrap();
        cache.get(cache_key)
    }

    fn cache_result(&self, cache_key: String, result: PaginatedResult) {
        let mut cache = self.cache.write().unwrap();
        cache.set(cache_key, result);
    }

    fn invalidate_cache(&self, _record: &MemoryRecord) {
        // For now, clear entire cache. Could be more selective in the future.
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    fn update_stats(&self) {
        let records = self.records.read().unwrap();
        let mut stats = self.stats.write().unwrap();

        stats.total_records = records.len();
        stats.records_by_type.clear();

        let mut total_access = 0u64;
        let mut total_relevance = 0.0;
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        for record in records.values() {
            *stats
                .records_by_type
                .entry(record.record_type.clone())
                .or_insert(0) += 1;
            total_access += record.access_count as u64;
            total_relevance += record.relevance_score;

            if oldest.is_none() || record.timestamp < oldest.unwrap() {
                oldest = Some(record.timestamp);
            }
            if newest.is_none() || record.timestamp > newest.unwrap() {
                newest = Some(record.timestamp);
            }
        }

        stats.avg_access_count = if records.is_empty() {
            0.0
        } else {
            total_access as f64 / records.len() as f64
        };
        stats.avg_relevance_score = if records.is_empty() {
            0.0
        } else {
            total_relevance / records.len() as f64
        };
        stats.oldest_record = oldest;
        stats.newest_record = newest;
    }
}

impl MemoryIndices {
    fn new() -> Self {
        Self {
            by_actor: HashMap::new(),
            by_action: HashMap::new(),
            by_target: HashMap::new(),
            by_type: HashMap::new(),
            by_timestamp: VecDeque::new(),
            by_relevance: Vec::new(),
        }
    }

    fn add_record(&mut self, record: &MemoryRecord) {
        self.by_actor
            .entry(record.actor.clone())
            .or_insert_with(Vec::new)
            .push(record.id);
        self.by_action
            .entry(record.action.clone())
            .or_insert_with(Vec::new)
            .push(record.id);
        self.by_target
            .entry(record.target.clone())
            .or_insert_with(Vec::new)
            .push(record.id);
        self.by_type
            .entry(record.record_type.clone())
            .or_insert_with(Vec::new)
            .push(record.id);

        // Maintain sorted timestamp index
        let timestamp = record.timestamp;
        let insert_pos = self
            .by_timestamp
            .iter()
            .position(|(ts, _)| *ts > timestamp)
            .unwrap_or(self.by_timestamp.len());
        self.by_timestamp.insert(insert_pos, (timestamp, record.id));

        // Maintain sorted relevance index
        let relevance = record.relevance_score;
        let insert_pos = self
            .by_relevance
            .iter()
            .position(|(r, _)| *r < relevance)
            .unwrap_or(self.by_relevance.len());
        self.by_relevance.insert(insert_pos, (relevance, record.id));
    }

    fn remove_record(&mut self, record: &MemoryRecord) {
        if let Some(actor_ids) = self.by_actor.get_mut(&record.actor) {
            actor_ids.retain(|&id| id != record.id);
        }
        if let Some(action_ids) = self.by_action.get_mut(&record.action) {
            action_ids.retain(|&id| id != record.id);
        }
        if let Some(target_ids) = self.by_target.get_mut(&record.target) {
            target_ids.retain(|&id| id != record.id);
        }
        if let Some(type_ids) = self.by_type.get_mut(&record.record_type) {
            type_ids.retain(|&id| id != record.id);
        }

        self.by_timestamp.retain(|(_, id)| *id != record.id);
        self.by_relevance.retain(|(_, id)| *id != record.id);
    }
}

impl QueryCache {
    fn new(max_size: usize, ttl_seconds: i64) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            ttl_seconds,
        }
    }

    fn get(&self, key: &str) -> Option<PaginatedResult> {
        if let Some((result, timestamp)) = self.cache.get(key) {
            let now = Utc::now();
            if (now - *timestamp).num_seconds() < self.ttl_seconds {
                return Some(result.clone());
            }
        }
        None
    }

    fn set(&mut self, key: String, result: PaginatedResult) {
        if self.cache.len() >= self.max_size {
            // Simple eviction: remove oldest entry
            if let Some(oldest_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest_key);
            }
        }
        self.cache.insert(key, (result, Utc::now()));
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for MemoryStoreStats {
    fn default() -> Self {
        Self {
            total_records: 0,
            records_by_type: HashMap::new(),
            avg_access_count: 0.0,
            avg_relevance_score: 0.0,
            oldest_record: None,
            newest_record: None,
            cache_hit_rate: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_record::MemoryType;

    #[test]
    fn test_optimized_memory_store_basic_operations() {
        let store = OptimizedMemoryStore::new(MemoryStoreConfig::default());

        let record = MemoryRecord::new(
            MemoryType::Temporal,
            "test_actor".to_string(),
            "test_action".to_string(),
            "test_target".to_string(),
            serde_json::json!({"test": "data"}),
        );

        let id = store.add_memory(record.clone()).unwrap();
        assert_eq!(id, record.id);

        let query = MemoryQuery {
            actor: Some("test_actor".to_string()),
            ..Default::default()
        };

        let result = store.query_memories_paginated(&query, 0, 10).unwrap();
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.total_count, 1);
    }

    #[cfg(feature = "async-store")]
    #[test]
    fn test_batch_operations() {
        let store = OptimizedMemoryStore::new(MemoryStoreConfig::default());

        let records: Vec<MemoryRecord> = (0..5)
            .map(|i| {
                MemoryRecord::new(
                    MemoryType::Temporal,
                    format!("actor_{}", i),
                    "test_action".to_string(),
                    "test_target".to_string(),
                    serde_json::json!({"index": i}),
                )
            })
            .collect();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(store.add_memories_batch(records)).unwrap();

        assert_eq!(result.successful.len(), 5);
        assert_eq!(result.failed.len(), 0);
        assert_eq!(result.total_processed, 5);
    }

    #[test]
    fn test_pagination() {
        let store = OptimizedMemoryStore::new(MemoryStoreConfig::default());

        for i in 0..15 {
            let record = MemoryRecord::new(
                MemoryType::Temporal,
                "test_actor".to_string(),
                format!("action_{}", i),
                "test_target".to_string(),
                serde_json::json!({"index": i}),
            );
            store.add_memory(record).unwrap();
        }

        let query = MemoryQuery {
            actor: Some("test_actor".to_string()),
            ..Default::default()
        };

        // First page
        let page1 = store.query_memories_paginated(&query, 0, 10).unwrap();
        assert_eq!(page1.records.len(), 10);
        assert_eq!(page1.total_count, 15);
        assert_eq!(page1.total_pages, 2);

        // Second page
        let page2 = store.query_memories_paginated(&query, 1, 10).unwrap();
        assert_eq!(page2.records.len(), 5);
        assert_eq!(page2.total_count, 15);
        assert_eq!(page2.total_pages, 2);
    }
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            actor: None,
            action: None,
            target: None,
            record_type: None,
            from_timestamp: None,
            to_timestamp: None,
            min_relevance: None,
            limit: None,
            offset: None,
            sort_by: None,
        }
    }
}
