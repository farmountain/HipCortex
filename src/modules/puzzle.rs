use std::collections::{HashSet, VecDeque};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct PuzzleState(pub [u8; 9]);

impl PuzzleState {
    fn neighbors(&self) -> Vec<PuzzleState> {
        let idx = self.0.iter().position(|&x| x == 0).unwrap();
        let mut swaps = Vec::new();
        let row = idx / 3;
        let col = idx % 3;
        if row > 0 {
            swaps.push(idx - 3);
        }
        if row < 2 {
            swaps.push(idx + 3);
        }
        if col > 0 {
            swaps.push(idx - 1);
        }
        if col < 2 {
            swaps.push(idx + 1);
        }
        let mut neigh = Vec::new();
        for s in swaps {
            let mut b = self.0;
            b[idx] = b[s];
            b[s] = 0;
            neigh.push(PuzzleState(b));
        }
        neigh
    }
}

pub fn solve(start: PuzzleState, goal: PuzzleState) -> Option<Vec<PuzzleState>> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((start.clone(), vec![start.clone()]));
    while let Some((state, path)) = queue.pop_front() {
        if state == goal {
            return Some(path);
        }
        if visited.insert(state.clone()) {
            for n in state.neighbors() {
                let mut p = path.clone();
                p.push(n.clone());
                queue.push_back((n, p));
            }
        }
    }
    None
}
