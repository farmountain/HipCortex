# self-model-sync Specification

## Purpose
Defines the architectural contract and synchronization requirements for mirroring high-frequency runtime metacognition (`SelfModel`) into persistent graph DB topology (`SymbolicStore`).

## Requirements

### Requirement: Canonical System Self Graph Node
The `SymbolicStore` SHALL maintain a canonical node with label `"System:Self"` representing the agent's aggregated metacognitive state.

#### Scenario: Agent queries long-term graph memory
- **WHEN** an active inference orchestrator (`McpBridge`) executes a topological search
- **THEN** the graph return includes node `"System:Self"` along with outgoing `HAS_CAPABILITY`, `EVALUATED_BY`, and `JUSTIFIED_BY` edges.

### Requirement: Hybrid Epoch Property Flush
The `SelfModel` SHALL periodically flush consolidated EWMA success rates and health scores to node `"System:Self"` on every perception adapt or checkpoint.

#### Scenario: Perception session completes adapt
- **WHEN** `PerceptionSession::adapt` successfully processes multimodal input
- **THEN** an asynchronous epoch event updates node `"System:Self"` properties without blocking real-time text ingestion p50 latency.

### Requirement: Guardrail Merkle Parity Enforcement
All epoch flushes to node `"System:Self"` SHALL invoke `SafetyGuardrail::check_precondition` before committing to storage.

#### Scenario: Malformed or unsafe metacognitive state detected
- **WHEN** `SafetyGuardrail` classifies an epoch flush payload with `Action::Block`
- **THEN** the graph mutation is aborted and a security violation is logged to `audit.log`.
