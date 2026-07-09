## ADDED Requirements

### Requirement: Multi-step WorldModel Rollout
The backend server SHALL expose a POST `/worldmodel/rollout` endpoint that performs multi-step recursive state prediction.

#### Scenario: Successful multi-step rollout
- **WHEN** a POST request is sent to `/worldmodel/rollout` with a valid initial state and list of actions
- **THEN** it returns the predicted final state and ensemble confidence score
