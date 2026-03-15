## ADDED Requirements

### Requirement: Self-Model Health Checks
The Aureus Bridge SHALL verify system health before initiating reflexion loops to prevent overtaxing resources.

#### Scenario: Health check before reflexion
- **WHEN** reflexion loop is scheduled to run
- **THEN** the Aureus Bridge SHALL query self_model.get_health() and proceed only if health >0.5

#### Scenario: Adaptive reflexion frequency
- **WHEN** Self-Model indicates high resource usage
- **THEN** the Aureus Bridge SHALL reduce reflexion frequency to conserve resources

### Requirement: World-Model Counterfactual Reasoning
The Aureus Bridge SHALL use World-Model for counterfactual reasoning during reflexion to improve belief updates.

#### Scenario: Counterfactual query in reflexion
- **WHEN** reflexion loop analyzes past actions
- **THEN** the Aureus Bridge SHALL query world_model.counterfactual("what if action X?") to compare outcomes

#### Scenario: Belief update with predictions
- **WHEN** updating beliefs with Bayesian inference
- **THEN** the Aureus Bridge SHALL incorporate world_model.predict() as prior for more informed belief updates

### Requirement: Coherence in Belief Systems
The Aureus Bridge SHALL ensure belief updates maintain coherence with symbolic knowledge.

#### Scenario: Belief-symbolic consistency
- **WHEN** reflexion updates beliefs
- **THEN** the Aureus Bridge SHALL verify new beliefs don't contradict symbolic_store facts via coherence_checker

#### Scenario: Contradictory belief detection
- **WHEN** belief update creates contradiction
- **THEN** the Aureus Bridge SHALL invoke coherence_checker.resolve() to reconcile beliefs with facts
