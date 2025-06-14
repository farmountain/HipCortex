# Retrieval Pipeline Test Plan

*Date: June 5, 2025*

This document lists the tests validating retrieval pipelines for RAG built on top of the `SymbolicStore` and `TemporalIndexer`.

## Unit Tests
- **recent_symbols_returns_nodes:** ensures that retrieving recent symbols yields the expected `SymbolicNode` from the store.
- **recent_symbols_limit:** verifies that the number of results respects the `n` argument.
- **local_rag_retrieve_match:** tests the `LocalRagAdapter` helper for label queries.

## System Integration Tests
- **retrieval_round_trip:** inserts a node, adapts a perception input, stores a temporal trace, and retrieves the node via the pipeline.
- **rag_to_pdf_round_trip:** retrieves a node and exports it to PDF.

## User Acceptance Tests
- **user_retrieve_recent_document:** simulates a user storing a document and fetching it with the retrieval pipeline.
- **user_export_to_notion:** retrieves a node and exports it to Notion.

All tests pass using `cargo test`, confirming the pipeline works across modules.
