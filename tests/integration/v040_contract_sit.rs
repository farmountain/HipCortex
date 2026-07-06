//! SIT tests for v0.4.0 server-extension contract fixes.
//! G-LINK: POST /memory/link field name aliases
//! G-BELIEFS: GET /memory/live_beliefs loops_run key
//! G-RELATED: GET /memory/search/related results enrichment

use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::web_server::AppState;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::self_model::SelfModel;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use hipcortex::CausalTopoGraph;
use std::sync::{Arc, Mutex, RwLock};

fn make_state() -> AppState<InMemoryBackend> {
    AppState {
        memory_store:   Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
        world_model:    Arc::new(RwLock::new(WorldModelEnhanced::new())),
        aureus:         Arc::new(Mutex::new(AureusBridge::new())),
        self_model:     Arc::new(SelfModel::new()),
        coherence:      Arc::new(CoherenceChecker::new()),
        topo_graph:     Arc::new(Mutex::new(CausalTopoGraph::new())),
    }
}

// ── G-LINK: serde alias tests ─────────────────────────────────────────────────

#[test]
fn test_memory_link_request_accepts_source_id_target_id() {
    use hipcortex::web_server::MemoryLinkRequest;
    let json = r#"{"source_id":"00000000-0000-0000-0000-000000000001","target_id":"00000000-0000-0000-0000-000000000002","relation":"supports"}"#;
    let req: MemoryLinkRequest = serde_json::from_str(json)
        .expect("extension-style source_id/target_id must deserialize");
    assert_eq!(req.from_id, "00000000-0000-0000-0000-000000000001");
    assert_eq!(req.to_id,   "00000000-0000-0000-0000-000000000002");
    assert_eq!(req.relation, "supports");
}

#[test]
fn test_memory_link_request_still_accepts_from_id_to_id() {
    use hipcortex::web_server::MemoryLinkRequest;
    let json = r#"{"from_id":"00000000-0000-0000-0000-000000000003","to_id":"00000000-0000-0000-0000-000000000004","relation":"caused_by"}"#;
    let req: MemoryLinkRequest = serde_json::from_str(json)
        .expect("SDK-style from_id/to_id must still deserialize");
    assert_eq!(req.from_id, "00000000-0000-0000-0000-000000000003");
    assert_eq!(req.to_id,   "00000000-0000-0000-0000-000000000004");
}
