# Retrieval Pipeline Test Plan

*Date: June 5, 2025*

This document lists the tests validating retrieval pipelines for RAG built on top of the `SymbolicStore` and `TemporalIndexer`.

## Unit Tests
- **recent_symbols_returns_nodes:** ensures that retrieving recent symbols yields the expected `SymbolicNode` from the store.
- **recent_symbols_limit:** verifies that the number of results respects the `n` argument.

## System Integration Tests
- **retrieval_round_trip:** inserts a node, adapts a perception input, stores a temporal trace, and retrieves the node via the pipeline.

## User Acceptance Tests
- **user_retrieve_recent_document:** simulates a user storing a document and fetching it with the retrieval pipeline.

All tests pass using `cargo test`, confirming the pipeline works across modules.
