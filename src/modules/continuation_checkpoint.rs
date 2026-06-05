// Continuation Checkpoint — Session-resumable reasoning state
//
// Stores a snapshot of an agent session's reasoning state as a first-class
// AuditLog entry. Enables session death recovery: a fresh session asks
// "give me the latest checkpoint for task τ" and resumes from it.
//
// Design principle: the checkpoint captures WHAT was decided and WHAT to do next,
// not HOW to compute it. The store owns state; the client owns computation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::procedural_cache::{FSMState, ProceduralTrace};

/// A single decision recorded during a reasoning session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Unique identifier for this decision
    pub id: Uuid,
    /// What was decided (human-readable)
    pub summary: String,
    /// The FSM state at the time of decision
    pub fsm_state: Option<FSMState>,
    /// When the decision was made
    pub timestamp: DateTime<Utc>,
    /// Confidence score [0.0, 1.0]
    pub confidence: f64,
    /// Additional structured context
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A continuation checkpoint — the minimal state needed to resume a session.
///
/// Stored as an AuditLog entry so it inherits hash-chain integrity and
/// tamper-evidence. Querying for the latest checkpoint for a task yields
/// the exact state to seed a new working set from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationCheckpoint {
    /// Unique identifier for this checkpoint
    pub id: Uuid,
    /// The task this checkpoint belongs to
    pub task_id: Uuid,
    /// Human-readable task description
    pub task_description: String,
    /// The session that created this checkpoint
    pub session_id: Uuid,
    /// Current FSM traces (snapshot of procedural state)
    pub fsm_traces: HashMap<Uuid, ProceduralTrace>,
    /// Decisions made so far in this task
    pub decisions: Vec<DecisionRecord>,
    /// The next step the session intended to take
    pub next_intended_step: String,
    /// Current FSM state for the primary trace (if any)
    pub current_fsm_state: Option<FSMState>,
    /// Checkpoint creation time
    pub created_at: DateTime<Utc>,
    /// Sequence number within this task (monotonically increasing)
    pub sequence_number: u64,
    /// Additional task-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ContinuationCheckpoint {
    /// Create a new checkpoint for a task.
    pub fn new(
        task_id: Uuid,
        task_description: String,
        session_id: Uuid,
        sequence_number: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id,
            task_description,
            session_id,
            fsm_traces: HashMap::new(),
            decisions: Vec::new(),
            next_intended_step: String::new(),
            current_fsm_state: None,
            created_at: Utc::now(),
            sequence_number,
            metadata: HashMap::new(),
        }
    }

    /// Record a decision made during reasoning.
    pub fn record_decision(
        &mut self,
        summary: String,
        fsm_state: Option<FSMState>,
        confidence: f64,
    ) {
        self.decisions.push(DecisionRecord {
            id: Uuid::new_v4(),
            summary,
            fsm_state,
            timestamp: Utc::now(),
            confidence,
            metadata: HashMap::new(),
        });
    }

    /// Set the next intended step for resumption.
    pub fn set_next_step(&mut self, step: String) {
        self.next_intended_step = step;
    }

    /// Snapshot the current FSM traces into this checkpoint.
    pub fn snapshot_fsm(&mut self, traces: HashMap<Uuid, ProceduralTrace>) {
        self.fsm_traces = traces;
    }

    /// Set the current FSM state.
    pub fn set_fsm_state(&mut self, state: FSMState) {
        self.current_fsm_state = Some(state);
    }

    /// Add arbitrary metadata.
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if this is a terminal checkpoint (task complete).
    pub fn is_terminal(&self) -> bool {
        matches!(&self.current_fsm_state, Some(FSMState::End))
    }

    /// Get a summary for display/logging.
    pub fn summary(&self) -> String {
        format!(
            "Checkpoint #{} for task '{}': {} decisions, FSM={:?}, next='{}'",
            self.sequence_number,
            self.task_description,
            self.decisions.len(),
            self.current_fsm_state,
            self.next_intended_step,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let task_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let cp =
            ContinuationCheckpoint::new(task_id, "Test reasoning task".to_string(), session_id, 1);

        assert_eq!(cp.task_id, task_id);
        assert_eq!(cp.session_id, session_id);
        assert_eq!(cp.sequence_number, 1);
        assert_eq!(cp.decisions.len(), 0);
        assert!(!cp.is_terminal());
    }

    #[test]
    fn test_checkpoint_with_decisions() {
        let task_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let mut cp =
            ContinuationCheckpoint::new(task_id, "Multi-step task".to_string(), session_id, 1);

        cp.record_decision(
            "Chose algorithm A over B".to_string(),
            Some(FSMState::Reason),
            0.85,
        );
        cp.record_decision(
            "Applied fix to module X".to_string(),
            Some(FSMState::Act),
            0.92,
        );
        cp.set_next_step("Verify fix with integration test".to_string());
        cp.set_fsm_state(FSMState::Reflexion);

        assert_eq!(cp.decisions.len(), 2);
        assert_eq!(cp.next_intended_step, "Verify fix with integration test");
        assert_eq!(cp.current_fsm_state, Some(FSMState::Reflexion));
        assert!(!cp.is_terminal());
    }

    #[test]
    fn test_terminal_checkpoint() {
        let task_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let mut cp =
            ContinuationCheckpoint::new(task_id, "Completed task".to_string(), session_id, 3);
        cp.set_fsm_state(FSMState::End);

        assert!(cp.is_terminal());
    }

    #[test]
    fn test_checkpoint_summary() {
        let task_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let mut cp =
            ContinuationCheckpoint::new(task_id, "Debug session".to_string(), session_id, 2);
        cp.record_decision(
            "Identified root cause".to_string(),
            Some(FSMState::Reason),
            0.95,
        );
        cp.set_next_step("Apply patch".to_string());

        let summary = cp.summary();
        assert!(summary.contains("Debug session"));
        assert!(summary.contains("1 decisions"));
        assert!(summary.contains("Apply patch"));
    }

    #[test]
    fn test_metadata_attachment() {
        let task_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let cp =
            ContinuationCheckpoint::new(task_id, "Task with metadata".to_string(), session_id, 1)
                .with_metadata("priority".to_string(), serde_json::json!("high"))
                .with_metadata("estimated_steps".to_string(), serde_json::json!(5));

        assert_eq!(cp.metadata.len(), 2);
        assert_eq!(
            cp.metadata.get("priority").unwrap(),
            &serde_json::json!("high")
        );
    }
}
