## MODIFIED Requirements

### Requirement: Multi-step WorldModel Rollout
The backend server SHALL expose a POST `/worldmodel/rollout` endpoint and internal simulation engine that performs multi-step recursive state prediction using Monte Carlo Tree Search (`MctsSimulator`) tree exploration over empirical Dirichlet transition counts (`TransitionModel`) and exact causal graph adjustments (`CausalGraph`).

#### Scenario: Successful multi-step rollout
- **WHEN** a POST request or simulation call is sent to `/worldmodel/rollout` with a valid initial state and list of actions or search depth
- **THEN** it executes UCB1 rollout iterations over Dirichlet transition probabilities and exact backdoor adjustments, returning the predicted best trajectory, final state, and ensemble confidence score

## ADDED Requirements

### Requirement: Empirical Backdoor Adjustment Causal Inference
The `CausalGraph` struct SHALL provide `compute_empirical_intervention(x, x_val, y, y_val, transitions)` to calculate interventional probabilities $P(Y=y \mid do(X=x))$ without relying on heuristic linear formulas. The engine SHALL identify parent variables $Z \in \text{Parents}(X)$ and marginalize exact Dirichlet transition counts using Backdoor Adjustment: $P(Y \mid do(X)) = \sum_z P(Y \mid X=x, Z=z) P(Z=z)$.

#### Scenario: Interventional probability computation via backdoor adjustment
- **WHEN** `compute_empirical_intervention` is queried for variables $X$ and $Y$ with observed transition records across parent states $Z$
- **THEN** it marginalizes over all parent combinations $z$, computes empirical conditional probabilities from `TransitionModel::counts` and `totals`, and returns the exact clamped probability $P(Y \mid do(X)) \in [0.0, 1.0]$

#### Scenario: Parentless causal intervention simplification
- **WHEN** `compute_empirical_intervention` is queried for an intervention variable $X$ that has no parent nodes in the `CausalGraph` DAG
- **THEN** it directly returns the empirical conditional probability $P(Y \mid X=x)$ directly from `TransitionModel` totals without extra parent loop iterations

### Requirement: Structural Equation Model Counterfactual Solver
The `CausalGraph` struct SHALL provide `compute_scm_counterfactual(actual_state, weights, intervention_var, intervention_value)` implementing Pearl's three-step Abduction-Action-Prediction pipeline to solve counterfactual descendant states given linear additive structural equation weights and observed state vectors.

#### Scenario: Abduction-Action-Prediction counterfactual derivation
- **WHEN** `compute_scm_counterfactual` is invoked with an observed state vector, causal edge weight coefficients, and an intervention target
- **THEN** it first abducts exogenous noise terms $U_i = Y_i - \sum_{j \in \text{Parents}(i)} w_{ij} X_j$, substitutes the target intervention value into the structural equations, propagates descendant values forward in topological sort order, and returns the modified counterfactual state map

### Requirement: Monte Carlo Tree Search Trajectory Rollout Simulator
The system SHALL provide an `MctsSimulator` struct (`depth_limit`, `exploration_constant`) with a `search(root_state, transitions, iterations)` method that performs multi-step path rollouts over the `TransitionModel`. The simulator SHALL select tree branches according to the UCB1 multi-armed bandit formula ($\frac{v}{n} + c \sqrt{\frac{\ln N}{n}}$), expand unvisited actions using `TransitionModel::predict()`, evaluate leaf values, and backpropagate cumulative rewards up to `depth_limit`.

#### Scenario: UCB1 action selection and trajectory rollout
- **WHEN** `MctsSimulator::search` runs for a specified number of iterations over a given initial state and transition model
- **THEN** it expands the state tree, balances exploration versus exploitation using UCB1, and returns the optimal first action having the highest cumulative visit count
