//! Ω Loop Engine: transactional cycle orchestrator (snapshot → straTa gap → simulate → surprise → Bayesian attr → topo mutate → coherence gate → commit/rollback).
//!
//! Chain-of-Thought:
//! 1. Snapshot current topo state (leverage SnapshotManager + in-mem copy).
//! 2. Detect coverage gaps via topo (PPR / localized) → produce StrataTrajectory stub.
//! 3. Simulate forward (delegate to WM + topo paths).
//! 4. Surprise / error signal → Bayesian attribution on edges.
//! 5. Mutate topo (error-driven add/update).
//! 6. Coherence gate (using CoherenceChecker) decides rollback vs commit via AuditLog + SafetyGuardrail.
//!
//! Uses existing: CausalTopoGraph, WorldModelEnhanced, CoherenceChecker, SnapshotManager, AuditLog, SafetyGuardrail.
//! Pure TDD, surgical. No changes outside loop_engine.rs + lib.rs registration.

use crate::audit_log::AuditLog;
use crate::coherence::CoherenceChecker;
use crate::safety_guardrail::SafetyGuardrail;
use crate::snapshot_manager::SnapshotManager;
use crate::topological_memory::CausalTopoGraph;
use crate::world_model_enhanced::WorldModelEnhanced;

use std::collections::HashMap;

/// Minimal metrics for loop observability (expanded in later tasks).
#[derive(Debug, Clone, Default)]
pub struct LoopMetrics {
    pub iterations: u64,
    pub snapshots_taken: u64,
    pub gaps_detected: u64,
    pub mutations: u64,
    pub rollbacks: u64,
}

/// Stub trajectory representing stratified simulation layers (straTa).
#[derive(Debug, Clone, Default)]
pub struct StrataTrajectory {
    pub layers: Vec<String>,
    pub coverage_score: f32,
}

/// Minimal belief state (can be extended to hold Dirichlet priors etc).
#[derive(Debug, Clone, Default)]
pub struct BeliefState {
    pub beliefs: HashMap<String, f32>,
}

/// Result of a snapshot capture for transactional cycle.
#[derive(Debug, Clone)]
pub struct IterationSnapshot {
    pub topo_node_count: usize,
    pub topo_edge_count: usize,
    pub timestamp: u64,
    pub belief_snapshot: BeliefState,
}

pub struct LoopEngine {
    pub topo: CausalTopoGraph,
    pub wm: WorldModelEnhanced,
    pub coherence: CoherenceChecker,
    pub snapshot_mgr: SnapshotManager,
    pub audit: AuditLog,
    pub safety: SafetyGuardrail,
    pub metrics: LoopMetrics,
    pub active_strata: Option<StrataTrajectory>,
    pub state: BeliefState,
}

impl LoopEngine {
    /// Construct a new LoopEngine. Uses in-memory sink for audit (tests / embedded use).
    /// Caller owns the initial topo substrate.
    pub fn new(topo: CausalTopoGraph) -> Self {
        Self {
            topo,
            wm: WorldModelEnhanced::new(),
            coherence: CoherenceChecker::new(),
            snapshot_mgr: SnapshotManager,
            audit: AuditLog::new_sink(),
            safety: SafetyGuardrail::new(),
            metrics: LoopMetrics::default(),
            active_strata: None,
            state: BeliefState::default(),
        }
    }

    /// Create a transactional snapshot of current iteration state.
    /// Uses SnapshotManager pattern + direct topo introspection (for MVP no full disk tar yet; future tasks wire file snapshot of serialized topo).
    pub fn create_iteration_snapshot(&mut self) -> Result<IterationSnapshot, String> {
        // Minimal transactional snapshot impl.
        // Follows project safety rule: always call check_precondition before mutating / snapshot path.
        if let Err(reason) = self.safety.check_precondition("loop_engine::create_iteration_snapshot") {
            return Err(format!("safety block: {}", reason));
        }

        self.metrics.snapshots_taken += 1;

        // SnapshotManager pattern: in full impl would serialize topo state to a temp path and call
        // SnapshotManager::save(temp_path, &format!("omega-{}", self.metrics.iterations)).
        // For skeleton we capture essential state in-memory (topo stats + belief) to satisfy
        // the transactional cycle contract and test. Future tasks will persist when serialization added.
        let snap = IterationSnapshot {
            topo_node_count: self.topo.node_count(),
            topo_edge_count: 0, // petgraph .edge_count() private; exposed via future method on CausalTopoGraph if needed
            timestamp: 0,
            belief_snapshot: self.state.clone(),
        };
        Ok(snap)
    }

    /// High-level entry for full Ω cycle. Currently a hard-stop stub + snapshot attempt.
    /// Steps 3-5 of plan will flesh straTa, simulate, mutate etc.
    pub fn run_omega_loop(&mut self) -> Result<(), String> {
        // Hard stop per plan: always snapshot first.
        let _snap = self.create_iteration_snapshot()?;
        // gap/straTa stub
        if let Some(strata) = self.detect_coverage_gap() {
            self.active_strata = Some(strata);
        }
        Ok(())
    }

    /// Step 3 stub: detect gaps using topo capabilities (PPR etc in future).
    /// Returns dummy strata for now.
    pub fn detect_coverage_gap(&self) -> Option<StrataTrajectory> {
        if self.topo.node_count() == 0 {
            return Some(StrataTrajectory {
                layers: vec!["root::perception".to_string(), "root::action".to_string()],
                coverage_score: 0.0,
            });
        }
        None
    }

    // Future: simulate, attribute_bayesian, mutate_topo, coherence_gate, etc.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_omega_loop_snapshot() {
        let topo = CausalTopoGraph::new();
        let mut engine = LoopEngine::new(topo);
        let snap = engine.create_iteration_snapshot();
        assert!(snap.is_ok(), "snapshot should succeed after minimal impl");
    }

    #[test]
    fn test_omega_loop_gap_detection_stub() {
        let topo = CausalTopoGraph::new();
        let engine = LoopEngine::new(topo);
        let gap = engine.detect_coverage_gap();
        assert!(gap.is_some(), "empty topo should report coverage gap via straTa stub");
    }

    #[test]
    fn test_omega_loop_basic_run_snapshot_strata() {
        // TDD for Step 3-5: run reaches snapshot + gap/straTa stub
        let topo = CausalTopoGraph::new();
        let mut engine = LoopEngine::new(topo);
        let res = engine.run_omega_loop();
        assert!(res.is_ok());
        assert!(engine.active_strata.is_some(), "run should populate active_strata from detect");
        assert!(engine.metrics.snapshots_taken >= 1);
    }
}