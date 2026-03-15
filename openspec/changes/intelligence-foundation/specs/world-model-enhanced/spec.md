## ADDED Requirements

### Requirement: State Transition Learning
The system SHALL learn probabilistic state transitions P(s'|s,a) from observations using Dirichlet-Multinomial conjugate prior with Laplace smoothing.

#### Scenario: Record state transition
- **WHEN** the system observes a state transition from state S to state S' via action A
- **THEN** the World-Model SHALL increment transition count for (S,A,S') and update Dirichlet posterior

#### Scenario: Predict next state
- **WHEN** code requests next state prediction given current state S and action A
- **THEN** the World-Model SHALL return probability distribution over possible next states using Dirichlet-Multinomial posterior

#### Scenario: Transition uncertainty
- **WHEN** a state-action pair has been observed fewer than 10 times
- **THEN** the World-Model SHALL return high uncertainty (entropy >2.0 bits) for that transition

#### Scenario: Entropy calculation
- **WHEN** code requests uncertainty for a state-action transition
- **THEN** the World-Model SHALL compute Shannon entropy H(P(s'|s,a)) of the predicted distribution

### Requirement: Entity Tracking
The system SHALL track entities over time with Kalman filtering to maintain beliefs about entity properties and predict future states.

#### Scenario: Register entity
- **WHEN** an entity appears in the system with initial properties
- **THEN** the World-Model SHALL create EntityTracker with Kalman filter (state = properties, process noise Q, measurement noise R)

#### Scenario: Update entity observation
- **WHEN** a new observation of an entity is received
- **THEN** the World-Model SHALL perform Kalman update step: update state estimate and covariance using measurement

#### Scenario: Predict entity state
- **WHEN** code requests entity state prediction N steps in the future
- **THEN** the World-Model SHALL perform N Kalman prediction steps returning predicted state and covariance

#### Scenario: Anomaly detection
- **WHEN** an entity observation deviates >3 standard deviations from predicted state
- **THEN** the World-Model SHALL flag an anomaly with details (property name, expected value, observed value, deviation)

#### Scenario: Entity permanence
- **WHEN** an entity has not been observed for >60 seconds
- **THEN** the World-Model SHALL continue tracking with prediction-only mode and mark entity as "unobserved"

### Requirement: Causal Graph
The system SHALL maintain a causal graph representing causal relationships between entities and support causal reasoning with do-calculus.

#### Scenario: Add causal relationship
- **WHEN** code declares that entity A causally influences entity B
- **THEN** the World-Model SHALL add directed edge A→B to the causal graph

#### Scenario: Query causal path
- **WHEN** code queries whether A causally affects C
- **THEN** the World-Model SHALL perform graph traversal and return true if path exists from A to C

#### Scenario: Causal intervention query
- **WHEN** code performs do-calculus intervention query P(Y|do(X=x))
- **THEN** the World-Model SHALL apply backdoor/frontdoor adjustment to compute intervention distribution removing confounding

#### Scenario: Counterfactual reasoning
- **WHEN** code asks "what if X had been x instead of x'?"
- **THEN** the World-Model SHALL compute counterfactual distribution using Pearl's counterfactual calculus

#### Scenario: Cyclic graph prevention
- **WHEN** code attempts to add edge that would create a cycle
- **THEN** the World-Model SHALL reject the edge addition and return error indicating cycle would be created

### Requirement: Predictive Models
The system SHALL support learned predictive models that can forecast future states based on current state and action sequences.

#### Scenario: Train predictive model
- **WHEN** the system has accumulated >100 state transition observations
- **THEN** the World-Model SHALL train LearnedTransitionPredictor using maximum likelihood estimation on transition history

#### Scenario: Multi-step prediction
- **WHEN** code requests prediction of state after action sequence [A1, A2, ..., An]
- **THEN** the World-Model SHALL apply predictive model recursively for each action returning final state distribution

#### Scenario: Ensemble prediction
- **WHEN** multiple predictive models are available
- **THEN** the World-Model SHALL combine predictions using ensemble averaging with weights proportional to past accuracy

#### Scenario: Model performance tracking
- **WHEN** predictions are made and actual outcomes observed
- **THEN** the World-Model SHALL track prediction accuracy (% correct for most likely state) and log quartile results

### Requirement: Uncertainty Quantification
The system SHALL provide calibrated uncertainty estimates for all predictions using confidence intervals and ensemble methods.

#### Scenario: Confidence interval
- **WHEN** code requests state prediction
- **THEN** the World-Model SHALL return point estimate and 95% confidence interval computed from prediction variance

#### Scenario: Epistemic vs aleatoric uncertainty
- **WHEN** code queries uncertainty decomposition
- **THEN** the World-Model SHALL separate epistemic uncertainty (model uncertainty) from aleatoric uncertainty (inherent randomness)

#### Scenario: Calibration checking
- **WHEN** >100 predictions have been made
- **THEN** the World-Model SHALL compute Expected Calibration Error (ECE) and target ECE <0.1 (well-calibrated)

#### Scenario: Uncertainty propagation
- **WHEN** making multi-step predictions
- **THEN** the World-Model SHALL propagate uncertainty through prediction chain using covariance propagation

### Requirement: World-Model Endpoints
The system SHALL expose HTTP endpoints for querying predictions, performing interventions, and accessing causal information.

#### Scenario: Predict next state endpoint
- **WHEN** a client makes POST /world-model/predict with current state and action
- **THEN** the system SHALL return predicted state distribution, most likely state, confidence, and uncertainty

#### Scenario: Counterfactual query endpoint
- **WHEN** a client makes POST /world-model/counterfactual with intervention specification
- **THEN** the system SHALL return counterfactual distribution and comparison to factual outcome

#### Scenario: Entity tracking endpoint
- **WHEN** a client makes GET /world-model/entity/{id}
- **THEN** the system SHALL return entity current state, predicted future states (1,5,10 steps), and anomaly history

#### Scenario: Causal graph query endpoint
- **WHEN** a client makes GET /world-model/causal-path with source and target entities
- **THEN** the system SHALL return whether causal path exists, path details if exists, and confounders if applicable
