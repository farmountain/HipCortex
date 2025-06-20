use approx::assert_abs_diff_eq;
use hipcortex::hypothesis_manager::HypothesisManager;
use std::fs;
use tempfile::NamedTempFile;

#[test]
fn update_probability_math() {
    let m: HypothesisManager<()> = HypothesisManager::new();
    let posterior = m.update_probability(0.5, 0.8);
    assert_abs_diff_eq!(posterior, 0.8, epsilon = 1e-6);
}

#[test]
fn best_path_and_backtrack() {
    let mut m = HypothesisManager::new();
    let root = m.add_root("root", 0.6);
    let a = m.add_child(root, "a", 0.7);
    let _b = m.add_child(root, "b", 0.4);
    // best leaf should be `a`
    let best = m.best_path();
    assert_eq!(
        best.iter().map(|h| h.state).collect::<Vec<_>>(),
        vec!["root", "a"]
    );
    // backtracking from a
    let path = m.backtrack(a);
    assert_eq!(
        path.iter().map(|h| h.state).collect::<Vec<_>>(),
        vec!["root", "a"]
    );
}

#[test]
fn pruning_removes_low_probability() {
    let mut m = HypothesisManager::new();
    let root = m.add_root("root", 0.6);
    let _good = m.add_child(root, "good", 0.7);
    let bad = m.add_child(root, "bad", 0.1);
    m.prune_low_probability(0.2);
    assert!(m.get(bad).is_none());
    assert!(m.get(root).is_some());
}

#[test]
fn export_generates_dot() {
    let mut m = HypothesisManager::new();
    let root = m.add_root("root", 0.5);
    m.add_child(root, "child", 0.6);
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_path_buf();
    m.export_dot(path.to_str().unwrap());
    let contents = fs::read_to_string(path).unwrap();
    assert!(contents.contains("digraph"));
}
