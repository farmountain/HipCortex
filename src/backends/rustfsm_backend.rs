use crate::procedural_cache::{FSMBackend, FSMState, FSMTransition, ProceduralTrace};
use petgraph::prelude::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct RustFSMBackend {
    traces: HashMap<Uuid, ProceduralTrace>,
    graph: DiGraph<FSMState, Option<String>>, // edge weights are conditions
    node_map: HashMap<FSMState, NodeIndex>,
}

impl RustFSMBackend {
    pub fn new() -> Self {
        Self {
            traces: HashMap::new(),
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    fn get_or_insert(&mut self, state: FSMState) -> NodeIndex {
        if let Some(idx) = self.node_map.get(&state) {
            *idx
        } else {
            let idx = self.graph.add_node(state.clone());
            self.node_map.insert(state, idx);
            idx
        }
    }
}

impl FSMBackend for RustFSMBackend {
    fn add_trace(&mut self, trace: ProceduralTrace) {
        self.traces.insert(trace.id, trace);
    }

    fn add_transition(&mut self, transition: FSMTransition) {
        let from = self.get_or_insert(transition.from.clone());
        let to = self.get_or_insert(transition.to.clone());
        self.graph.add_edge(from, to, transition.condition.clone());
    }

    fn advance(&mut self, trace_id: Uuid, condition: Option<&str>) -> Option<FSMState> {
        let trace = self.traces.get_mut(&trace_id)?;
        let from_idx = *self.node_map.get(&trace.current_state)?;
        let mut next = None;
        for edge in self.graph.edges(from_idx) {
            if edge.weight().as_deref() == condition {
                next = Some(edge.target());
                break;
            }
        }
        let idx = next?;
        let new_state = self.graph[idx].clone();
        trace.current_state = new_state.clone();
        Some(new_state)
    }

    fn advance_batch(
        &mut self,
        trace_ids: &[Uuid],
        condition: Option<&str>,
    ) -> Vec<Option<FSMState>> {
        trace_ids
            .iter()
            .map(|id| self.advance(*id, condition))
            .collect()
    }

    fn assert_fsm_invariants(&self) {
        // reachability from Start
        if let Some(start_idx) = self.node_map.get(&FSMState::Start) {
            let mut bfs = petgraph::visit::Bfs::new(&self.graph, *start_idx);
            let mut visited = HashSet::new();
            while let Some(nx) = bfs.next(&self.graph) {
                visited.insert(nx);
            }
            assert_eq!(
                visited.len(),
                self.graph.node_count(),
                "unreachable states detected"
            );
        }
        // deterministic transitions
        for idx in self.graph.node_indices() {
            let mut seen: HashMap<Option<String>, NodeIndex> = HashMap::new();
            for edge in self.graph.edges(idx) {
                let cond = edge.weight().clone();
                if let Some(existing) = seen.insert(cond.clone(), edge.target()) {
                    assert_eq!(existing, edge.target(), "ambiguous transition");
                }
            }
        }
    }

    fn traces(&self) -> &HashMap<Uuid, ProceduralTrace> {
        &self.traces
    }

    fn traces_mut(&mut self) -> &mut HashMap<Uuid, ProceduralTrace> {
        &mut self.traces
    }
}
