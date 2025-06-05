use hipcortex::aureus_bridge::AureusBridge;

#[test]
fn aureus_bridge_reflexion_loop() {
    let aureus = AureusBridge::new();
    aureus.reflexion_loop();
}

#[test]
fn aureus_bridge_multiple_reflexion_loops() {
    let aureus = AureusBridge::new();
    aureus.reflexion_loop();
    aureus.reflexion_loop();
}
