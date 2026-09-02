// Unit tests for SubstrateDaemon (Phase-D, G-LOOP, AC-8).

use std::sync::{Arc, Mutex, RwLock};
use hipcortex::substrate_daemon::{DaemonStatus, SubstrateDaemon};
use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::memory_store::MemoryStore;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::self_model::SelfModel;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::InMemoryBackend;

fn make_handle() -> Arc<CognitiveHandle<InMemoryBackend>> {
    Arc::new(CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        Arc::new(RwLock::new(WorldModelEnhanced::new())),
        Arc::new(SelfModel::new()),
        None,
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    ))
}

#[test]
fn new_daemon_has_zero_active_handles() {
    let daemon = SubstrateDaemon::new();
    assert_eq!(daemon.active_count(), 0);
}

#[test]
fn status_unknown_handle_returns_none() {
    let daemon = SubstrateDaemon::new();
    let unknown = uuid::Uuid::new_v4();
    assert!(daemon.status(unknown).is_none());
}

#[test]
fn stop_unknown_handle_returns_false() {
    let daemon = SubstrateDaemon::new();
    assert!(!daemon.stop(uuid::Uuid::new_v4()));
}

#[test]
fn subscribe_creates_running_handle() {
    let mut daemon = SubstrateDaemon::new();
    let cog = make_handle();
    let id = daemon.subscribe("test-agent".to_string(), cog);

    let info = daemon.status(id).expect("handle must exist after subscribe");
    assert_eq!(info.actor, "test-agent");
    assert_eq!(info.status, DaemonStatus::Running);
    assert_eq!(daemon.active_count(), 1);
}

#[test]
fn stop_marks_handle_stopped() {
    let mut daemon = SubstrateDaemon::new();
    let cog = make_handle();
    let id = daemon.subscribe("stopper".to_string(), cog);

    assert!(daemon.stop(id), "stop must return true for known handle");
    let info = daemon.status(id).expect("handle still queryable after stop");
    assert_eq!(info.status, DaemonStatus::Stopped);
    assert_eq!(daemon.active_count(), 0);
}

#[test]
fn multiple_handles_tracked_independently() {
    let mut daemon = SubstrateDaemon::new();
    let id1 = daemon.subscribe("agent-1".to_string(), make_handle());
    let id2 = daemon.subscribe("agent-2".to_string(), make_handle());

    assert_eq!(daemon.active_count(), 2);
    daemon.stop(id1);
    assert_eq!(daemon.active_count(), 1);
    assert_eq!(daemon.status(id2).unwrap().status, DaemonStatus::Running);
}
