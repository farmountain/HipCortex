use crate::procedural_cache::ProceduralTrace;
use std::sync::{Arc, Mutex};

pub trait A2AClient: Send + Sync {
    fn send_trace(&self, trace: &ProceduralTrace);
}

pub struct LocalPeer {
    inbox: Arc<Mutex<Vec<ProceduralTrace>>>,
}

impl LocalPeer {
    pub fn new() -> Self {
        Self {
            inbox: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn inbox(&self) -> Arc<Mutex<Vec<ProceduralTrace>>> {
        self.inbox.clone()
    }
}

impl A2AClient for LocalPeer {
    fn send_trace(&self, trace: &ProceduralTrace) {
        self.inbox.lock().unwrap().push(trace.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::procedural_cache::{FSMState, ProceduralTrace};
    use std::collections::HashMap;

    #[test]
    fn send_and_receive() {
        let peer = LocalPeer::new();
        let trace = ProceduralTrace {
            id: uuid::Uuid::new_v4(),
            current_state: FSMState::Start,
            memory: HashMap::new(),
        };
        peer.send_trace(&trace);
        let inbox = peer.inbox();
        assert_eq!(inbox.lock().unwrap().len(), 1);
    }
}
