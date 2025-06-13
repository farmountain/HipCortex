mod cli_tests;
#[cfg(feature = "grpc-server")]
mod grpc_tests;
mod integration_tests;
mod llm_integration_tests;
mod reasoning_trace_sit_tests;
mod system_integration_tests;
mod world_model_cli_sit;
mod world_model_uat;
mod conversation_memory_sit;
mod conversation_memory_uat;
mod test_end_to_end;
mod uat_tests;
mod retrieval_pipeline_sit;
mod retrieval_pipeline_uat;
mod edge_workflow_sit;
mod edge_workflow_uat;
