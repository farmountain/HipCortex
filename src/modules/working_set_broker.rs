// Working Set Broker — Active context-window manager
//
// The load-bearing component from the five-improvement architecture.
//
// HipCortex currently is a passive durable store. The Working Set Broker
// transforms it into an active working-memory broker — the "OS layer" that
// owns the client's context window as a managed resource.
//
// Core operation: materialize_working_set(task, budget) → BoundedPayload
//
// Given a task τ and token budget b, the broker assembles a minimal payload
// from the Symbolic Store, Temporal Indexer, Procedural Cache, and any
// available continuation checkpoints — guaranteed to fit within b tokens.
//
// This dissolves the fundamental problem: S → ∞, W fixed. The broker makes
// every page-in bounded by construction, removing the unbounded variable
// from the client entirely.

use std::collections::HashMap;
use uuid::Uuid;

use crate::audit_log::AuditLog;
use crate::broker_policy::{
    BoundedPayload, BrokerPolicy, DefaultBrokerPolicy, TaskDescription, TemporalTraceSummary,
};
use crate::continuation_checkpoint::ContinuationCheckpoint;
use crate::procedural_cache::ProceduralCache;
use crate::session_context::{PagedItem, SessionContext};
use crate::symbolic_store::{GraphDatabase, InMemoryGraph, SymbolicStore};
use crate::temporal_indexer::{TemporalIndexer, TemporalTrace};

/// The Working Set Broker — orchestrates context materialization.
///
/// Owns session contexts (Tier 0), applies broker policies, and integrates
/// with checkpoints for session resumption.
pub struct WorkingSetBroker {
    /// Active sessions and their context windows (Tier 0)
    sessions: HashMap<Uuid, SessionContext>,
    /// The broker policy for selecting what to include
    policy: Box<dyn BrokerPolicy>,
    /// Default token budget for new sessions
    default_budget: usize,
    /// Estimated tokens per node (for budget calculations)
    tokens_per_node: usize,
    /// Estimated tokens per temporal trace
    tokens_per_trace: usize,
    /// Max idle time before a session is considered stale
    max_session_idle_secs: u64,
}

impl WorkingSetBroker {
    /// Create a new broker with default configuration.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            policy: Box::new(DefaultBrokerPolicy),
            default_budget: 100_000, // ~100K token default budget
            tokens_per_node: 50,
            tokens_per_trace: 40,
            max_session_idle_secs: 3600, // 1 hour
        }
    }

    /// Create with a custom policy.
    pub fn with_policy(policy: Box<dyn BrokerPolicy>) -> Self {
        Self {
            sessions: HashMap::new(),
            policy,
            default_budget: 100_000,
            tokens_per_node: 50,
            tokens_per_trace: 40,
            max_session_idle_secs: 3600,
        }
    }

    /// Create with full configuration.
    pub fn with_config(
        policy: Box<dyn BrokerPolicy>,
        default_budget: usize,
        tokens_per_node: usize,
        tokens_per_trace: usize,
        max_session_idle_secs: u64,
    ) -> Self {
        Self {
            sessions: HashMap::new(),
            policy,
            default_budget,
            tokens_per_node,
            tokens_per_trace,
            max_session_idle_secs,
        }
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Create a new session with the default budget.
    pub fn create_session(&mut self, session_id: Uuid) -> &SessionContext {
        let ctx = SessionContext::new(session_id, self.default_budget);
        self.sessions.insert(session_id, ctx);
        self.sessions.get(&session_id).unwrap()
    }

    /// Create a session with a custom budget.
    pub fn create_session_with_budget(
        &mut self,
        session_id: Uuid,
        budget: usize,
    ) -> &SessionContext {
        let ctx = SessionContext::new(session_id, budget);
        self.sessions.insert(session_id, ctx);
        self.sessions.get(&session_id).unwrap()
    }

    /// Get a session context (or create if not exists).
    pub fn get_or_create_session(&mut self, session_id: Uuid) -> &mut SessionContext {
        if !self.sessions.contains_key(&session_id) {
            self.create_session(session_id);
        }
        self.sessions.get_mut(&session_id).unwrap()
    }

    /// Remove a session and free its context.
    pub fn remove_session(&mut self, session_id: Uuid) -> bool {
        self.sessions.remove(&session_id).is_some()
    }

    /// Clean up stale sessions.
    ///
    /// Returns the number of sessions removed.
    pub fn cleanup_stale_sessions(&mut self) -> usize {
        let stale_ids: Vec<Uuid> = self
            .sessions
            .iter()
            .filter(|(_, ctx)| {
                ctx.is_idle(std::time::Duration::from_secs(self.max_session_idle_secs))
            })
            .map(|(id, _)| *id)
            .collect();

        let count = stale_ids.len();
        for id in stale_ids {
            self.sessions.remove(&id);
        }
        count
    }

    // ========================================================================
    // Core Operation: Materialize Working Set
    // ========================================================================

    /// Materialize a bounded working set for a task under a token budget.
    ///
    /// This is the primary API — the one operation that closes the gap between
    /// unbounded durable state S and bounded client context W.
    ///
    /// Algorithm:
    /// 1. Check for a continuation checkpoint for this task → seed from it
    /// 2. Query the Symbolic Store for relevant subgraph nodes/edges
    /// 3. Query the Temporal Indexer for recent traces
    /// 4. Apply the broker policy to select what fits in the budget
    /// 5. Page the selected items into the session's Tier 0 context
    /// 6. Return the bounded payload
    pub fn materialize_working_set(
        &mut self,
        task: TaskDescription,
        budget: usize,
        session_id: Uuid,
        store: &SymbolicStore<InMemoryGraph>,
        indexer: &TemporalIndexer<Uuid>,
        _cache: &ProceduralCache,
        audit: Option<&AuditLog>,
    ) -> BoundedPayload {
        // Phase 1: Extract all immutable data from self BEFORE mutable borrow
        let tokens_per_trace = self.tokens_per_trace;
        let tokens_per_node = self.tokens_per_node;
        let (fsm_state, next_step) = if let Some(audit_log) = audit {
            self.load_latest_checkpoint_state(&task.task_id, audit_log)
        } else {
            (None, None)
        };

        // Phase 2: Get store data (independent of self)
        let all_nodes = store.backend().all_nodes();
        let all_edges = store.backend().all_edges();

        // Phase 3: Compute trace summaries (before mutable borrow)
        let effective_budget = {
            let session = self.get_or_create_session(session_id);
            budget.min(session.remaining_budget().max(100))
        };
        let max_traces = effective_budget / tokens_per_trace;
        let recent = indexer.get_recent(max_traces);
        let trace_summaries: Vec<TemporalTraceSummary> = recent
            .iter()
            .map(|t| TemporalTraceSummary {
                id: t.id,
                timestamp: format!("{:?}", t.timestamp),
                relevance: t.relevance,
                summary: format!("Trace {} (relevance: {:.2})", t.id, t.relevance),
            })
            .collect();

        // Phase 4: Apply policy (no borrow of self.sessions)
        let mut payload = self.policy.select_working_set(
            &task,
            effective_budget,
            &all_nodes,
            &all_edges,
            &trace_summaries,
            fsm_state.as_deref(),
            next_step.as_deref(),
        );

        payload.session_id = session_id;

        // Phase 5: Page items into session context (re-acquire session)
        let session = self.get_or_create_session(session_id);
        for node in &payload.nodes {
            session.page_in(node.id, 0.8, tokens_per_node, None);
        }
        for trace in &payload.temporal_traces {
            session.page_in(trace.id, trace.relevance, tokens_per_trace, None);
        }

        payload
    }

    /// Materialize a working set seeded from a specific checkpoint.
    ///
    /// Used for session resumption — recreates the working set from the
    /// state captured in a continuation checkpoint.
    pub fn materialize_from_checkpoint(
        &mut self,
        task: TaskDescription,
        budget: usize,
        session_id: Uuid,
        checkpoint: &ContinuationCheckpoint,
        store: &SymbolicStore<InMemoryGraph>,
        indexer: &TemporalIndexer<Uuid>,
    ) -> BoundedPayload {
        let fsm_state = checkpoint
            .current_fsm_state
            .as_ref()
            .map(|s| format!("{:?}", s));
        let next_step = if checkpoint.next_intended_step.is_empty() {
            None
        } else {
            Some(checkpoint.next_intended_step.clone())
        };

        // Build task context from checkpoint decisions
        let mut task_with_context = task.clone();
        for decision in &checkpoint.decisions {
            task_with_context.context.insert(
                format!("decision_{}", decision.id),
                decision.summary.clone(),
            );
        }

        self.materialize_working_set(
            task_with_context,
            budget,
            session_id,
            store,
            indexer,
            &ProceduralCache::new(),
            None,
        )
    }

    // ========================================================================
    // Checkpoint Integration
    // ========================================================================

    /// Load the latest checkpoint state for a task from the AuditLog.
    ///
    /// Returns (fsm_state, next_step) if a checkpoint exists.
    fn load_latest_checkpoint_state(
        &self,
        task_id: &Uuid,
        audit: &AuditLog,
    ) -> (Option<String>, Option<String>) {
        // Query audit log for checkpoint entries for this task
        // In full implementation: deserialize ContinuationCheckpoint from audit entries
        // For now: return None (checkpoint loading requires AuditLog extension)
        let _ = task_id;
        let _ = audit;
        (None, None)
    }

    // ========================================================================
    // Tier 0 Management
    // ========================================================================

    /// Evict stale items from a session's context.
    pub fn evict_stale_from_session(
        &mut self,
        session_id: Uuid,
        max_age_secs: u64,
    ) -> (usize, usize) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.evict_stale(
                std::time::Duration::from_secs(max_age_secs),
                self.tokens_per_node,
            )
        } else {
            (0, 0)
        }
    }

    /// Check remaining budget for a session.
    pub fn session_budget_remaining(&self, session_id: Uuid) -> Option<usize> {
        self.sessions.get(&session_id).map(|s| s.remaining_budget())
    }

    /// Get the number of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for WorkingSetBroker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decay::DecayType;
    use crate::symbolic_store::{GraphDatabase, InMemoryGraph, SymbolicStore};
    use crate::temporal_indexer::{TemporalIndexer, TemporalTrace};
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_create_session() {
        let mut broker = WorkingSetBroker::new();
        let session_id = Uuid::new_v4();

        let ctx = broker.create_session(session_id);
        assert_eq!(ctx.session_id, session_id);
        assert_eq!(ctx.token_budget, 100_000);
        assert_eq!(broker.active_session_count(), 1);
    }

    #[test]
    fn test_create_session_custom_budget() {
        let mut broker = WorkingSetBroker::new();
        let session_id = Uuid::new_v4();

        broker.create_session_with_budget(session_id, 50_000);
        let remaining = broker.session_budget_remaining(session_id);
        assert_eq!(remaining, Some(50_000));
    }

    #[test]
    fn test_get_or_create_session() {
        let mut broker = WorkingSetBroker::new();
        let session_id = Uuid::new_v4();

        // First call creates
        broker.get_or_create_session(session_id);
        assert_eq!(broker.active_session_count(), 1);

        // Second call returns existing
        let ctx2_id = broker.get_or_create_session(session_id).session_id;
        assert_eq!(broker.active_session_count(), 1);
        assert_eq!(ctx2_id, session_id);
    }

    #[test]
    fn test_remove_session() {
        let mut broker = WorkingSetBroker::new();
        let session_id = Uuid::new_v4();

        broker.create_session(session_id);
        assert!(broker.remove_session(session_id));
        assert_eq!(broker.active_session_count(), 0);
        assert!(!broker.remove_session(session_id)); // Already removed
    }

    #[test]
    fn test_materialize_respects_budget() {
        let mut broker = WorkingSetBroker::new();
        let session_id = Uuid::new_v4();
        let store = SymbolicStore::new();
        let mut indexer = TemporalIndexer::<Uuid>::new(100, 3600);
        let cache = ProceduralCache::new();

        let task = TaskDescription {
            task_id: Uuid::new_v4(),
            description: "Test materialization".to_string(),
            tags: vec!["test".to_string()],
            priority: 5,
            context: HashMap::new(),
        };

        let budget = 500;
        let payload = broker
            .materialize_working_set(task, budget, session_id, &store, &indexer, &cache, None);

        assert!(
            payload.estimated_tokens <= budget,
            "Payload estimated_tokens {} should not exceed budget {}",
            payload.estimated_tokens,
            budget
        );
        assert_eq!(payload.session_id, session_id);
    }

    #[test]
    fn test_materialize_with_store_data() {
        let mut broker = WorkingSetBroker::new();
        let session_id = Uuid::new_v4();
        let mut store = SymbolicStore::new();
        let mut indexer = TemporalIndexer::<Uuid>::new(100, 3600);
        let cache = ProceduralCache::new();

        // Add some data to the store
        let node_id = store.add_node("function_handle_request", HashMap::new());

        // Add a temporal trace
        let trace = TemporalTrace {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            data: node_id,
            relevance: 0.9,
            decay_factor: 1.0,
            last_access: SystemTime::now(),
            decay_type: DecayType::Exponential {
                half_life: Duration::from_secs(3600),
            },
        };
        indexer.insert(trace);

        let task = TaskDescription {
            task_id: Uuid::new_v4(),
            description: "Handle request".to_string(),
            tags: vec!["function".to_string()],
            priority: 7,
            context: HashMap::new(),
        };

        let payload =
            broker.materialize_working_set(task, 2000, session_id, &store, &indexer, &cache, None);

        // Should have selected the function node and the trace
        assert!(
            payload.nodes.len() > 0 || payload.temporal_traces.len() > 0,
            "Payload should contain data from the store"
        );
    }
}
