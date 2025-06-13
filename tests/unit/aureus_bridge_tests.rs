use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::memory_store::MemoryStore;
use hipcortex::aureus_bridge::AureusConfig;
use hipcortex::llm_clients::mock::MockClient;

#[test]
fn aureus_bridge_reflexion_loop() {
    let mut aureus = AureusBridge::new();
    let mut store = MemoryStore::new("test_aureus.jsonl").unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
}

#[test]
fn aureus_bridge_multiple_reflexion_loops() {
    let mut aureus = AureusBridge::new();
    let mut store = MemoryStore::new("test_aureus.jsonl").unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
    aureus.reflexion_loop("ctx", &mut store);
}

#[test]
fn aureus_bridge_loop_counter() {
    let mut aureus = AureusBridge::new();
    let mut store = MemoryStore::new("test_aureus.jsonl").unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
    aureus.reflexion_loop("ctx", &mut store);
    assert_eq!(aureus.loops_run(), 2);
    aureus.reset();
    assert_eq!(aureus.loops_run(), 0);
}

#[test]
fn aureus_bridge_chain_of_thought_prompt() {
    let path = "test_aureus_cot.jsonl";
    let _ = std::fs::remove_file(path);
    let mut aureus = AureusBridge::with_client(Box::new(MockClient));
    aureus.configure(AureusConfig { enable_cot: true });
    let mut store = MemoryStore::new(path).unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
    let records = store.all();
    assert_eq!(records.len(), 1);
    assert!(records[0].target.contains("Think step by step."));
    std::fs::remove_file(path).ok();
}
