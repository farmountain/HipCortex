## ADDED Requirements

### Requirement: Pinned search candidate ordering under truncation limits
When the `search_semantic` or hybrid search pipeline collects records with `priority == "pinned"`, it SHALL sort those candidates descending by `timestamp` before merging or truncating to `limit`. This guarantees that high-volume pinned memory collections do not suffer from insertion-order starvation where old initial pinned records crowd out newly created pinned records.

#### Scenario: High volume pinned candidate ordering
- **WHEN** multiple pinned candidates match semantic filters and total pinned count `P` exceeds `limit` `L`
- **THEN** the top `L` candidates retained by the search pipeline SHALL be the `L` most recently timestamped pinned records
