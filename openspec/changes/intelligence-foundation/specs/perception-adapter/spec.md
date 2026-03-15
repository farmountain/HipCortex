## ADDED Requirements

### Requirement: Self-Model Resource Validation
The Perception Adapter SHALL validate resource availability before expensive PCA operations.

#### Scenario: Resource check before PCA
- **WHEN** code requests PCA dimensionality reduction on perception data
- **THEN** the Perception Adapter SHALL call self_model.check_resources() to verify sufficient CPU and memory

#### Scenario: Graceful degradation
- **WHEN** Self-Model indicates insufficient resources for PCA
- **THEN** the Perception Adapter SHALL skip PCA and return raw features with warning

### Requirement: World-Model Perception Integration
The Perception Adapter SHALL feed processed perceptions to World-Model for entity tracking.

#### Scenario: Perception to entity mapping
- **WHEN** PCA processes perception data
- **THEN** the Perception Adapter SHALL extract entity information and update world_model.update_entity() with observed properties

#### Scenario: Anomaly detection from perception
- **WHEN** World-Model detects anomaly in entity perception
- **THEN** the Perception Adapter SHALL flag anomalous perception for further investigation

### Requirement: Coherence for Perception Consistency
The Perception Adapter SHALL ensure perceptions are consistent with existing symbolic knowledge.

#### Scenario: Perception validation
- **WHEN** perception data is processed
- **THEN** the Perception Adapter SHALL verify entities in perception exist in symbolic_store via coherence_checker

#### Scenario: Novel entity handling
- **WHEN** perception contains entity not in symbolic store
- **THEN** the Perception Adapter SHALL create temporary entity and flag for coherence review
