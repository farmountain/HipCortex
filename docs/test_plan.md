# Test Plan

This document lists the main test scenarios for verifying the OpenManus/MCP integration path.

## Unit Tests
- `conversation_memory_tests::ingest_openmanus_json` – verifies that OpenManus JSON messages are parsed into `ConversationMemory`.
- `integration_layer_tests::*` – basic connection, authentication and message send logic of `IntegrationLayer`.
- `perception_adapter_tests::adapt_text` – exercises the `PerceptionAdapter` with text inputs.
- `aureus_bridge_tests::*` – validates reflexion loops via the LLM client trait.

## System Integration Tests (SIT)
- `conversation_memory_sit::openmanus_round_trip_into_store` – end to end flow from conversation memory into a memory store.
- `openmanus_integration_sit::openmanus_message_through_adapter_and_layer` – sends an OpenManus message through the `PerceptionAdapter` and out via `IntegrationLayer`.
- `llm_integration_tests::reflexion_loop_stores_response` – ensures the bridge stores responses when using an LLM connector.

## User Acceptance Tests (UAT)
- `conversation_memory_uat::user_flow_conversation_memory` – mimics a user session storing messages and sending a final result through the `IntegrationLayer`.
- `uat_tests::athena_chain_of_thought_reasoning` – checks CoT reflexion results with the mock LLM client.

Run all tests with:
```bash
cargo test
```
All tests should pass without failures.
