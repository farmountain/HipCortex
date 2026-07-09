## ADDED Requirements

### Requirement: Enhanced MCP add_memory Schema
The Python MCP server's `add_memory` tool schema SHALL define the `record_type` parameter containing all 5 backend cognitive tiers.

#### Scenario: Add memory tool definition
- **WHEN** client requests tools/list
- **THEN** the returned schema for add_memory lists all 5 tiers ("Temporal", "Symbolic", "Procedural", "Reflexion", "Perception") in its enum
