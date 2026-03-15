## ADDED Requirements

### Requirement: Intelligence Health Endpoints
The Integration Layer SHALL expose HTTP endpoints for querying Self-Model, World-Model, and Coherence health.

#### Scenario: Self-Model health endpoint
- **WHEN** client makes GET /health/self-model request
- **THEN** the Integration Layer SHALL return JSON with overall health score, module breakdown, capability status, and resource usage

#### Scenario: World-Model health endpoint
- **WHEN** client makes GET /health/world-model request
- **THEN** the Integration Layer SHALL return JSON with entity count, prediction accuracy, uncertainty metrics, and causal graph stats

#### Scenario: Coherence health endpoint
- **WHEN** client makes GET /health/coherence request
- **THEN** the Integration Layer SHALL return JSON with coherence score, active inconsistencies, last check time, and resolution metrics

### Requirement: Intelligence Prediction Endpoints
The Integration Layer SHALL expose endpoints for using intelligence capabilities from external clients.

#### Scenario: Predict next state endpoint
- **WHEN** client makes POST /predict/next-state with current state and action
- **THEN** the Integration Layer SHALL delegate to world_model.predict() and return prediction with confidence and uncertainty

#### Scenario: Counterfactual query endpoint
- **WHEN** client makes POST /query/counterfactual with intervention
- **THEN** the Integration Layer SHALL delegate to world_model.counterfactual() and return alternate outcome distribution

#### Scenario: Entity prediction endpoint
- **WHEN** client makes GET /predict/entity/{id}?steps=N
- **THEN** the Integration Layer SHALL use world_model.entity_tracker.predict(steps) and return future entity states

### Requirement: Self-Model Decision API
The Integration Layer SHALL provide unified API for operation decision-making using Self-Model.

#### Scenario: Can-execute query
- **WHEN** client makes POST /decide/can-execute with operation name and context
- **THEN** the Integration Layer SHALL call self_model.should_execute() and return decision with rationale, confidence, and expected resources

#### Scenario: Bulk decision query
- **WHEN** client makes POST /decide/batch with multiple operations
- **THEN** the Integration Layer SHALL evaluate all operations in batch and return decisions with priorities

### Requirement: Coherence Control API
The Integration Layer SHALL provide endpoints for forcing coherence checks and retrieving inconsistency reports.

#### Scenario: Force coherence check endpoint
- **WHEN** client makes POST /coherence/check request
- **THEN** the Integration Layer SHALL trigger immediate full coherence validation and return results

#### Scenario: List inconsistencies endpoint
- **WHEN** client makes GET /coherence/inconsistencies request
- **THEN** the Integration Layer SHALL retrieve active inconsistencies from coherence_checker and return with details

#### Scenario: Manual resolution endpoint
- **WHEN** client makes POST /coherence/resolve/{id} with resolution value
- **THEN** the Integration Layer SHALL apply manual resolution via coherence_checker and return confirmation

### Requirement: Unified Monitoring Dashboard Data
The Integration Layer SHALL aggregate intelligence metrics for monitoring dashboards.

#### Scenario: Metrics aggregation
- **WHEN** monitoring system queries metrics
- **THEN** the Integration Layer SHALL collect metrics from Self-Model, World-Model, and Coherence and return unified JSON

#### Scenario: Health summary endpoint
- **WHEN** client makes GET /health/summary request
- **THEN** the Integration Layer SHALL return single health score aggregating all intelligence components with breakdown
