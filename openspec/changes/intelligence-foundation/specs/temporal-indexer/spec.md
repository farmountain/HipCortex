## ADDED Requirements

### Requirement: Self-Model Integration
The Temporal Indexer SHALL check with Self-Model before executing operations and report health metrics continuously.

#### Scenario: Capability check before insert
- **WHEN** code requests to insert a memory trace
- **THEN** the Temporal Indexer SHALL call self_model.can_execute("temporal_insert", context) before processing

#### Scenario: Operation rejection
- **WHEN** Self-Model rejects the insert operation
- **THEN** the Temporal Indexer SHALL return error with Self-Model's rationale without modifying memory

#### Scenario: Resource usage reporting
- **WHEN** an insert operation completes
- **THEN** the Temporal Indexer SHALL report actual resource usage (duration, memory delta) to Self-Model for learning

#### Scenario: Health reporting
- **WHEN** 60 seconds elapse or on-demand health check requested
- **THEN** the Temporal Indexer SHALL report health metrics (avg latency, error rate, index size) to Self-Model

### Requirement: World-Model Integration
The Temporal Indexer SHALL feed observations to World-Model to enable state tracking and prediction.

#### Scenario: Feed trace observation
- **WHEN** a memory trace is successfully inserted
- **THEN** the Temporal Indexer SHALL extract state information and send to world_model.observe_transition()

#### Scenario: Entity extraction
- **WHEN** a trace contains entity references
- **THEN** the Temporal Indexer SHALL extract entity IDs and properties, updating world_model.update_entity()

### Requirement: Coherence Integration
The Temporal Indexer SHALL trigger coherence checks after critical operations to maintain consistency.

#### Scenario: Coherence check after insert
- **WHEN** a memory trace insertion completes
- **THEN** the Temporal Indexer SHALL call coherence_checker.validate_entity(entity_ids) asynchronously

#### Scenario: Coherence violation handling
- **WHEN** Coherence Checker detects inconsistency involving temporal memory
- **THEN** the Temporal Indexer SHALL participate in resolution by providing temporal ordering information
