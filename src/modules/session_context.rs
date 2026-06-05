// Session Context — Client context window as Tier 0 of the memory hierarchy
//
// Extends the TemporalIndexer's tier model outward across the MCP boundary.
// The client's context window is modeled as the fastest, smallest, most
// volatile tier (Tier 0) of the same decay/eviction hierarchy HipCortex
// already manages for short-term and long-term memory.
//
// Design: HipCortex tracks what it has paged into each session, with what
// recency, and decides what to evict (page-out) and what to refresh. The
// decay factors that govern STM/LTM now also govern client-context residency.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// A single item paged into a client's context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedItem {
    /// The entity or trace ID paged into context
    pub entity_id: Uuid,
    /// When this item was paged into the session
    pub paged_at: SystemTime,
    /// Relevance score at time of paging [0.0, 1.0]
    pub relevance: f32,
    /// How many times the client has accessed this item
    pub access_count: u64,
    /// Last time the client accessed this item
    pub last_accessed: SystemTime,
    /// The item's content hash (for freshness checks)
    pub content_hash: Option<String>,
}

/// A client session's context window, managed by HipCortex as Tier 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Unique session identifier
    pub session_id: Uuid,
    /// Token budget for this session's context window
    pub token_budget: usize,
    /// Items currently paged into the client's context
    pub paged_items: Vec<PagedItem>,
    /// Estimated token count of all paged items
    pub estimated_tokens: usize,
    /// When this session was created
    pub created_at: SystemTime,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// Session metadata
    pub metadata: HashMap<String, String>,
}

impl SessionContext {
    /// Create a new session context with a token budget.
    pub fn new(session_id: Uuid, token_budget: usize) -> Self {
        let now = SystemTime::now();
        Self {
            session_id,
            token_budget,
            paged_items: Vec::new(),
            estimated_tokens: 0,
            created_at: now,
            last_activity: now,
            metadata: HashMap::new(),
        }
    }

    /// Page an item into the client's context window.
    ///
    /// Returns true if the item was paged in, false if it would exceed budget.
    pub fn page_in(
        &mut self,
        entity_id: Uuid,
        relevance: f32,
        token_cost: usize,
        content_hash: Option<String>,
    ) -> bool {
        // Check if already paged (update instead)
        if let Some(existing) = self
            .paged_items
            .iter_mut()
            .find(|i| i.entity_id == entity_id)
        {
            existing.relevance = relevance;
            existing.last_accessed = SystemTime::now();
            existing.access_count += 1;
            existing.content_hash = content_hash;
            self.last_activity = SystemTime::now();
            return true;
        }

        // Check budget
        if self.estimated_tokens + token_cost > self.token_budget {
            return false;
        }

        let now = SystemTime::now();
        self.paged_items.push(PagedItem {
            entity_id,
            paged_at: now,
            relevance,
            access_count: 1,
            last_accessed: now,
            content_hash,
        });
        self.estimated_tokens += token_cost;
        self.last_activity = now;
        true
    }

    /// Page an item out of the client's context window.
    ///
    /// Returns the token cost freed, or 0 if the item wasn't paged.
    pub fn page_out(&mut self, entity_id: Uuid, token_cost: usize) -> usize {
        if let Some(pos) = self
            .paged_items
            .iter()
            .position(|i| i.entity_id == entity_id)
        {
            self.paged_items.remove(pos);
            self.estimated_tokens = self.estimated_tokens.saturating_sub(token_cost);
            self.last_activity = SystemTime::now();
            token_cost
        } else {
            0
        }
    }

    /// Evict stale items that haven't been accessed within the given duration.
    ///
    /// Returns the number of items evicted and total tokens freed.
    pub fn evict_stale(&mut self, max_age: Duration, token_cost_per_item: usize) -> (usize, usize) {
        let now = SystemTime::now();
        let before_len = self.paged_items.len();

        self.paged_items.retain(|item| {
            now.duration_since(item.last_accessed)
                .unwrap_or(Duration::ZERO)
                < max_age
        });

        let evicted = before_len - self.paged_items.len();
        let tokens_freed = evicted * token_cost_per_item;
        self.estimated_tokens = self.estimated_tokens.saturating_sub(tokens_freed);

        (evicted, tokens_freed)
    }

    /// Evict lowest-relevance items to make room for new items.
    ///
    /// Returns the number of items evicted.
    pub fn evict_lowest_relevance(
        &mut self,
        tokens_needed: usize,
        token_cost_per_item: usize,
    ) -> usize {
        let items_to_evict = (tokens_needed + token_cost_per_item - 1) / token_cost_per_item;

        if items_to_evict == 0 || self.paged_items.is_empty() {
            return 0;
        }

        // Sort by relevance (ascending) and remove the lowest
        self.paged_items.sort_by(|a, b| {
            a.relevance
                .partial_cmp(&b.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let evict_count = items_to_evict.min(self.paged_items.len());
        self.paged_items.drain(0..evict_count);
        self.estimated_tokens = self
            .estimated_tokens
            .saturating_sub(evict_count * token_cost_per_item);
        self.last_activity = SystemTime::now();

        evict_count
    }

    /// Get the remaining token budget.
    pub fn remaining_budget(&self) -> usize {
        self.token_budget.saturating_sub(self.estimated_tokens)
    }

    /// Check if a specific entity is currently paged.
    pub fn is_paged(&self, entity_id: Uuid) -> bool {
        self.paged_items.iter().any(|i| i.entity_id == entity_id)
    }

    /// Get the number of paged items.
    pub fn item_count(&self) -> usize {
        self.paged_items.len()
    }

    /// Mark session activity (prevents eviction).
    pub fn touch(&mut self) {
        self.last_activity = SystemTime::now();
    }

    /// Check if the session has been idle beyond the given duration.
    pub fn is_idle(&self, max_idle: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.last_activity)
            .unwrap_or(Duration::ZERO)
            > max_idle
    }

    /// Get items sorted by relevance (highest first).
    pub fn top_by_relevance(&self, n: usize) -> Vec<&PagedItem> {
        let mut items: Vec<&PagedItem> = self.paged_items.iter().collect();
        items.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items.truncate(n);
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session_id = Uuid::new_v4();
        let ctx = SessionContext::new(session_id, 10000);

        assert_eq!(ctx.session_id, session_id);
        assert_eq!(ctx.token_budget, 10000);
        assert_eq!(ctx.paged_items.len(), 0);
        assert_eq!(ctx.estimated_tokens, 0);
    }

    #[test]
    fn test_page_in_within_budget() {
        let mut ctx = SessionContext::new(Uuid::new_v4(), 1000);
        let entity_id = Uuid::new_v4();

        assert!(ctx.page_in(entity_id, 0.9, 500, None));
        assert_eq!(ctx.item_count(), 1);
        assert_eq!(ctx.estimated_tokens, 500);
        assert!(ctx.is_paged(entity_id));
    }

    #[test]
    fn test_page_in_exceeds_budget() {
        let mut ctx = SessionContext::new(Uuid::new_v4(), 100);

        assert!(ctx.page_in(Uuid::new_v4(), 0.9, 80, None));
        assert!(!ctx.page_in(Uuid::new_v4(), 0.8, 80, None)); // Would be 160 > 100
        assert_eq!(ctx.item_count(), 1);
    }

    #[test]
    fn test_page_in_duplicate_updates() {
        let mut ctx = SessionContext::new(Uuid::new_v4(), 1000);
        let entity_id = Uuid::new_v4();

        assert!(ctx.page_in(entity_id, 0.5, 200, None));
        assert!(ctx.page_in(entity_id, 0.9, 200, None)); // Update, not add

        assert_eq!(ctx.item_count(), 1);
        let item = &ctx.paged_items[0];
        assert_eq!(item.access_count, 2);
        assert!((item.relevance - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_page_out() {
        let mut ctx = SessionContext::new(Uuid::new_v4(), 1000);
        let entity_id = Uuid::new_v4();

        ctx.page_in(entity_id, 0.7, 300, None);
        assert_eq!(ctx.estimated_tokens, 300);

        let freed = ctx.page_out(entity_id, 300);
        assert_eq!(freed, 300);
        assert_eq!(ctx.estimated_tokens, 0);
        assert_eq!(ctx.item_count(), 0);
    }

    #[test]
    fn test_evict_stale() {
        let mut ctx = SessionContext::new(Uuid::new_v4(), 1000);

        // Page in items (they'll have last_accessed = now, so won't be stale yet)
        ctx.page_in(Uuid::new_v4(), 0.5, 100, None);
        ctx.page_in(Uuid::new_v4(), 0.5, 100, None);

        // Evict with very short max_age — items were just created, should survive
        let (evicted, _) = ctx.evict_stale(Duration::from_secs(3600), 100);
        assert_eq!(evicted, 0);
        assert_eq!(ctx.item_count(), 2);
    }

    #[test]
    fn test_evict_lowest_relevance() {
        let mut ctx = SessionContext::new(Uuid::new_v4(), 1000);

        // Page in items with different relevance
        ctx.page_in(Uuid::new_v4(), 0.2, 100, None);
        ctx.page_in(Uuid::new_v4(), 0.5, 100, None);
        ctx.page_in(Uuid::new_v4(), 0.9, 100, None);

        // Need 200 tokens (2 items' worth)
        let evicted = ctx.evict_lowest_relevance(200, 100);
        assert_eq!(evicted, 2);
        assert_eq!(ctx.item_count(), 1);
        // The remaining item should be the highest relevance one
        assert!((ctx.paged_items[0].relevance - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_remaining_budget() {
        let mut ctx = SessionContext::new(Uuid::new_v4(), 1000);
        assert_eq!(ctx.remaining_budget(), 1000);

        ctx.page_in(Uuid::new_v4(), 0.5, 350, None);
        assert_eq!(ctx.remaining_budget(), 650);
    }

    #[test]
    fn test_idle_detection() {
        let ctx = SessionContext::new(Uuid::new_v4(), 1000);
        // Session just created — should not be idle
        assert!(!ctx.is_idle(Duration::from_secs(1)));
    }

    #[test]
    fn test_top_by_relevance() {
        let mut ctx = SessionContext::new(Uuid::new_v4(), 1000);

        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();

        ctx.page_in(id_a, 0.3, 100, None);
        ctx.page_in(id_b, 0.9, 100, None);
        ctx.page_in(id_c, 0.6, 100, None);

        let top = ctx.top_by_relevance(2);
        assert_eq!(top.len(), 2);
        assert!((top[0].relevance - 0.9).abs() < 0.001); // Highest first
        assert!((top[1].relevance - 0.6).abs() < 0.001);
    }
}
