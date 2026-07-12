# HipCortex Core Execution Flows (Step-by-Step Traces)

This document maps concrete step-by-step Rust code execution traces across all **5 foundational multi-hop execution workflows** in HipCortex.

---

## Flow 1: Multimodal Ingestion & Merkle Audit Sealing

Traces external text/agent payloads being decorrelated, safety-audited, appended to canonical storage, and cryptographically linked into `audit.log`.

### Step 1: Ingestion & Schema Validation (`IntegrationLayer`)
```rust
// In src/modules/integration_layer.rs:
impl InputPayload {
    fn valid(&self) -> bool {
        !self.text.trim().is_empty() && self.text.len() <= 512
    }
}
```

### Step 2: Normalization & PCA Decorrelation (`PerceptionAdapter`)
```rust
// In src/modules/perception_adapter.rs:
pub fn encode(&mut self, input: PerceptInput) -> Result<Vec<f32>, AdapterError> {
    self.rate_limiter.check_capacity()?;
    let raw_vector = self.extract_features(&input);
    let decorrelated = self.pca_transform.fit_transform(&raw_vector);
    Ok(decorrelated.as_slice().to_vec()) // COMPRESS_DIM = 4
}
```

### Step 3: Pre-Mutation Content Safety Audit (`SafetyGuardrail`)
```rust
// In src/safety_guardrail.rs:
pub fn check_precondition(&mut self, op_context: &str) -> Result<(), String> {
    let classification = self.classifier.classify(op_context);
    if matches!(classification.recommended_action, Action::Block) {
        let err = format!("Blocked risk={:.2}", classification.risk_score);
        self.log_violation(op_context, &err);
        return Err(err); // Halt execution immediately
    }
    Ok(())
}
```

### Step 4 & 5: Persistence & Cryptographic Merkle Link (`MemoryStore`)
```rust
// In src/memory_store.rs:
pub fn append(&mut self, mut record: MemoryRecord) -> Result<Uuid> {
    self.guardrail.check_precondition(&record.content)?;
    record.prev_hash = self.get_latest_merkle_root();
    record.integrity_hash = record.compute_sha256();
    self.backend.put(&record.id, &record.serialize())?;
    self.audit_log.append_merkle_entry(&record)?;
    Ok(record.id)
}
```

---

## Flow 2: Autonomous Metacognitive Reflexion Loop

Traces how accumulating episodic memory triggers self-awareness, expected utility evaluation, resource forecasting, and chain-of-thought generation.

### Step 1: Episodic Accumulation Observation (`AureusBridge`)
```rust
// In src/modules/aureus_bridge.rs:
pub fn observe_memory_delta(&mut self, delta: &MemoryDelta) {
    if delta.relevance_sum >= self.config.trigger_threshold {
        self.loop_engine.request_cycle("high_relevance_accumulation");
    }
}
```

### Step 2: Expected Utility Decision Evaluation (`SelfModel`)
```rust
// In src/modules/self_model/mod.rs:
let decision = decision_engine.evaluate(
    "reflexion_cycle",
    DecisionContext { priority: 0.8, user_facing: false, .. },
    historical_tracker.get_success_rate("reflexion_cycle"),
    forecasted_resources,
    health_aggregator.get_overall_health()?.overall, // ∈ [0, 1]
);
if !decision.should_execute { return; }
```

### Step 3 & 4: Generation Dispatch & Pruning (`LoopEngine`)
```rust
// In src/modules/loop_engine.rs:
pub fn execute_cycle(&mut self, client: &mut dyn LLMClient) -> Result<ReflexionHypothesis> {
    let prompt = self.context_builder.build_reflexion_prompt();
    let raw = client.generate(&prompt)?;
    let hypothesis = self.parse_hypothesis(&raw)?;
    if hypothesis.confidence < self.config.prune_threshold {
        self.hypotheses_graph.prune(&hypothesis.id);
    }
    Ok(hypothesis)
}
```

---

## Flow 3: Cross-Module Coherence & Invariant Verification

Traces background consistency sweeps, conflict identification across heterogeneous stores, automated patch resolution, and DAG invariant enforcement.

### Step 1 & 2: Background Consistency Check (`CoherenceChecker`)
```rust
// In src/modules/coherence/mod.rs:
pub fn check_consistency(&self) -> Result<Vec<InconsistencyReport>> {
    let mut reports = Vec::new();
    for entity in self.symbolic_store.all_nodes() {
        let temp_count = self.temporal_indexer.count_references(&entity.id);
        let sym_count = self.symbolic_store.count_edges(&entity.id);
        if temp_count != sym_count {
            reports.push(InconsistencyReport {
                inconsistency_type: InconsistencyType::TemporalSymbolicConflict,
                severity: Severity::High,
                description: format!("Discrepancy for entity {}", entity.id),
            });
        }
    }
    Ok(reports)
}
```

### Step 3 & 4: Automated Patch Resolution & Acyclicity Enforcement
```rust
// In src/modules/coherence/mod.rs:
pub fn resolve_all(&mut self, strategy: ResolutionStrategy) -> Result<Vec<ResolutionResult>> {
    for report in self.check_consistency()? {
        match strategy {
            ResolutionStrategy::Consensus => self.apply_consensus_patch(&report),
            ResolutionStrategy::Recency => self.overwrite_with_latest_timestamp(&report),
            _ => self.flag_for_manual_intervention(&report),
        }
    }
    self.system_invariants.assert_dags_acyclic()?;
    Ok(results)
}
```

---

## Flow 4: World-Model Dynamics Learning & Kalman Forecasting

Traces transition observation recording, Dirichlet prior updating, linear Kalman filter entity tracking, and multi-step covariance projection.

### Step 1 & 2: Transition Dynamics Update (`TransitionModel`)
```rust
// In src/modules/world_model_enhanced/mod.rs:
pub fn record_transition(&mut self, trans: StateTransition) -> Result<()> {
    let counts = self.transition_counts.entry((trans.from_state, trans.action)).or_default();
    *counts.entry(trans.to_state).or_insert(0.0) += 1.0;
    self.recompute_probabilities(); // Enforces sum(P) == 1.0
    Ok(())
}
```

### Step 3 & 4: Kalman Entity Tracking & Projection (`EntityTracker`)
```rust
// In src/modules/world_model_enhanced/mod.rs:
pub fn update_entity(&mut self, id: &str, obs: EntityObservation) -> Result<()> {
    let entity = self.entities.get_mut(id).ok_or("entity not found")?;
    let gain = entity.covariance * &(entity.covariance + obs.measurement_noise).try_inverse()?;
    entity.properties += gain * &(obs.measured_properties - &entity.properties);
    entity.covariance = (DMatrix::identity() - gain) * entity.covariance; // Remains PSD
    Ok(())
}
```

---

## Flow 5: Batched Procedural FSM Advancement & Emergency Rollback

Traces finite state machine batch advancement, transition table verification, metacognitive health monitoring, and atomic snapshot restore.

### Step 1 & 2: Bulk FSM Advancement (`ProceduralCache`)
```rust
// In src/modules/procedural_cache.rs:
pub fn advance_batch(&mut self, trace_ids: &[Uuid], condition: Option<&str>) -> Vec<Option<FSMState>> {
    trace_ids.iter().map(|id| {
        let trace = self.traces.get_mut(id)?;
        let next = self.transition_table.get(&(trace.current_state.clone(), condition?))?;
        trace.current_state = next.clone();
        Some(next.clone())
    }).collect()
}
```

### Step 3 & 4: Degradation Detection & Atomic Restore (`SnapshotManager`)
```rust
// In src/snapshot_manager.rs:
pub fn monitor_and_rollback_if_corrupted(&mut self, health: &HealthAggregator) -> Result<()> {
    if health.get_overall_health()?.overall < CRITICAL_HALT_THRESHOLD {
        let checkpoint = self.get_latest_checkpoint()?;
        self.memory_store.restore_from_snapshot(&checkpoint)?;
    }
    Ok(())
}
```
