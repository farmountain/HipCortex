## ADDED Requirements

### Requirement: Python SDK Installation Documentation
The documentation SHALL clearly provide instructions on how to install the Python SDK via `pip`.

#### Scenario: User reads README
- **WHEN** a user or agent opens the project `README.md`
- **THEN** they find clear `pip install hipcortex` instructions for installing the CLI/SDK without needing Rust.

### Requirement: SafetyGuardrail Documentation
The documentation SHALL describe the `SafetyGuardrail` system and how it protects memory mutations in the integration layer.

#### Scenario: User reviews architecture
- **WHEN** a user or agent reads `docs/architecture.md`
- **THEN** they see an explanation of `SafetyGuardrail` preventing unauthorized or malformed memory writes.

### Requirement: Complete MCP Tools Reference
The MCP documentation SHALL list all available tools provided by the `server.py`, including `search_code`.

#### Scenario: Agent checks available tools
- **WHEN** an agent reads `sdk/mcp/README.md`
- **THEN** the Tools table includes the `search_code` tool along with its description.
