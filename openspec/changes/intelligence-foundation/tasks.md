## 1. Self-Model Module Implementation

- [x] 1.1 Create src/modules/self_model/mod.rs with SelfModel struct, initialization, and module coordinator
- [x] 1.2 Implement src/modules/self_model/capability.rs with CapabilityRegistry, CapabilityDescriptor, and Limitation types
- [x] 1.3 Implement src/modules/self_model/resource.rs with ResourceMonitor and linear regression prediction
- [x] 1.4 Implement src/modules/self_model/performance.rs with PerformanceTracker, EWMA averaging, and Bayesian estimation
- [x] 1.5 Implement src/modules/self_model/health.rs with HealthAggregator and weighted geometric mean calculation
- [x] 1.6 Implement src/modules/self_model/decision.rs with DecisionEngine and expected utility maximization
- [x] 1.7 Add Self-Model module to src/lib.rs exports and re-export public types

## 2. Self-Model Unit Tests

- [x] 2.1 Write unit tests for CapabilityRegistry (register, query, update, errors)
- [x] 2.2 Write unit tests for ResourceMonitor (tracking, prediction accuracy R²>0.7)
- [x] 2.3 Write unit tests for PerformanceTracker (EWMA, Bayesian estimation, confidence intervals)
- [x] 2.4 Write unit tests for HealthAggregator (weighted mean, degradation detection)
- [x] 2.5 Write unit tests for DecisionEngine (expected utility, approval/rejection logic)
- [x] 2.6 Write property-based tests for Self-Model invariants (health ∈ [0,1], monotonic resource usage)

## 3. World-Model Enhanced Module Implementation

- [x] 3.1 Create src/modules/world_model_enhanced/mod.rs with WorldModel struct and central coordinator
- [x] 3.2 Implement src/modules/world_model_enhanced/transition.rs with TransitionModel and Dirichlet-Multinomial learning
- [x] 3.3 Implement src/modules/world_model_enhanced/entity.rs with EntityTracker and Kalman filtering
- [x] 3.4 Implement src/modules/world_model_enhanced/causal.rs with CausalGraph and do-calculus
- [x] 3.5 Implement src/modules/world_model_enhanced/predictor.rs with PredictiveModel trait and LearnedTransitionPredictor
- [x] 3.6 Implement src/modules/world_model_enhanced/uncertainty.rs with UncertaintyEstimator and confidence intervals
- [x] 3.7 Add World-Model module to src/lib.rs exports

## 4. World-Model Unit Tests

- [x] 4.1 Write unit tests for TransitionModel (observation recording, prediction, entropy calculation)
- [x] 4.2 Write unit tests for EntityTracker (Kalman predict/update, anomaly detection, entity permanence)
- [x] 4.3 Write unit tests for CausalGraph (add node/edge, path queries, cycle prevention, do-calculus)
- [x] 4.4 Write unit tests for PredictiveModel (training, multi-step prediction, ensemble averaging)
- [x] 4.5 Write unit tests for UncertaintyEstimator (confidence intervals, epistemic/aleatoric decomposition, ECE<0.1)
- [x] 4.6 Write property-based tests for World-Model invariants (probability sums to 1.0, acyclicity, calibration)

## 5. Coherence Checker Module Implementation

- [x] 5.1 Create src/modules/coherence/mod.rs with CoherenceChecker struct and main API
- [x] 5.2 Implement src/modules/coherence/checker.rs with ConsistencyChecker and inconsistency detection
- [x] 5.3 Implement src/modules/coherence/resolver.rs with ConflictResolver and resolution strategies
- [x] 5.4 Implement src/modules/coherence/invariants.rs with SystemInvariants and enforcement
- [x] 5.5 Add Coherence module to src/lib.rs exports

## 6. Coherence Checker Unit Tests

- [ ] 6.1 Write unit tests for inconsistency detection (temporal-symbolic, procedural-world, causal, entity permanence)
- [ ] 6.2 Write unit tests for ConflictResolver (consensus, recency, confidence strategies, rollback)
- [ ] 6.3 Write unit tests for SystemInvariants (memory consistency, decay monotonicity, graph acyclicity, conservation)
- [ ] 6.4 Write property-based tests for Coherence invariants (no invalid states reachable, resolution deterministic)

## 7. Temporal Indexer Integration

- [ ] 7.1 Add HealthReporter trait implementation to TemporalIndexer
- [ ] 7.2 Add self_model.can_execute() check before insert operations
- [ ] 7.3 Add resource usage reporting to Self-Model after operations
- [ ] 7.4 Add world_model.observe_transition() calls after trace insertion
- [ ] 7.5 Add coherence_checker.validate_entity() calls after insertions
- [ ] 7.6 Write integration tests for Temporal Indexer intelligence hooks

## 8. Symbolic Store Integration

- [ ] 8.1 Add HealthReporter trait implementation to SymbolicStore
- [ ] 8.2 Add self_model.check_resources() before graph operations
- [ ] 8.3 Add world_model.register_entity() and update_entity() on graph changes
- [ ] 8.4 Add coherence_checker.is_operation_valid() before modifications
- [ ] 8.5 Add entity deletion validation via Coherence Checker
- [ ] 8.6 Write integration tests for Symbolic Store intelligence hooks

## 9. Procedural Cache Integration

- [ ] 9.1 Add HealthReporter trait implementation to ProceduralCache
- [ ] 9.2 Add self_model.can_execute() check before FSM transitions
- [ ] 9.3 Add health-based transition filtering (block risky transitions if health <0.5)
- [ ] 9.4 Add world_model.observe_transition() for FSM state changes
- [ ] 9.5 Add coherence pre-transition checks for procedural-world consistency
- [ ] 9.6 Write integration tests for Procedural Cache intelligence hooks

## 10. Aureus Bridge Integration

- [ ] 10.1 Add self_model.get_health() check before reflexion loops
- [ ] 10.2 Implement adaptive reflexion frequency based on resource usage
- [ ] 10.3 Add world_model.counterfactual() queries during reflexion
- [ ] 10.4 Incorporate world_model.predict() as prior in Bayesian belief updates
- [ ] 10.5 Add coherence_checker validation for belief-symbolic consistency
- [ ] 10.6 Write integration tests for Aureus Bridge intelligence integration

## 11. Perception Adapter Integration

- [ ] 11.1 Add self_model.check_resources() before PCA operations
- [ ] 11.2 Implement graceful degradation (skip PCA if resources insufficient)
- [ ] 11.3 Add world_model.update_entity() with perception observations
- [ ] 11.4 Add anomaly handling when World-Model detects perception anomalies
- [ ] 11.5 Add coherence_checker validation for perception consistency
- [ ] 11.6 Write integration tests for Perception Adapter intelligence hooks

## 12. Integration Layer API Extensions

- [ ] 12.1 Implement GET /health/self-model endpoint
- [ ] 12.2 Implement GET /health/world-model endpoint
- [ ] 12.3 Implement GET /health/coherence endpoint
- [ ] 12.4 Implement POST /predict/next-state endpoint
- [ ] 12.5 Implement POST /query/counterfactual endpoint
- [ ] 12.6 Implement GET /predict/entity/{id} endpoint
- [ ] 12.7 Implement POST /decide/can-execute endpoint
- [ ] 12.8 Implement POST /decide/batch endpoint
- [ ] 12.9 Implement POST /coherence/check endpoint
- [ ] 12.10 Implement GET /coherence/inconsistencies endpoint
- [ ] 12.11 Implement POST /coherence/resolve/{id} endpoint
- [ ] 12.12 Implement GET /health/summary endpoint
- [ ] 12.13 Write integration tests for all new API endpoints

## 13. System Integration Tests (SIT)

- [ ] 13.1 Write SIT for Self-Model + Temporal Indexer (operation gating, health reporting)
- [ ] 13.2 Write SIT for Self-Model + Symbolic Store (resource checking, performance tracking)
- [ ] 13.3 Write SIT for Self-Model + Procedural Cache (decision gating, adaptive behavior)
- [ ] 13.4 Write SIT for World-Model + Temporal Indexer (observation feeding, state tracking)
- [ ] 13.5 Write SIT for World-Model + Symbolic Store (entity sync, causal graph building)
- [ ] 13.6 Write SIT for World-Model + Procedural Cache (prediction validation, transition learning)
- [ ] 13.7 Write SIT for Coherence + all modules (inconsistency detection, auto-resolution)
- [ ] 13.8 Write SIT for end-to-end intelligence workflow (trace insert → prediction → coherence check)

## 14. User Acceptance Tests (UAT)

- [ ] 14.1 Write UAT for Self-Model: Developer monitors system health via /health/self-model endpoint
- [ ] 14.2 Write UAT for Self-Model: System rejects operation under high load with clear rationale
- [ ] 14.3 Write UAT for Self-Model: Performance learning improves predictions over time
- [ ] 14.4 Write UAT for World-Model: Accurate state prediction (>70% accuracy on test scenarios)
- [ ] 14.5 Write UAT for World-Model: Counterfactual reasoning answers "what if" questions
- [ ] 14.6 Write UAT for World-Model: Entity tracking maintains object permanence across observations
- [ ] 14.7 Write UAT for Coherence: Inconsistencies detected between temporal and symbolic memory
- [ ] 14.8 Write UAT for Coherence: Automatic conflict resolution succeeds (>80% success rate)
- [ ] 14.9 Write UAT for Coherence: Invariant violations halt operations and trigger alerts
- [ ] 14.10 Write UAT for Integration: All modules report health correctly
- [ ] 14.11 Write UAT for End-to-End: Complete cognitive workflow (perception → decision → action → learning → coherence)

## 15. Property-Based Tests (Invariant Validation)

- [ ] 15.1 Write proptest for Self-Model: health score always ∈ [0,1]
- [ ] 15.2 Write proptest for Self-Model: resource prediction error decreases with more data
- [ ] 15.3 Write proptest for World-Model: transition probabilities sum to 1.0 for all state-action pairs
- [ ] 15.4 Write proptest for World-Model: causal graph remains acyclic under arbitrary edge additions
- [ ] 15.5 Write proptest for World-Model: uncertainty never negative, decreases with more observations
- [ ] 15.6 Write proptest for Coherence: entity count conserved across all modules
- [ ] 15.7 Write proptest for Coherence: decay scores monotonically decrease over time
- [ ] 15.8 Write proptest for Coherence: no inconsistency remains after successful resolution

## 16. Performance Benchmarking

- [ ] 16.1 Benchmark Self-Model operation: can_execute() latency (target <2ms)
- [ ] 16.2 Benchmark Self-Model: health aggregation latency (target <5ms)
- [ ] 16.3 Benchmark World-Model: state prediction latency (target <10ms)
- [ ] 16.4 Benchmark World-Model: entity tracking update latency (target <5ms)
- [ ] 16.5 Benchmark Coherence: full consistency check latency (target <100ms for 1000 entities)
- [ ] 16.6 Benchmark integration overhead: trace insertion with intelligence vs without (target <10ms delta)
- [ ] 16.7 Benchmark memory usage: measure intelligence module memory footprint (target <100MB)
- [ ] 16.8 Benchmark throughput: operations/sec with intelligence enabled (target >1000 ops/sec)

## 17. Documentation

- [ ] 17.1 Write API documentation for Self-Model module (rustdoc with examples)
- [ ] 17.2 Write API documentation for World-Model module (rustdoc with examples)
- [ ] 17.3 Write API documentation for Coherence module (rustdoc with examples)
- [ ] 17.4 Write architecture guide explaining intelligence layer design
- [ ] 17.5 Write integration guide for module authors (how to add intelligence hooks)
- [ ] 17.6 Write examples: self_model_quickstart.rs
- [ ] 17.7 Write examples: world_model_prediction.rs
- [ ] 17.8 Write examples: coherence_checking.rs
- [ ] 17.9 Update README.md with intelligence features section
- [ ] 17.10 Create INTELLIGENCE.md with detailed capabilities and usage

## 18. Testing Infrastructure

- [ ] 18.1 Set up criterion.rs for performance benchmarks
- [ ] 18.2 Set up proptest for property-based tests
- [ ] 18.3 Create test fixtures for common scenarios (sample traces, entities, states)
- [ ] 18.4 Create test helpers for mocking Self-Model, World-Model, Coherence
- [ ] 18.5 Set up CI pipeline to run all test categories (unit, SIT, UAT, property, benchmarks)

## 19. Validation & Quality Assurance

- [ ] 19.1 Run all 600+ unit tests and verify >80% code coverage
- [ ] 19.2 Run all 300+ integration tests (SIT) and verify cross-module interactions work
- [ ] 19.3 Run all 200+ property-based tests and verify no invariant violations
- [ ] 19.4 Run all 11 UAT scenarios and verify end-user workflows succeed
- [ ] 19.5 Run performance benchmarks and verify all targets met (<10ms overhead, <100MB memory)
- [ ] 19.6 Validate prediction accuracy: >70% for next state prediction
- [ ] 19.7 Validate uncertainty calibration: ECE <0.1
- [ ] 19.8 Validate coherence detection: >95% of injected inconsistencies detected
- [ ] 19.9 Validate auto-resolution: >80% success rate, <5% false positives
- [ ] 19.10 Verify all 147+ existing tests still pass (no regressions)

## 20. Pre-Production Readiness

- [ ] 20.1 Code review: Self-Model module reviewed and approved
- [ ] 20.2 Code review: World-Model module reviewed and approved
- [ ] 20.3 Code review: Coherence module reviewed and approved
- [ ] 20.4 Code review: All integration hooks reviewed and approved
- [ ] 20.5 Security review: No vulnerabilities introduced
- [ ] 20.6 Memory profiling with valgrind/massif (verify no leaks, <100MB footprint)
- [ ] 20.7 Load testing: system stable under 10x normal load
- [ ] 20.8 Documentation review: all docs accurate and complete
- [ ] 20.9 Create deployment checklist and rollback plan
- [ ] 20.10 Prepare monitoring dashboards for production metrics

## 21. Deployment - Internal Alpha (Week 11)

- [ ] 21.1 Deploy to development environment
- [ ] 21.2 Manual smoke testing of all intelligence features
- [ ] 21.3 Verify health endpoints return correct data
- [ ] 21.4 Verify prediction endpoints work with test data
- [ ] 21.5 Verify coherence checking detects test inconsistencies
- [ ] 21.6 Gather feedback from 3 core developers
- [ ] 21.7 Fix any P0 bugs discovered
- [ ] 21.8 Document alpha findings

## 22. Deployment - Internal Beta (Week 12)

- [ ] 22.1 Deploy to staging environment (production-like)
- [ ] 22.2 Run full UAT test suite in staging
- [ ] 22.3 Performance testing under simulated load
- [ ] 22.4 Monitor logs for unexpected behavior
- [ ] 22.5 Gather feedback from 10 extended team members
- [ ] 22.6 Fix any P1 bugs discovered
- [ ] 22.7 Validate metrics match dev environment
- [ ] 22.8 Document beta findings and NPS score

## 23. Deployment - Production Canary (Week 13)

- [ ] 23.1 Set feature flag to enable intelligence for 5% of traffic
- [ ] 23.2 Deploy to production with canary configuration
- [ ] 23.3 Monitor key metrics: latency p95, error rate, memory usage
- [ ] 23.4 Perform canary analysis comparing 5% (new) vs 95% (old)
- [ ] 23.5 Increase to 25% if metrics healthy after 24 hours
- [ ] 23.6 Increase to 50% if metrics healthy after 48 hours
- [ ] 23.7 Prepare rollback procedure in case of issues
- [ ] 23.8 Document canary results

## 24. Deployment - Full Rollout (Week 14)

- [ ] 24.1 Promote to 100% traffic after successful 50% canary
- [ ] 24.2 Monitor for 48 hours continuous stability
- [ ] 24.3 Verify all success metrics met (accuracy >70%, ECE <0.1, detection >95%)
- [ ] 24.4 Verify performance targets met (<10ms overhead, <100MB memory)
- [ ] 24.5 Verify no increase in error rate or latency
- [ ] 24.6 Declare production success
- [ ] 24.7 Publish announcement of intelligence capabilities
- [ ] 24.8 Archive change and document lessons learned
