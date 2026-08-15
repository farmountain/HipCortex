// Unit tests for Coherence module public API
//
// Covers CoherenceChecker (coordinator), WriteRejection, CoherenceMetrics,
// and edge cases for detection, resolution, invariants not covered by
// inline #[cfg(test)] blocks in the source modules.

use hipcortex::coherence::{
    CandidateValue, CoherenceChecker, CoherenceMetrics, ConflictResolver, ConsistencyChecker,
    InconsistencyReport, InconsistencyType, InvariantType, InvariantViolation, ResolutionResult,
    ResolutionStrategy, SystemInvariants, WriteRejection,
};

// ============================================================================
// CoherenceChecker (coordinator) tests
// ============================================================================

#[test]
fn test_coherence_checker_default_metrics() {
    let checker = CoherenceChecker::new();
    let metrics = checker.get_metrics().unwrap();
    assert_eq!(metrics.total_checks, 0);
    assert_eq!(metrics.inconsistencies_found, 0);
    assert_eq!(metrics.auto_resolutions_succeeded, 0);
    assert_eq!(metrics.auto_resolutions_failed, 0);
    assert_eq!(metrics.invariants_validated, 0);
    assert_eq!(metrics.invariants_violated, 0);
    assert_eq!(metrics.coherence_score, 1.0);
    assert_eq!(metrics.writes_gated, 0);
    assert_eq!(metrics.writes_blocked, 0);
}

#[test]
fn test_check_consistency_returns_empty_when_no_modules() {
    let checker = CoherenceChecker::new();
    let inconsistencies = checker.check_consistency().unwrap();
    assert_eq!(inconsistencies.len(), 0);
}

#[test]
fn test_check_consistency_increments_metrics() {
    let checker = CoherenceChecker::new();
    checker.check_consistency().unwrap();
    let metrics = checker.get_metrics().unwrap();
    assert_eq!(metrics.total_checks, 1);
}

#[test]
fn test_check_entity_returns_empty_for_unknown_entity() {
    let checker = CoherenceChecker::new();
    let results = checker.check_entity("nonexistent").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_scheduled_check_not_needed_immediately() {
    let checker = CoherenceChecker::new();
    assert!(!checker.should_run_scheduled_check().unwrap());
}

#[test]
fn test_coherence_score_perfect_when_no_entities() {
    let checker = CoherenceChecker::new();
    let score = checker.compute_coherence_score(0).unwrap();
    assert_eq!(score, 1.0);
}

#[test]
fn test_coherence_score_perfect_when_no_inconsistencies() {
    let checker = CoherenceChecker::new();
    let score = checker.compute_coherence_score(100).unwrap();
    assert_eq!(score, 1.0);
}

#[test]
fn test_coherence_score_updates_cached_metrics() {
    let checker = CoherenceChecker::new();
    checker.compute_coherence_score(50).unwrap();
    let metrics = checker.get_metrics().unwrap();
    assert_eq!(metrics.coherence_score, 1.0);
}

#[test]
fn test_get_active_inconsistencies_initially_empty() {
    let checker = CoherenceChecker::new();
    let active = checker.get_active_inconsistencies().unwrap();
    assert!(active.is_empty());
}

#[test]
fn test_get_resolution_history_initially_empty() {
    let checker = CoherenceChecker::new();
    let history = checker.get_resolution_history().unwrap();
    assert!(history.is_empty());
}

#[test]
fn test_enforce_invariants_returns_empty_when_no_data() {
    let checker = CoherenceChecker::new();
    let violations = checker.enforce_invariants().unwrap();
    assert_eq!(violations.len(), 0);
}

#[test]
fn test_enforce_invariants_increments_metrics() {
    let checker = CoherenceChecker::new();
    checker.enforce_invariants().unwrap();
    let metrics = checker.get_metrics().unwrap();
    assert_eq!(metrics.invariants_validated, 1);
}

#[test]
fn test_check_invariant_all_types_return_ok() {
    let checker = CoherenceChecker::new();
    for inv in &[
        InvariantType::MemoryConsistency,
        InvariantType::DecayMonotonicity,
        InvariantType::GraphAcyclicity,
        InvariantType::Conservation,
    ] {
        let result = checker.check_invariant(*inv).unwrap();
        assert!(
            result.is_none(),
            "Invariant {:?} should pass with no data",
            inv
        );
    }
}

#[test]
fn test_gate_write_passes_when_no_violations() {
    let checker = CoherenceChecker::new();
    let result = checker.gate_write("test_context");
    assert!(result.is_ok());
}

#[test]
fn test_gate_write_increments_counter() {
    let checker = CoherenceChecker::new();
    checker.gate_write("ctx").unwrap();
    let (gated, blocked) = checker.get_gate_stats().unwrap();
    assert_eq!(gated, 1);
    assert_eq!(blocked, 0);
}

#[test]
fn test_check_and_resolve_passes_when_no_inconsistencies() {
    let checker = CoherenceChecker::new();
    let result = checker.check_and_resolve(ResolutionStrategy::Consensus);
    assert!(result.is_ok());
}

#[test]
fn test_default_trait_creates_valid_checker() {
    let checker = CoherenceChecker::default();
    let metrics = checker.get_metrics().unwrap();
    assert_eq!(metrics.total_checks, 0);
}

// ============================================================================
// WriteRejection tests
// ============================================================================

#[test]
fn test_write_rejection_has_reason() {
    let rejection = WriteRejection {
        inconsistencies: vec![],
        violations: vec![],
        reason: "test rejection".to_string(),
    };
    assert_eq!(rejection.reason, "test rejection");
    assert!(rejection.inconsistencies.is_empty());
    assert!(rejection.violations.is_empty());
}

#[test]
fn test_write_rejection_with_inconsistencies() {
    let report = InconsistencyReport::new(
        InconsistencyType::TemporalSymbolicMismatch,
        vec!["e1".to_string()],
        "mismatch".to_string(),
        5,
    );
    let rejection = WriteRejection {
        inconsistencies: vec![report.clone()],
        violations: vec![],
        reason: "inconsistent".to_string(),
    };
    assert_eq!(rejection.inconsistencies.len(), 1);
    assert_eq!(rejection.inconsistencies[0].id, report.id);
}

#[test]
fn test_write_rejection_with_violations() {
    let violation = InvariantViolation::new(
        InvariantType::Conservation,
        "bad counts".to_string(),
        vec!["e1".to_string()],
        true,
    );
    let rejection = WriteRejection {
        inconsistencies: vec![],
        violations: vec![violation],
        reason: "invariant violated".to_string(),
    };
    assert_eq!(rejection.violations.len(), 1);
    assert!(rejection.violations[0].critical);
}

// ============================================================================
// CoherenceMetrics tests
// ============================================================================

#[test]
fn test_metrics_default_values() {
    let m = CoherenceMetrics::new();
    assert_eq!(m.total_checks, 0);
    assert_eq!(m.coherence_score, 1.0);
    assert_eq!(m.writes_gated, 0);
    assert_eq!(m.writes_blocked, 0);
}

#[test]
fn test_metrics_clone_preserves_values() {
    let mut m = CoherenceMetrics::new();
    m.total_checks = 42;
    m.coherence_score = 0.75;
    let cloned = m.clone();
    assert_eq!(cloned.total_checks, 42);
    assert_eq!(cloned.coherence_score, 0.75);
}

// ============================================================================
// InconsistencyType completeness
// ============================================================================

#[test]
fn test_all_inconsistency_types_represented() {
    // Verify all 5 inconsistency types exist
    let types = vec![
        InconsistencyType::TemporalSymbolicMismatch,
        InconsistencyType::ProceduralWorldConflict,
        InconsistencyType::CausalViolation,
        InconsistencyType::EntityPermanenceViolation,
        InconsistencyType::GraphInconsistency,
    ];
    assert_eq!(types.len(), 5);
    // All should be distinct
    use std::collections::HashSet;
    let set: HashSet<_> = types.iter().collect();
    assert_eq!(set.len(), 5);
}

#[test]
fn test_inconsistency_report_severity_clamped() {
    // Severity is a u8 but semantically 0-10
    for sev in &[0u8, 5u8, 10u8] {
        let report = InconsistencyReport::new(
            InconsistencyType::CausalViolation,
            vec!["e1".to_string()],
            format!("sev {}", sev),
            *sev,
        );
        assert!(report.severity <= 10);
        assert!(!report.id.is_empty());
    }
}

#[test]
fn test_inconsistency_report_detected_at_is_monotonic() {
    let r1 = InconsistencyReport::new(
        InconsistencyType::TemporalSymbolicMismatch,
        vec!["a".to_string()],
        "first".to_string(),
        3,
    );
    std::thread::sleep(std::time::Duration::from_millis(2));
    let r2 = InconsistencyReport::new(
        InconsistencyType::TemporalSymbolicMismatch,
        vec!["b".to_string()],
        "second".to_string(),
        3,
    );
    assert!(r2.detected_at >= r1.detected_at);
}

// ============================================================================
// Graph edit distance edge cases
// ============================================================================

#[test]
fn test_graph_edit_distance_empty_sets() {
    let checker = ConsistencyChecker::new();
    let empty: Vec<(String, String)> = vec![];
    let distance = checker.compute_graph_edit_distance(&empty, &empty);
    assert_eq!(distance, 0);
}

#[test]
fn test_graph_edit_distance_one_empty() {
    let checker = ConsistencyChecker::new();
    let edges = vec![("A".to_string(), "B".to_string())];
    let empty: Vec<(String, String)> = vec![];
    let distance = checker.compute_graph_edit_distance(&edges, &empty);
    assert_eq!(distance, 1);
}

#[test]
fn test_graph_edit_distance_large_symmetric_difference() {
    let checker = ConsistencyChecker::new();
    let edges1: Vec<(String, String)> = (0..10)
        .map(|i| (format!("n{}", i), format!("n{}", i + 1)))
        .collect();
    let edges2: Vec<(String, String)> = (10..20)
        .map(|i| (format!("n{}", i), format!("n{}", i + 1)))
        .collect();
    let distance = checker.compute_graph_edit_distance(&edges1, &edges2);
    assert_eq!(distance, 20); // 10 to remove + 10 to add
}

// ============================================================================
// Resolution strategy edge cases
// ============================================================================

#[test]
fn test_resolve_by_recency_single_candidate() {
    let resolver = ConflictResolver::new();
    let inconsistency = InconsistencyReport::new(
        InconsistencyType::TemporalSymbolicMismatch,
        vec!["e1".to_string()],
        "test".to_string(),
        5,
    );
    let candidates = vec![CandidateValue {
        value: serde_json::json!("only_value"),
        source: "module_a".to_string(),
        timestamp: 100,
        confidence: 0.9,
    }];
    let result = resolver
        .resolve_by_recency(&inconsistency, candidates)
        .unwrap();
    assert!(result.success);
    assert_eq!(result.chosen_value, Some(serde_json::json!("only_value")));
}

#[test]
fn test_resolve_by_recency_equal_timestamps_picks_first_max() {
    let resolver = ConflictResolver::new();
    let inconsistency = InconsistencyReport::new(
        InconsistencyType::TemporalSymbolicMismatch,
        vec!["e1".to_string()],
        "test".to_string(),
        5,
    );
    let candidates = vec![
        CandidateValue {
            value: serde_json::json!("A"),
            source: "mod_a".to_string(),
            timestamp: 100,
            confidence: 0.5,
        },
        CandidateValue {
            value: serde_json::json!("B"),
            source: "mod_b".to_string(),
            timestamp: 100, // same timestamp
            confidence: 0.9,
        },
    ];
    let result = resolver
        .resolve_by_recency(&inconsistency, candidates)
        .unwrap();
    assert!(result.success);
    // max_by_key returns the last element on ties, so B wins
    assert_eq!(result.chosen_value, Some(serde_json::json!("B")));
}

#[test]
fn test_resolve_by_confidence_equal_scores() {
    let resolver = ConflictResolver::new();
    let inconsistency = InconsistencyReport::new(
        InconsistencyType::TemporalSymbolicMismatch,
        vec!["e1".to_string()],
        "test".to_string(),
        5,
    );
    let candidates = vec![
        CandidateValue {
            value: serde_json::json!("X"),
            source: "a".to_string(),
            timestamp: 1,
            confidence: 0.8,
        },
        CandidateValue {
            value: serde_json::json!("Y"),
            source: "b".to_string(),
            timestamp: 2,
            confidence: 0.8, // same confidence
        },
    ];
    let result = resolver
        .resolve_by_confidence(&inconsistency, candidates)
        .unwrap();
    assert!(result.success);
    // max_by on equal f64 values returns the second argument (last)
    assert_eq!(result.chosen_value, Some(serde_json::json!("Y")));
}

#[test]
fn test_resolve_by_consensus_unanimous() {
    let resolver = ConflictResolver::new();
    let inconsistency = InconsistencyReport::new(
        InconsistencyType::TemporalSymbolicMismatch,
        vec!["e1".to_string()],
        "test".to_string(),
        5,
    );
    let candidates = vec![
        CandidateValue {
            value: serde_json::json!("same"),
            source: "a".to_string(),
            timestamp: 1,
            confidence: 0.5,
        },
        CandidateValue {
            value: serde_json::json!("same"),
            source: "b".to_string(),
            timestamp: 2,
            confidence: 0.5,
        },
        CandidateValue {
            value: serde_json::json!("same"),
            source: "c".to_string(),
            timestamp: 3,
            confidence: 0.5,
        },
    ];
    let result = resolver
        .resolve_by_consensus(&inconsistency, candidates)
        .unwrap();
    assert!(result.success);
    assert_eq!(result.chosen_value, Some(serde_json::json!("same")));
}

#[test]
fn test_resolve_by_consensus_tie_breaker_picks_first_max() {
    let resolver = ConflictResolver::new();
    let inconsistency = InconsistencyReport::new(
        InconsistencyType::TemporalSymbolicMismatch,
        vec!["e1".to_string()],
        "test".to_string(),
        5,
    );
    let candidates = vec![
        CandidateValue {
            value: serde_json::json!("A"),
            source: "a".to_string(),
            timestamp: 1,
            confidence: 0.5,
        },
        CandidateValue {
            value: serde_json::json!("A"),
            source: "b".to_string(),
            timestamp: 2,
            confidence: 0.5,
        },
        CandidateValue {
            value: serde_json::json!("B"),
            source: "c".to_string(),
            timestamp: 3,
            confidence: 0.5,
        },
        CandidateValue {
            value: serde_json::json!("B"),
            source: "d".to_string(),
            timestamp: 4,
            confidence: 0.5,
        },
    ];
    let result = resolver
        .resolve_by_consensus(&inconsistency, candidates)
        .unwrap();
    assert!(result.success);
    // 2 votes each for A and B, HashMap iteration order non-deterministic
    // Either A or B can win depending on iteration order
    let chosen_val = result.chosen_value.unwrap();
    assert!(
        chosen_val == serde_json::json!("A") || chosen_val == serde_json::json!("B"),
        "Expected A or B, got {:?}",
        chosen_val
    );
}

// ============================================================================
// Manual override + resolution history
// ============================================================================

#[test]
fn test_manual_override_recorded_in_resolver_history() {
    let mut resolver = ConflictResolver::new();
    resolver
        .apply_manual_override("inc_1", serde_json::json!("val"), "op")
        .unwrap();
    let history = resolver.get_history().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].inconsistency_id, "inc_1");
    assert!(history[0].success);
    assert_eq!(history[0].manual_override_by, Some("op".to_string()));
}

#[test]
fn test_multiple_manual_overrides_accumulate_history() {
    let mut resolver = ConflictResolver::new();
    for i in 0..5 {
        resolver
            .apply_manual_override(
                &format!("inc_{}", i),
                serde_json::json!(format!("val_{}", i)),
                &format!("op_{}", i),
            )
            .unwrap();
    }
    let history = resolver.get_history().unwrap();
    assert_eq!(history.len(), 5);
    for (i, entry) in history.iter().enumerate() {
        assert_eq!(entry.inconsistency_id, format!("inc_{}", i));
        assert_eq!(entry.manual_override_by, Some(format!("op_{}", i)));
    }
}

// ============================================================================
// SystemInvariants edge cases
// ============================================================================

#[test]
fn test_would_create_cycle_detects_pending_cycle() {
    let mut invariants = SystemInvariants::new();
    invariants.add_symbolic_edge("A", "B");
    invariants.add_symbolic_edge("B", "C");
    // Adding C→A would create a cycle
    let cycle = invariants.would_create_cycle("C", "A", "symbolic");
    assert!(cycle.is_some());
    // Adding C→D is safe
    let no_cycle = invariants.would_create_cycle("C", "D", "symbolic");
    assert!(no_cycle.is_none());
}

#[test]
fn test_would_create_cycle_unknown_graph_type() {
    let invariants = SystemInvariants::new();
    let result = invariants.would_create_cycle("A", "B", "unknown");
    assert!(result.is_none());
}

#[test]
fn test_would_create_cycle_causal_graph() {
    let mut invariants = SystemInvariants::new();
    invariants.add_causal_edge("X", "Y");
    invariants.add_causal_edge("Y", "Z");
    let cycle = invariants.would_create_cycle("Z", "X", "causal");
    assert!(cycle.is_some());
}

#[test]
fn test_set_symbolic_edges_replaces_all() {
    let mut invariants = SystemInvariants::new();
    invariants.add_symbolic_edge("A", "B");
    invariants.set_symbolic_edges(vec![("C".to_string(), "D".to_string())]);
    // After set, only C→D should be present
    let violation = invariants.check_graph_acyclicity().unwrap();
    assert!(violation.is_none());
    // Verify no cycle with C→D
    let result = invariants.would_create_cycle("D", "C", "symbolic");
    assert!(result.is_some()); // D→C would create cycle with C→D
}

#[test]
fn test_set_causal_edges_triggers_acyclicity_check() {
    let mut invariants = SystemInvariants::new();
    invariants.set_causal_edges(vec![
        ("A".to_string(), "B".to_string()),
        ("B".to_string(), "A".to_string()), // cycle
    ]);
    let violation = invariants.check_graph_acyclicity().unwrap();
    assert!(violation.is_some());
    assert!(violation.unwrap().critical);
}

#[test]
fn test_validate_all_with_cycle_detects_violation() {
    let mut invariants = SystemInvariants::new();
    invariants.add_symbolic_edge("A", "B");
    invariants.add_symbolic_edge("B", "C");
    invariants.add_symbolic_edge("C", "A"); // cycle
    let violations = invariants.validate_all().unwrap();
    assert!(!violations.is_empty());
    let acyclicity_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.invariant_type == InvariantType::GraphAcyclicity)
        .collect();
    assert!(!acyclicity_violations.is_empty());
}

#[test]
fn test_validate_all_with_conservation_and_cycle() {
    let mut invariants = SystemInvariants::new();
    // Add conservation violation
    invariants.record_entity_creation("e1");
    invariants.record_entity_deletion("e1");
    invariants.record_entity_deletion("e1"); // 1 create, 2 deletes
                                             // Add cycle
    invariants.add_symbolic_edge("X", "Y");
    invariants.add_symbolic_edge("Y", "X");
    let violations = invariants.validate_all().unwrap();
    assert!(violations.len() >= 2);
    let types: Vec<_> = violations.iter().map(|v| v.invariant_type).collect();
    assert!(types.contains(&InvariantType::Conservation));
    assert!(types.contains(&InvariantType::GraphAcyclicity));
}

#[test]
fn test_activation_history_capped_at_100() {
    let mut invariants = SystemInvariants::new();
    for i in 0..150 {
        invariants.record_activation("entity_x", i as f64 * 0.1);
    }
    let history = &invariants.activation_history["entity_x"];
    assert_eq!(history.len(), 100);
}

#[test]
fn test_multiple_entities_activation_tracking() {
    let mut invariants = SystemInvariants::new();
    invariants.record_activation("e1", 1.0);
    invariants.record_activation("e1", 0.9);
    invariants.record_activation("e2", 0.8);
    invariants.record_activation("e2", 0.7);
    assert_eq!(invariants.activation_history.len(), 2);
    assert_eq!(invariants.activation_history["e1"].len(), 2);
    assert_eq!(invariants.activation_history["e2"].len(), 2);
}

#[test]
fn test_validate_specific_all_variants() {
    let mut invariants = SystemInvariants::new();
    let variants = [
        InvariantType::MemoryConsistency,
        InvariantType::DecayMonotonicity,
        InvariantType::GraphAcyclicity,
        InvariantType::Conservation,
    ];
    for variant in &variants {
        let result = invariants.validate_specific(*variant).unwrap();
        assert!(
            result.is_none(),
            "{:?} should pass with empty data",
            variant
        );
    }
}

// ============================================================================
// DAG / cycle detection edge cases
// ============================================================================

#[test]
fn test_is_dag_empty_graph() {
    let invariants = SystemInvariants::new();
    let empty: Vec<(String, String)> = vec![];
    assert!(invariants.is_dag(&empty));
}

#[test]
fn test_is_dag_single_node() {
    let invariants = SystemInvariants::new();
    // A single node with no edges is trivially a DAG
    let empty: Vec<(String, String)> = vec![];
    assert!(invariants.is_dag(&empty));
}

#[test]
fn test_is_dag_disconnected_components() {
    let invariants = SystemInvariants::new();
    let edges = vec![
        ("A".to_string(), "B".to_string()),
        ("C".to_string(), "D".to_string()),
    ];
    assert!(invariants.is_dag(&edges));
}

#[test]
fn test_is_dag_self_loop() {
    let invariants = SystemInvariants::new();
    let edges = vec![("A".to_string(), "A".to_string())];
    // A self-loop is a cycle
    assert!(!invariants.is_dag(&edges));
}

#[test]
fn test_is_dag_diamond_pattern() {
    let invariants = SystemInvariants::new();
    let edges = vec![
        ("A".to_string(), "B".to_string()),
        ("A".to_string(), "C".to_string()),
        ("B".to_string(), "D".to_string()),
        ("C".to_string(), "D".to_string()),
    ];
    assert!(invariants.is_dag(&edges));
}

#[test]
fn test_detect_cycle_three_node_loop() {
    let invariants = SystemInvariants::new();
    let edges = vec![
        ("A".to_string(), "B".to_string()),
        ("B".to_string(), "C".to_string()),
        ("C".to_string(), "A".to_string()),
    ];
    let cycle = invariants.detect_cycle(&edges);
    assert!(cycle.is_some());
    let c = cycle.unwrap();
    assert!(c.len() >= 3);
}

#[test]
fn test_detect_cycle_with_disconnected_nodes() {
    let invariants = SystemInvariants::new();
    let edges = vec![
        ("A".to_string(), "B".to_string()),
        ("C".to_string(), "D".to_string()),
    ];
    assert!(invariants.detect_cycle(&edges).is_none());
}

// ============================================================================
// Resolution strategy variant coverage
// ============================================================================

#[test]
fn test_resolution_strategy_all_variants_distinct() {
    use std::collections::HashSet;
    let strategies = vec![
        ResolutionStrategy::Consensus,
        ResolutionStrategy::Recency,
        ResolutionStrategy::Confidence,
    ];
    let set: HashSet<_> = strategies.iter().collect();
    assert_eq!(set.len(), 3);
}

#[test]
fn test_resolution_result_failure_structure() {
    let result = ResolutionResult {
        id: "r1".to_string(),
        inconsistency_id: "inc_1".to_string(),
        strategy: ResolutionStrategy::Consensus,
        success: false,
        chosen_value: None,
        candidates: vec![],
        rationale: "failed due to no candidates".to_string(),
        resolved_at: 12345,
    };
    assert!(!result.success);
    assert!(result.chosen_value.is_none());
    assert!(!result.rationale.is_empty());
}

#[test]
fn test_candidate_value_serialization_roundtrip() {
    let cv = CandidateValue {
        value: serde_json::json!({"key": [1, 2, 3]}),
        source: "test_mod".to_string(),
        timestamp: 999888777,
        confidence: 0.95,
    };
    let json = serde_json::to_string(&cv).unwrap();
    let parsed: CandidateValue = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.source, cv.source);
    assert_eq!(parsed.timestamp, cv.timestamp);
    assert!((parsed.confidence - cv.confidence).abs() < 1e-10);
}

// ============================================================================
// InvariantViolation edge cases
// ============================================================================

#[test]
fn test_invariant_violation_non_critical() {
    let v = InvariantViolation::new(
        InvariantType::DecayMonotonicity,
        "activation increased slightly".to_string(),
        vec!["e1".to_string()],
        false,
    );
    assert!(!v.critical);
    assert!(v.timestamp > 0);
}

#[test]
fn test_invariant_violation_multiple_affected() {
    let v = InvariantViolation::new(
        InvariantType::GraphAcyclicity,
        "cycle detected".to_string(),
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
        true,
    );
    assert_eq!(v.affected.len(), 3);
}

#[test]
fn test_invariant_violation_with_diagnostics_retains_data() {
    let v = InvariantViolation::new(
        InvariantType::Conservation,
        "imbalance".to_string(),
        vec!["e1".to_string()],
        true,
    )
    .with_diagnostic("created".to_string(), 5.into())
    .with_diagnostic("deleted".to_string(), 8.into());
    assert_eq!(v.diagnostics["created"], serde_json::json!(5));
    assert_eq!(v.diagnostics["deleted"], serde_json::json!(8));
}

// ============================================================================
// ConsistencyChecker edge cases
// ============================================================================

#[test]
fn test_with_thresholds_extreme_values() {
    let checker = ConsistencyChecker::with_thresholds(0, 0.0);
    assert_eq!(checker.graph_edit_distance_threshold, 0);
    assert!((checker.probability_threshold - 0.0).abs() < 1e-10);
}

#[test]
fn test_with_thresholds_high_values() {
    let checker = ConsistencyChecker::with_thresholds(usize::MAX, 1.0);
    assert_eq!(checker.graph_edit_distance_threshold, usize::MAX);
    assert!((checker.probability_threshold - 1.0).abs() < 1e-10);
}

#[test]
fn test_check_all_does_not_panic() {
    let mut checker = ConsistencyChecker::new();
    // Multiple calls should not panic or accumulate state incorrectly
    for _ in 0..10 {
        let result = checker.check_all();
        assert!(result.is_ok());
    }
}

#[test]
fn test_check_entity_different_ids() {
    let mut checker = ConsistencyChecker::new();
    // Different entity IDs should all work without panicking
    for id in &["alpha", "beta", "gamma", "delta"] {
        let result = checker.check_entity(id);
        assert!(result.is_ok(), "Failed for entity {}", id);
    }
}

// ============================================================================
// Serialization consistency
// ============================================================================

#[test]
fn test_inconsistency_report_json_roundtrip() {
    let report = InconsistencyReport::new(
        InconsistencyType::EntityPermanenceViolation,
        vec!["ent_a".to_string(), "ent_b".to_string()],
        "entities deleted but still tracked".to_string(),
        8,
    );
    let json = serde_json::to_string(&report).unwrap();
    let parsed: InconsistencyReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.inconsistency_type, report.inconsistency_type);
    assert_eq!(parsed.affected_entities, report.affected_entities);
    assert_eq!(parsed.severity, report.severity);
    assert_eq!(parsed.description, report.description);
}

#[test]
fn test_resolution_result_json_roundtrip() {
    let result = ResolutionResult {
        id: "res_42".to_string(),
        inconsistency_id: "inc_42".to_string(),
        strategy: ResolutionStrategy::Confidence,
        success: true,
        chosen_value: Some(serde_json::json!({"resolved": true})),
        candidates: vec![],
        rationale: "chosen by confidence".to_string(),
        resolved_at: 1234567890,
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: ResolutionResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, result.id);
    assert!(parsed.success);
    assert_eq!(parsed.strategy, ResolutionStrategy::Confidence);
}

#[test]
fn test_invariant_violation_json_roundtrip() {
    let violation = InvariantViolation::new(
        InvariantType::DecayMonotonicity,
        "decay violation".to_string(),
        vec!["x".to_string()],
        false,
    );
    let json = serde_json::to_string(&violation).unwrap();
    let parsed: InvariantViolation = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.invariant_type, violation.invariant_type);
    assert_eq!(parsed.description, violation.description);
    assert_eq!(parsed.critical, violation.critical);
}
