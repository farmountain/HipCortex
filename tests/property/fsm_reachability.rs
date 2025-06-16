use hipcortex::procedural_cache::{FSMState, FSMTransition, ProceduralCache, ProceduralTrace};
use proptest::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

proptest! {
    #[test]
    fn reach_end_state(steps in 1usize..5) {
        let mut cache = ProceduralCache::new();
        let trace = ProceduralTrace {
            id: Uuid::new_v4(),
            current_state: FSMState::Start,
            memory: HashMap::new(),
        };
        cache.add_trace(trace.clone());
        for _ in 0..steps {
            cache.add_transition(FSMTransition { from: FSMState::Start, to: FSMState::Observe, condition: None });
            cache.add_transition(FSMTransition { from: FSMState::Observe, to: FSMState::Act, condition: None });
            cache.add_transition(FSMTransition { from: FSMState::Act, to: FSMState::End, condition: None });
        }
        // run sequence Start -> Observe -> Act -> End
        let _ = cache.advance(trace.id, None);
        let _ = cache.advance(trace.id, None);
        let end_state = cache.advance(trace.id, None);
        prop_assert_eq!(end_state, Some(FSMState::End));
    }
}
