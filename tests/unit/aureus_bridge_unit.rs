use hipcortex::aureus_bridge::AureusBridge;

#[test]
fn loops_increment() {
    let mut a = AureusBridge::new();
    a.reflexion_loop();
    assert_eq!(a.loops_run(), 1);
    a.reset();
    assert_eq!(a.loops_run(), 0);
}
