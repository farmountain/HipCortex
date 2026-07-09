# extension-chat-dispatch Specification

## Purpose
TBD - created by archiving change fix-extension-dispatch. Update Purpose after archive.
## Requirements
### Requirement: Chat command separation for query and search
The extension SHALL separate dispatch routing for `query` and `search` prefixes.

#### Scenario: Dispatch split
- **WHEN** user inputs a message starting with "query"
- **THEN** it routes to handleQueryMemory, and messages starting with "search" route to handleSearchMemory

