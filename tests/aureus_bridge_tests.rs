use hipcortex::aureus_bridge::AureusBridge;

#[test]
fn aureus_bridge_reflexion_loop() {
    let mut aureus = AureusBridge::new();
    aureus.reflexion_loop();
}

#[test]
fn aureus_bridge_multiple_reflexion_loops() {
    let mut aureus = AureusBridge::new();
    aureus.reflexion_loop();
    aureus.reflexion_loop();
}

#[test]
fn aureus_bridge_loop_counter() {
    let mut aureus = AureusBridge::new();
    aureus.reflexion_loop();
    aureus.reflexion_loop();
    assert_eq!(aureus.loops_run(), 2);
    aureus.reset();
    assert_eq!(aureus.loops_run(), 0);
}
