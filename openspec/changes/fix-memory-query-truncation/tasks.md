## 1. Web Server Handlers Slice-Time Sorting (`src/web_server.rs`)

- [x] 1.1 Verify and assert timestamp-descending sorting before `filtered_records.truncate(limit)` in `handle_query_memory` (`/memory/query`)
- [x] 1.2 Update `handle_memory_live_beliefs` (`/memory/beliefs`) to sort `pinned` beliefs descending by timestamp before applying `truncate(limit)`

## 2. Memory Store Semantic Search Slice-Time Sorting (`src/memory_store.rs`)

- [x] 2.1 Update `MemoryStore::search_semantic` (`top_k_records`) to sort candidate `pinned` records descending by `timestamp` before merging and applying `truncate(limit)`

## 3. Longitudinal Integration Testing (`tests/integration/`)

- [x] 3.1 Create or update integration test (`tests/integration/web_server_gaps_sit.rs`) with a longitudinal test case pre-seeding `> limit` timestamped records (`Day 1` to `Day 150`) and verifying newest-first survival under query limits
- [x] 3.2 Add integration test verifying newest-first survival of pinned beliefs in `handle_memory_live_beliefs` under limit truncation
- [x] 3.3 Add integration test verifying newest-first survival of pinned records in `search_semantic` under limit truncation

## 4. Verification & Build

- [x] 4.1 Run full cargo test suite (`cargo test --lib --tests`) to verify 100% pass across all unit and integration tests
