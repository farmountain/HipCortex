use hipcortex::hypothesis_manager::HypothesisManager;

#[test]
fn add_and_backtrack() {
    let mut m = HypothesisManager::new();
    let root = m.add_root(1);
    let child = m.add_child(root, 2);
    let path = m.backtrack(child);
    assert_eq!(path, vec![&1, &2]);
}
