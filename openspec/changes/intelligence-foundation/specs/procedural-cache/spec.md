## ADDED Requirements

### Requirement: Self-Model Decision Gating
The Procedural Cache SHALL consult Self-Model before FSM transitions to ensure system readiness.

#### Scenario: Capability check before transition
- **WHEN** FSM attempts state transition
- **THEN** the Procedural Cache SHALL call self_model.can_execute("procedural_transition", context)

#### Scenario: Transition based on system health
- **WHEN** Self-Model health score <0.5
- **THEN** the Procedural Cache SHALL allow only safe/rollback transitions and block risky state changes

### Requirement: World-Model State Updates
The Procedural Cache SHALL notify World-Model of FSM state changes to improve transition predictions.

#### Scenario: FSM transition observation
- **WHEN** an FSM successfully transitions from state S to S' via action A
- **THEN** the Procedural Cache SHALL record transition with world_model.observe_transition(S, A, S')

#### Scenario: FSM state properties
- **WHEN** FSM state includes additional properties (context variables)
- **THEN** the Procedural Cache SHALL include property values in world_model observation for richer modeling

### Requirement: Coherence for FSM Consistency
The Procedural Cache SHALL ensure FSM transitions are coherent with World-Model predictions.

#### Scenario: Pre-transition coherence check
- **WHEN** FSM plans transition
- **THEN** the Procedural Cache SHALL query world_model.predict() and verify predicted state matches FSM target state

#### Scenario: Incoherent transition detection
- **WHEN** FSM transition contradicts World-Model (FSM allows S→S' but world-model P(S'|S,A) < 0.1)
- **THEN** the Procedural Cache SHALL flag to coherence_checker as ProceduralWorldConflict
