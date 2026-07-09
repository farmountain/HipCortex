# record-type-aliases Specification

## Purpose
TBD - created by archiving change record-type-alias-mapping. Update Purpose after archive.
## Requirements
### Requirement: Cognitive Tier Input Aliases Normalization
The memory ingestion API SHALL normalize cognitive-science memory tier aliases to canonical backend enum values before storage.

#### Scenario: Aliased memory type resolution
- **WHEN** a record is added with record_type set to "Episodic" or "Semantic"
- **THEN** it resolves to MemoryType::Temporal and MemoryType::Symbolic respectively

