# Symbolic and Procedural Memory Test Plan

*Date: June 5, 2025*

This document lists the tests validating symbolic reasoning and procedural state management within HipCortex.

## Unit Tests
- **add_and_get_node** – ensures a node can be created and retrieved by ID.
- **add_edge_and_neighbors** – verifies edges produce neighbor results.
- **add_edge_to_nonexistent_node** – confirms edges to nonexistent nodes don't break queries.
- **duplicate_node** – adds nodes with the same label to ensure unique IDs.
- **update_find_and_remove_node** – covers property updates and node removal.
- **edges_from_query** – checks retrieving outgoing edges for a node.
- **query_by_label_and_property** – validates label and property search filters.
- **advance_transition** – tests a single procedural state transition.
- **checkpoint_roundtrip** – verifies saving and loading procedural trace checkpoints.

## System Integration Tests
- **edge_workflow_small_device** – exercises an end-to-end workflow combining the TemporalIndexer, ProceduralCache and MemoryStore on a small device scenario.
- **memory_round_trip** – inserts a symbolic node, stores it in temporal memory and verifies retrieval.
- **store_reasoning_trace_via_adapter_and_indexer** – adapts a text input and stores it as a temporal trace.
- **query_symbol_via_indexer** – retrieves a symbolic node referenced from temporal memory.
- **retrieval_round_trip** – integrates the PerceptionAdapter, SymbolicStore and TemporalIndexer through the retrieval pipeline.

## User Acceptance Tests
- **user_runs_edge_workflow** – simulates a user executing an edge workflow via the IntegrationLayer.
- **user_flow_conversation_memory** – exercises capturing messages and confirming conversation state.
- **user_retrieve_recent_document** – stores a document and retrieves it through the pipeline.

All tests pass via `cargo test`, confirming that symbolic and procedural memory features operate correctly.
