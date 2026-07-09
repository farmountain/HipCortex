## ADDED Requirements

### Requirement: Causal LM Tool Registration
The extension SHALL register a `hipcortex_causal` LM tool with the VS Code language model API.

#### Scenario: Causal tool invocation
- **WHEN** the `hipcortex_causal` tool is invoked with a mode and query
- **THEN** it executes the corresponding HTTP request against the WorldModel causal endpoints and returns the JSON result
