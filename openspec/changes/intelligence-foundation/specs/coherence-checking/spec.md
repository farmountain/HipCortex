## ADDED Requirements

### Requirement: Inconsistency Detection
The system SHALL detect inconsistencies between temporal memory, symbolic memory, procedural memory, and world-model representations.

#### Scenario: Temporal-symbolic mismatch
- **WHEN** an event exists in temporal indexer but referenced entity missing from symbolic store
- **THEN** the Coherence Checker SHALL flag TemporalSymbolicMismatch inconsistency with event ID and missing entity ID

#### Scenario: Procedural-world conflict
- **WHEN** procedural FSM allows transition S→S' but world-model predicts P(S'|S,A) = 0
- **THEN** the Coherence Checker SHALL flag ProceduralWorldConflict with state, action, and probability discrepancy

#### Scenario: Causal violation
- **WHEN** observed event sequence violates causal constraints in causal graph
- **THEN** the Coherence Checker SHALL flag CausalViolation with event sequence and violated constraint

#### Scenario: Entity permanence violation
- **WHEN** entity exists in world-model but has been deleted from symbolic store
- **THEN** the Coherence Checker SHALL flag EntityPermanenceViolation with entity ID and timestamps

#### Scenario: Graph consistency
- **WHEN** relationship exists in symbolic DAG but contradicts edges in world-model causal graph
- **THEN** the Coherence Checker SHALL compute graph edit distance and flag if distance >threshold

### Requirement: Automatic Conflict Resolution
The system SHALL automatically resolve detected inconsistencies using configurable resolution strategies with safe rollback.

#### Scenario: Consensus resolution
- **WHEN** an inconsistency is detected with multiple conflicting values
- **THEN** the Coherence Checker SHALL apply consensus strategy selecting the value that appears in the majority of modules

#### Scenario: Recency resolution
- **WHEN** an inconsistency involves temporal ordering
- **THEN** the Coherence Checker SHALL apply recency strategy selecting the most recently observed value based on timestamps

#### Scenario: Confidence resolution
- **WHEN** modules provide confidence scores with their values
- **THEN** the Coherence Checker SHALL apply confidence strategy selecting the value with highest confidence score

#### Scenario: Resolution failure handling
- **WHEN** automatic resolution fails or creates new inconsistencies
- **THEN** the Coherence Checker SHALL rollback resolution attempt, log as P1 incident, and alert on-call engineer

#### Scenario: Resolution history
- **WHEN** a conflict is resolved
- **THEN** the Coherence Checker SHALL log resolution details (conflict type, strategy used, values considered, chosen value, timestamp) for audit

#### Scenario: Manual override capability
- **WHEN** an operator manually resolves a conflict
- **THEN** the Coherence Checker SHALL accept the override, disable automatic resolution for that specific conflict, and log the override

### Requirement: System Invariants
The system SHALL enforce mathematical invariants across modules and validate them continuously.

#### Scenario: Memory consistency invariant
- **WHEN** coherence check runs
- **THEN** the Coherence Checker SHALL verify that ∀ entity e, temporal_count(e) = symbolic_count(e) = world_model_count(e)

#### Scenario: Decay monotonicity invariant
- **WHEN** temporal decay is applied
- **THEN** the Coherence Checker SHALL verify that activation scores never increase: ∀ t1 < t2, activation(t2) ≤ activation(t1)

#### Scenario: Graph acyclicity invariant
- **WHEN** edges are added to symbolic or causal graphs
- **THEN** the Coherence Checker SHALL verify graphs remain acyclic using topological sort

#### Scenario: Conservation invariant
- **WHEN** entities are moved or transformed
- **THEN** the Coherence Checker SHALL verify that entity count is conserved: entities_created - entities_deleted = net_change

#### Scenario: Invariant violation handling
- **WHEN** an invariant is violated
- **THEN** the Coherence Checker SHALL immediately halt further operations, log critical error, and trigger system health degradation

### Requirement: Real-time Coherence Monitoring
The system SHALL continuously monitor coherence and provide real-time metrics on consistency state.

#### Scenario: Scheduled coherence check
- **WHEN** 60 seconds elapse since last coherence check
- **THEN** the Coherence Checker SHALL run full consistency validation across all modules asynchronously

#### Scenario: Operation-triggered check
- **WHEN** a critical operation completes (entity creation, deletion, state transition)
- **THEN** the Coherence Checker SHALL run targeted consistency check on affected entities

#### Scenario: Coherence metrics
- **WHEN** coherence check completes
- **THEN** the Coherence Checker SHALL update metrics: total_checks, inconsistencies_found, auto_resolutions_succeeded, auto_resolutions_failed, invariants_validated

#### Scenario: Coherence score calculation
- **WHEN** system requests coherence score
- **THEN** the Coherence Checker SHALL compute coherence_score = 1.0 - (active_inconsistencies / total_entities) where 1.0 is perfect coherence

### Requirement: Property-Based Validation
The system SHALL support property-based testing to validate that coherence properties hold under arbitrary inputs.

#### Scenario: Generate test cases
- **WHEN** property-based test runs
- **THEN** the Coherence Checker SHALL generate random operation sequences and verify coherence maintained throughout

#### Scenario: Invariant as property
- **WHEN** property test specifies invariant (e.g., "acyclicity")
- **THEN** the Coherence Checker SHALL verify invariant holds after every generated operation

#### Scenario: Shrinking on failure
- **WHEN** property test detects coherence violation
- **THEN** the testing framework SHALL shrink input to minimal failing case and report exact operation sequence causing violation

### Requirement: Coherence Endpoints
The system SHALL expose HTTP endpoints for querying coherence status, forcing checks, and retrieving inconsistency reports.

#### Scenario: Query coherence status
- **WHEN** a client makes GET /coherence/status request
- **THEN** the system SHALL return coherence score, active inconsistencies count, last check timestamp, and metrics

#### Scenario: Force coherence check
- **WHEN** a client makes POST /coherence/check request
- **THEN** the system SHALL immediately run full coherence validation and return results

#### Scenario: List inconsistencies
- **WHEN** a client makes GET /coherence/inconsistencies request
- **THEN** the system SHALL return list of active inconsistencies with type, affected entities, detection time, and resolution status

#### Scenario: Resolve inconsistency manually
- **WHEN** a client makes POST /coherence/resolve/{id} with resolution value
- **THEN** the system SHALL apply manual resolution, mark inconsistency as resolved, and disable automatic resolution for that conflict
