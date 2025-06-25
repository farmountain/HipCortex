use hipcortex::safety_guardrail::SAFETY_GUARDRAIL;
use uuid::Uuid;

#[test]
fn pre_and_post_checks() {
    let mut guard = SAFETY_GUARDRAIL.lock().unwrap();
    guard.reset();
    assert!(guard.check_precondition("ok").is_ok());
    assert!(guard.check_precondition("invalid input").is_err());
    assert!(guard.check_postcondition("fine").is_ok());
    assert!(guard.check_postcondition("some error").is_err());
    assert!(guard.violation_count() >= 2);
}

#[test]
fn graph_backend_blocked() {
    use hipcortex::symbolic_store::SymbolicStore;
    use std::collections::HashMap;
    let mut guard = SAFETY_GUARDRAIL.lock().unwrap();
    guard.reset();
    drop(guard);
    let mut store = SymbolicStore::new();
    let id = store.add_node("invalid", HashMap::new());
    assert_eq!(id, Uuid::nil());
}

#[test]
fn integration_layer_blocked() {
    use hipcortex::integration_layer::IntegrationLayer;
    let mut guard = SAFETY_GUARDRAIL.lock().unwrap();
    guard.reset();
    drop(guard);
    let layer = IntegrationLayer::new();
    layer.send_message("k", "invalid message");
    let guard = SAFETY_GUARDRAIL.lock().unwrap();
    assert!(guard.violation_count() > 0);
}

#[test]
fn semantic_cache_blocked() {
    use hipcortex::semantic_cache::SemanticCache;
    let mut guard = SAFETY_GUARDRAIL.lock().unwrap();
    guard.reset();
    drop(guard);
    let mut cache = SemanticCache::new(2);
    cache.put_embedding("k".into(), vec![]);
    let guard = SAFETY_GUARDRAIL.lock().unwrap();
    assert!(guard.violation_count() > 0);
}

#[test]
fn llm_prompt_blocked() {
    use hipcortex::llm_clients::local_llm_client::LocalLLMClient;
    use hipcortex::llm_clients::LLMClient;
    let mut guard = SAFETY_GUARDRAIL.lock().unwrap();
    guard.reset();
    drop(guard);
    let client = LocalLLMClient::new("echo");
    let resp = client.generate_response("invalid prompt");
    assert!(resp.is_empty());
    let guard = SAFETY_GUARDRAIL.lock().unwrap();
    assert!(guard.violation_count() > 0);
}
