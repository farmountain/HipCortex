use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::memory_store::MemoryStore;

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
