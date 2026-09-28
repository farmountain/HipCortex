//! Loop-ready contract — SIT tests (v3.18.0).
//!
//! A consumer bootstraps from `snapshot()`, then pages the transition feed from the
//! snapshot's `tx_cursor`. These tests hold the feed to the contract end to end:
//!
//! - bootstrap: settles at or before the cut are in the snapshot, later ones in the feed;
//! - AC-12: concurrent settles page gap-free, each exactly once, stamps strictly rising;
//! - AC-10: after a restart every earlier cursor is behind the horizon;
//! - AC-18: a caught-up poll is cheap and a full retention window stays bounded;
//! - AC-17 (feature `web-server`): the REST feed carries the paging fields and the SCM.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use chrono::Utc;
use hipcortex::action_intent::{ActionIntent, ActionReceipt};
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::cognitive_state::{CognitiveDelta, CognitiveHandle};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::transition_view::TransitionPage;
use hipcortex::tx_log::TxLog;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::InMemoryBackend;
use uuid::Uuid;

type Handle = CognitiveHandle<InMemoryBackend>;

const ACTOR: &str = "agent";

fn handle_with(tx_log: Option<Arc<TxLog>>) -> Handle {
    CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        Arc::new(RwLock::new(WorldModelEnhanced::new())),
        Arc::new(SelfModel::new()),
        tx_log,
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    )
}

fn open_log(path: &Path) -> Arc<TxLog> {
    Arc::new(TxLog::open(path).unwrap())
}

/// Opens an intent for `actor` and settles it with a successful receipt.
/// Returns the intent id and the receipt's cursor (its settle stamp).
fn settle_new(h: &Handle, actor: &str) -> (Uuid, u64) {
    let intent = ActionIntent::new_probe(actor.to_string(), "filesystem".to_string(), None, 60_000);
    let id = intent.id;
    h.transact(CognitiveDelta::OpenIntent(intent), actor).unwrap();
    let cursor = h
        .transact(
            CognitiveDelta::AcceptReceipt(ActionReceipt {
                intent_id: id,
                ok: true,
                observation: serde_json::json!({ "disk_free_gb": 42.5 }),
                sensor_path: "mcp:filesystem".to_string(),
                ts: Utc::now(),
            }),
            actor,
        )
        .unwrap();
    (id, cursor)
}

fn ids(page: &TransitionPage) -> Vec<Uuid> {
    page.transitions.iter().map(|t| t.intent_id).collect()
}

#[test]
fn consumer_bootstrapped_from_a_snapshot_gets_every_later_settle_from_the_feed() {
    let dir = tempfile::tempdir().unwrap();
    let h = handle_with(Some(open_log(&dir.path().join("tx.jsonl"))));
    let before: Vec<Uuid> = (0..2).map(|_| settle_new(&h, ACTOR).0).collect();

    let cut = h.snapshot(ACTOR).unwrap().tx_cursor;
    let after: Vec<Uuid> = (0..3).map(|_| settle_new(&h, ACTOR).0).collect();

    let page = h.transitions_page(ACTOR, cut);
    assert!(!page.truncated);
    assert_eq!(ids(&page), after, "every later settle, in settle order");
    assert!(before.iter().all(|id| !ids(&page).contains(id)), "settles at or before the cut stay out");

    let caught_up = h.transitions_page(ACTOR, page.next_cursor);
    assert!(caught_up.transitions.is_empty());
    assert_eq!(caught_up.next_cursor, page.next_cursor);
}

#[test]
fn concurrent_settles_are_paged_gap_free_and_exactly_once() {
    const WORKERS: usize = 4;
    const SETTLES: usize = 50;
    let dir = tempfile::tempdir().unwrap();
    let h = Arc::new(handle_with(Some(open_log(&dir.path().join("tx.jsonl")))));

    let workers: Vec<_> = (0..WORKERS)
        .map(|w| {
            let h = Arc::clone(&h);
            std::thread::spawn(move || {
                let actor = format!("worker-{w}");
                (0..SETTLES).map(|_| settle_new(&h, &actor).0).collect::<Vec<_>>()
            })
        })
        .collect();

    // Page every actor ("") while the workers settle, then drain once they are done.
    let mut cursor = 0;
    let mut seen: Vec<Uuid> = Vec::new();
    let mut last_stamp = 0;
    let mut record = |page: TransitionPage, cursor: &mut u64| {
        assert!(!page.truncated);
        for t in &page.transitions {
            assert!(t.settled_tx > last_stamp, "stamps must rise strictly: {} after {}", t.settled_tx, last_stamp);
            last_stamp = t.settled_tx;
            seen.push(t.intent_id);
        }
        *cursor = page.next_cursor;
    };
    while !workers.iter().all(|w| w.is_finished()) {
        record(h.transitions_page("", cursor), &mut cursor);
    }
    let settled: HashSet<Uuid> = workers.into_iter().flat_map(|w| w.join().unwrap()).collect();
    record(h.transitions_page("", cursor), &mut cursor);

    assert_eq!(seen.len(), WORKERS * SETTLES, "each settle delivered exactly once");
    assert_eq!(seen.iter().copied().collect::<HashSet<_>>(), settled);
}

#[test]
fn restart_puts_every_earlier_cursor_behind_the_horizon() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tx.jsonl");
    let last = {
        let h = handle_with(Some(open_log(&path)));
        (0..3).map(|_| settle_new(&h, ACTOR).1).last().unwrap()
    };

    let h = handle_with(Some(open_log(&path)));
    let horizon = h.transitions_page(ACTOR, 0).horizon_tx;
    assert!(horizon >= last, "horizon {horizon} must cover the last stamp {last}");
    assert!(h.transitions_page(ACTOR, 0).truncated);
    assert!(h.transitions_page(ACTOR, horizon - 1).truncated);
    assert!(!h.transitions_page(ACTOR, horizon).truncated);

    let (_, stamp) = settle_new(&h, ACTOR);
    assert!(stamp > horizon, "stamps keep rising past the restart");
    assert_eq!(h.transitions_page(ACTOR, horizon).transitions.len(), 1);
}

fn mean(runs: u32, mut f: impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..runs {
        f();
    }
    start.elapsed() / runs
}

#[test]
fn caught_up_poll_is_cheap_and_a_full_window_is_bounded() {
    // The window is seeded directly: AC-18 measures paging, and settling 4096 receipts
    // through the real path costs minutes (each receipt rescans every Temporal record).
    let window = hipcortex::cognitive_state::DEFAULT_TRANSITION_RETENTION;
    let h = handle_with(None);
    h.open_intents.lock().unwrap().extend((1..=window as u64).map(|stamp| {
        let mut i = ActionIntent::new_probe(ACTOR.to_string(), "filesystem".to_string(), None, 60_000);
        i.status = hipcortex::action_intent::IntentStatus::Received;
        i.receipt_ok = Some(true);
        i.settled_tx = Some(stamp);
        i
    }));
    let last = h.transitions_page(ACTOR, 0).next_cursor;
    assert_eq!(last, window as u64);

    // A caught-up poll returns at most 16 new transitions; measure that upper end.
    let caught_up = mean(200, || assert_eq!(h.transitions_page(ACTOR, last - 16).transitions.len(), 16));
    let full = mean(20, || assert_eq!(h.transitions_page(ACTOR, 0).transitions.len(), window));
    println!("AC-18: caught-up page {caught_up:?} mean, full {window}-page {full:?} mean");
    assert!(caught_up < Duration::from_millis(2), "caught-up page took {caught_up:?}");
    assert!(full < Duration::from_millis(50), "full page took {full:?}");
}

#[cfg(feature = "web-server")]
mod rest {
    use super::*;
    use hipcortex::archive_store::ArchiveStore;
    use hipcortex::aureus_bridge::AureusBridge;
    use hipcortex::scm_view::ScmView;
    use hipcortex::substrate_daemon::SubstrateDaemon;
    use hipcortex::symbolic_store::SymbolicStore;
    use hipcortex::topological_memory::CausalTopoGraph;
    use hipcortex::web_server::{build_app, AppState};
    use hipcortex::workspace::WorkspaceRegistry;
    use serde_json::Value;

    /// App state whose TxLog is shared by the server and the cognitive handle.
    fn state(dir: &tempfile::TempDir, retention: usize) -> (AppState<InMemoryBackend>, Arc<Handle>) {
        let memory_store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
        let world_model = Arc::new(RwLock::new(WorldModelEnhanced::new()));
        let self_model = Arc::new(SelfModel::new());
        let coherence = Arc::new(CoherenceChecker::new());
        let calibration = Arc::new(CalibrationTracker::new());
        let tx_log = open_log(&dir.path().join("tx.jsonl"));
        let cognitive = Arc::new(
            CognitiveHandle::new(
                Arc::clone(&memory_store),
                Arc::clone(&world_model),
                Arc::clone(&self_model),
                Some(Arc::clone(&tx_log)),
                Arc::clone(&coherence),
                Arc::clone(&calibration),
                Arc::new(CognitiveGC::new()),
            )
            .with_transition_retention(retention),
        );
        let state = AppState {
            memory_store,
            symbolic_store: Arc::new(Mutex::new(SymbolicStore::new())),
            world_model,
            aureus: Arc::new(Mutex::new(AureusBridge::new())),
            self_model,
            coherence,
            topo_graph: Arc::new(Mutex::new(CausalTopoGraph::new())),
            archive_store: Arc::new(Mutex::new(ArchiveStore::new_in_memory())),
            tx_log: Some(tx_log),
            calibration,
            cognitive: Arc::clone(&cognitive),
            forks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            twins: Arc::new(Mutex::new(std::collections::HashMap::new())),
            daemon: Arc::new(Mutex::new(SubstrateDaemon::new())),
            workspace_registry: Arc::new(Mutex::new(WorkspaceRegistry::new())),
            passive_capture_enabled: false,
            wm_path: None,
        };
        (state, cognitive)
    }

    async fn serve(state: AppState<InMemoryBackend>) -> String {
        let app = build_app(state);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::Server::from_tcp(listener)
                .unwrap()
                .serve(app.into_make_service())
                .await
                .unwrap_or_default();
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        format!("http://127.0.0.1:{}", addr.port())
    }

    async fn get_json(url: String) -> (u16, Value) {
        let resp = reqwest::get(url).await.unwrap();
        let status = resp.status().as_u16();
        (status, resp.json().await.unwrap())
    }

    #[tokio::test]
    async fn transitions_endpoint_reports_cursor_horizon_and_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let (state, cognitive) = state(&dir, 2);
        let stamps: Vec<u64> = (0..3).map(|_| settle_new(&cognitive, ACTOR).1).collect();
        let base = serve(state).await;

        let (status, page) = get_json(format!("{base}/snapshot/{ACTOR}/transitions?since=0")).await;
        assert_eq!(status, 200);
        assert_eq!(page["count"], 2);
        let got: Vec<u64> = page["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["settled_tx"].as_u64().unwrap())
            .collect();
        assert_eq!(got, stamps[1..]);
        assert_eq!(page["next_cursor"], stamps[2]);
        assert_eq!(page["horizon_tx"], stamps[0]);
        assert_eq!(page["truncated"], true);

        let (_, caught_up) =
            get_json(format!("{base}/snapshot/{ACTOR}/transitions?since={}", stamps[2])).await;
        assert_eq!(caught_up["count"], 0);
        assert_eq!(caught_up["next_cursor"], stamps[2]);
        assert_eq!(caught_up["truncated"], false);
    }

    #[tokio::test]
    async fn scm_endpoint_serves_the_causal_model() {
        let dir = tempfile::tempdir().unwrap();
        let (state, cognitive) = state(&dir, 16);
        {
            let wm = state.world_model.read().unwrap();
            wm.add_causal_edge("a".to_string(), "y".to_string()).unwrap();
            wm.rewrite_structural_equation("y", vec![2.0]).unwrap();
            wm.apply_intervention("a", 1.0).unwrap();
        }
        let expected = cognitive.scm_view().unwrap();
        let base = serve(state).await;

        let (status, body) = get_json(format!("{base}/snapshot/{ACTOR}/scm")).await;
        assert_eq!(status, 200);
        let served: ScmView = serde_json::from_value(body).unwrap();
        assert_eq!(served, expected);
        assert_eq!(served.nodes.iter().find(|n| n.id == "y").unwrap().parents, ["a"]);
    }
}
