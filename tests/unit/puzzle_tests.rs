use hipcortex::puzzle::{solve, PuzzleState};

#[test]
fn solve_simple_puzzle() {
    let start = PuzzleState([1, 2, 3, 4, 5, 6, 7, 0, 8]);
    let goal = PuzzleState([1, 2, 3, 4, 5, 6, 7, 8, 0]);
    let path = solve(start, goal).unwrap();
    assert!(path.len() > 1);
}
