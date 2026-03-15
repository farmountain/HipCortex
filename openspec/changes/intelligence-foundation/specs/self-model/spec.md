## ADDED Requirements

### Requirement: Capability Registry
The system SHALL maintain a registry of all available capabilities with their current status, limitations, and resource requirements.

#### Scenario: Register capability
- **WHEN** a module initializes and registers its capabilities
- **THEN** the Self-Model SHALL store the capability descriptor including name, required resources, expected performance, and current limitations

#### Scenario: Query capability
- **WHEN** code requests capability information for a specific operation
- **THEN** the Self-Model SHALL return the capability descriptor with current status and any active limitations

#### Scenario: Capability unavailable
- **WHEN** a capability is queried but not registered
- **THEN** the Self-Model SHALL return an error indicating the capability is unknown

### Requirement: Resource Monitoring
The system SHALL continuously monitor CPU usage, memory usage, disk I/O, and network I/O to track resource consumption patterns.

#### Scenario: Resource usage tracking
- **WHEN** the system executes operations
- **THEN** the Self-Model SHALL record resource consumption every 1 second with operation context

#### Scenario: Resource prediction
- **WHEN** code requests resource usage prediction for an operation
- **THEN** the Self-Model SHALL use linear regression on historical data to predict expected resource consumption with R² >0.7 accuracy target

#### Scenario: Resource exhaustion detection
- **WHEN** available resources fall below 10% of capacity
- **THEN** the Self-Model SHALL mark the system as resource-constrained and recommend rejecting non-critical operations

### Requirement: Performance Tracking
The system SHALL track operation latencies, success rates, and throughput to learn performance characteristics over time.

#### Scenario: Record operation outcome
- **WHEN** an operation completes
- **THEN** the Self-Model SHALL record duration, result (success/failure), and context using EWMA with α=0.1 for averaging

#### Scenario: Predict operation latency
- **WHEN** code requests latency prediction for an operation
- **THEN** the Self-Model SHALL return predicted latency using EWMA and confidence interval using historical variance

#### Scenario: Success rate estimation
- **WHEN** code requests success probability for an operation under current conditions
- **THEN** the Self-Model SHALL use Bayesian estimation (Beta distribution) to return success probability with credible interval

### Requirement: Health Aggregation
The system SHALL aggregate health signals from all subsystems into a single system health score and per-module health scores.

#### Scenario: Health monitoring
- **WHEN** subsystems report health metrics (latency, error rate, resource usage)
- **THEN** the Self-Model SHALL compute weighted geometric mean health score where 1.0 is perfect health and 0.0 is complete failure

#### Scenario: Module health degradation
- **WHEN** a module's health score falls below 0.5
- **THEN** the Self-Model SHALL mark that module as degraded and include it in system health warnings

#### Scenario: System health query
- **WHEN** code or monitoring requests system health
- **THEN** the Self-Model SHALL return overall health score and per-module breakdown with degradation details

### Requirement: Decision Engine
The system SHALL decide whether to execute operations based on capability availability, resource constraints, expected performance, and current system health.

#### Scenario: Operation approval
- **WHEN** code requests decision for an operation
- **THEN** the Self-Model SHALL evaluate capability availability, resource sufficiency, expected success rate, and system health to return approval decision with confidence score

#### Scenario: Operation rejection with rationale
- **WHEN** the Self-Model determines an operation should not execute
- **THEN** it SHALL return rejection with specific rationale (e.g., "insufficient memory: predicted 200MB needed, 150MB available")

#### Scenario: Expected utility calculation
- **WHEN** evaluating whether to execute an operation
- **THEN** the Self-Model SHALL compute expected utility = P(success) × value(success) - P(failure) × cost(failure) and approve if EU > threshold

#### Scenario: Adaptive learning
- **WHEN** actual operation outcomes differ from predictions
- **THEN** the Self-Model SHALL update its models (resource models, performance models, success rate models) to improve future predictions

### Requirement: Health Endpoints
The system SHALL expose HTTP endpoints for querying Self-Model state, health, and making capability checks.

#### Scenario: Query self-model health
- **WHEN** a client makes GET /health/self-model request
- **THEN** the system SHALL return JSON with overall health score, module health breakdown, and any active warnings

#### Scenario: Check operation capability
- **WHEN** a client makes POST /self-model/can-execute with operation name and context
- **THEN** the system SHALL return decision (approve/reject), confidence, expected resources, expected latency, and rationale

#### Scenario: Query resource predictions
- **WHEN** a client makes GET /self-model/resources/predict with operation name
- **THEN** the system SHALL return predicted CPU, memory, disk I/O, and network I/O with confidence intervals
