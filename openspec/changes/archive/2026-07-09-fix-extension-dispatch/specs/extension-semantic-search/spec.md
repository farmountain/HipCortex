## ADDED Requirements

### Requirement: Scored Semantic Search in VS Code Extension
The VS Code extension SHALL support scored semantic search when processing a search request.

#### Scenario: Successful semantic search display
- **WHEN** the user runs a search command through the chat participant
- **THEN** the extension queries POST /memory/search and renders results with their match score percentage
