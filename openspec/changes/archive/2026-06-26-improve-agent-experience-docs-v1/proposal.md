## Why

The documentation and onboarding experience for HipCortex has several critical gaps that hinder the agent and human experience:
- Python users and agents aren't guided to use `pip install hipcortex`, missing a streamlined onboarding path.
- The robust `SafetyGuardrail` feature implemented in the core engine isn't highlighted in architecture or usage docs, leaving users unaware of the system's safety and resilience.
- The MCP tools documentation misses key capabilities like `search_code`, meaning agents and users won't know they can query the code graph.

Solving this will eliminate friction during agent onboarding and give humans a clearer mental model of the system's guardrails and capabilities.

## What Changes

- Update `README.md` to document `pip install hipcortex` for quick Python/Agent setup.
- Update `docs/architecture.md` to explicitly mention `SafetyGuardrail` and its role in protecting the memory substrate.
- Update `sdk/mcp/README.md` tool table to include the `search_code` tool.

## Capabilities

### New Capabilities
- `agent-experience-docs`: Clear documentation covering Python SDK installation, complete MCP tools (`search_code`), and system guardrails.

### Modified Capabilities
- None

## Impact

- **Documentation**: Updates to `README.md`, `docs/architecture.md`, and `sdk/mcp/README.md`.
- **System**: No code changes. Purely documentation improvement.
