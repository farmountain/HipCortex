use hipcortex::aureus_bridge::{AureusBridge, AureusConfig};
use hipcortex::llm_clients::mock::MockClient;
use hipcortex::memory_store::MemoryStore;

#[test]
fn reflexion_loop_stores_response() {
    let path = "test_llm_mem.jsonl";
    let _ = std::fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    store.clear();
    let mut bridge = AureusBridge::with_client(Box::new(MockClient));
    bridge.configure(AureusConfig {
        enable_cot: true,
        prune_threshold: 0.2,
    });
    bridge.reflexion_loop("context", &mut store);
    assert_eq!(store.all().len(), 1);
    std::fs::remove_file(path).ok();
}
