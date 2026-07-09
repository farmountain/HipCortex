## Context

The current documentation lacks visibility into `pip install hipcortex`, the `SafetyGuardrail` enforcement mechanism, and the `search_code` MCP tool. This leads to friction during agent onboarding and a lack of awareness about the system's safety guarantees and graph query capabilities.

## Goals / Non-Goals

**Goals:**
- Provide clear setup instructions for Python/Agent users.
- Explicitly document the system's built-in safety mechanisms.
- Ensure the MCP tools documentation is exhaustive.

**Non-Goals:**
- Modifying the underlying Rust engine or Python SDK codebase.
- Re-architecting the documentation structure beyond simple additions.

## Decisions

- **Prominent `pip` instruction**: `README.md` will list `pip install hipcortex` prominently in the Quickstart or a new "Python SDK & CLI" section. This is preferred over burying it in `docs/usage.md` because most agent users look at `README.md` first.
- **Safety Documentation**: `docs/architecture.md` will get a dedicated section or bullet point detailing how `SafetyGuardrail` protects the memory substrate from bad state mutations.
- **MCP Tools Table Update**: `sdk/mcp/README.md` will add a row for `search_code` to match the implementation in `sdk/mcp/server.py`.

## Risks / Trade-offs

- **Risk**: Documentation drift if MCP tools are updated again.
  - **Mitigation**: Future changes to MCP tools should include updating `sdk/mcp/README.md` as part of the PR checklist.
