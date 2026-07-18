## ADDED Requirements

### Requirement: Deterministic Newest-First Truncation on Memory Queries
All memory query endpoints (`GET /memory/query`) and paginated/bounded retrieval functions SHALL sort candidate records by `timestamp` descending (`b.timestamp.cmp(&a.timestamp)`) before enforcing result count ceilings (`limit`). Whenever total matching records exceed `limit`, the returned array MUST contain the most recently timestamped memories up to `limit`.

#### Scenario: Query limit truncation on longitudinal data
- **WHEN** the store contains `200` matching records spanning sequential timestamps from `Day 1` to `Day 200` and `queryMemory({ limit: 10 })` is invoked
- **THEN** the returned records SHALL consist exactly of the `10` newest records (`Day 200` through `Day 191`) sorted newest-first

### Requirement: Deterministic Pinned Record Truncation in Semantic Search
The `MemoryStore::search_semantic` (`top_k_records`) pipeline SHALL sort matching `pinned` priority records by `timestamp` descending before applying `.truncate(limit)`. When more than `limit` pinned records match the search criteria, the returned vector SHALL preserve the `limit` newest pinned records and discard older pinned candidates.

#### Scenario: Pinned records count exceeds search limit
- **WHEN** the store contains `20` matching records with `priority="pinned"` spanning `Day 1` to `Day 20` and `search_semantic` is called with `limit=5`
- **THEN** the results SHALL contain the `5` newest pinned records (`Day 20` through `Day 16`) at score `2.0`
