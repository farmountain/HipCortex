## ADDED Requirements

### Requirement: Enhanced Memory Ingest Parsing
The memory ingestion endpoints (`/memory/add` and `/memory/bulk`) SHALL parse aliases in both single and batch ingestion routes.

#### Scenario: Bulk ingestion with aliases
- **WHEN** a list of records with various cognitive-science aliases is bulk ingested
- **THEN** all records are parsed, mapped, and successfully stored in the database
