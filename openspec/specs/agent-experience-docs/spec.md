# agent-experience-docs Specification

## Purpose
Provides standardized guidelines, graph architecture constraints, safety guardrails, and step-by-step execution flow traces for autonomous agents and contributors pair programming in the HipCortex memory engine.

## Requirements

### Requirement: Agent Code Intelligence & Architecture Guidelines
The documentation SHALL clearly specify crate namespace attribute hoisting (`src/lib.rs`), cryptographic persistence routing (`MemoryStore`), and safety guardrail hooks (`SafetyGuardrail`).

#### Scenario: Agent contributes to codebase
- **WHEN** an AI pair programming agent or human contributor prepares to modify files in `src/modules/`
- **THEN** they reference the formalized Agent Operating Rules to ensure changes are hoisted in `src/lib.rs` and intercepted by `SafetyGuardrail`.

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

