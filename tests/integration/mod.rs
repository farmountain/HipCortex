mod agent_bridge_sit;
mod v110_cognitive_sit;
#[cfg(feature = "web-server")]
mod v110_rest_sit;
mod cli_tests;
mod conversation_memory_sit;
mod conversation_memory_uat;
mod edge_workflow_sit;
mod edge_workflow_uat;
mod effort_confidence_sit;
mod effort_confidence_uat;
mod graph_backend_sit;
#[cfg(feature = "grpc-server")]
mod grpc_tests;
mod humanoid_perception_uat;
mod integration_tests;
mod intelligence_hooks_sit;
#[cfg(feature = "web-server")]
mod intelligence_sit;
mod intelligence_uat;
#[cfg(feature = "web-server")]
mod intelligence_wiring_sit;
mod llm_integration_tests;
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
mod mcp_server_sit;
#[cfg(all(feature = "web-server", feature = "grpc-server"))]
mod mcp_server_uat;
mod openmanus_integration_sit;
mod plugin_host_sit;
mod plugin_host_uat;
mod rag_export_sit;
mod rag_export_uat;
mod react_engine_sit;
mod loop_omega_sit;
mod reasoning_trace_sit_tests;
mod retrieval_pipeline_sit;
mod retrieval_pipeline_uat;
mod safety_guardrail_sit;
mod semantic_cache_sit;
mod semantic_cache_uat;
mod semantic_compression_sit;
mod semantic_compression_uat;
#[cfg(feature = "web-server")]
mod sit_tests;
mod smart_glasses_sit;
mod system_integration_tests;
mod test_end_to_end;
#[cfg(feature = "web-server")]
mod uat_tests;
mod web_server_gaps_sit;
mod world_model_cli_sit;
mod world_model_uat;
#[cfg(feature = "web-server")]
mod worldmodel_self_http_sit;

mod cognitive_os_test;
mod consolidation_gates_sit;
mod hybrid_rollout_sit;
mod omega_loop_auditable_tests;
mod react_e2e_sit;
mod credit_assign_sit;
mod ood_invariance_sit;
#[cfg(feature = "web-server")]
mod rollout_bounds_sit;
mod topological_substrate_tests;
mod causal_compactor_sit;
mod workspace_durability_sit;
mod critic_verifier_sit;
mod loop_gates_sit;
mod motif_contraction_sit;
mod restart_survivability_sit;
mod epistemic_write_path_sit;
mod cognitive_coherence_sit;
