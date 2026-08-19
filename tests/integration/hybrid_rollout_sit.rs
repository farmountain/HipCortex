//! SIT: ContinuousDynamics, HybridRollout, DigitalTwin, ExperienceStore integration.

use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::continuous_dynamics::ContinuousDynamics;
use hipcortex::digital_twin::{DigitalTwin, SyncPolicy};
use hipcortex::experience_store::ExperienceStore;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::sync::{Arc, Mutex, RwLock};

fn make_handle() -> Arc<CognitiveHandle<InMemoryBackend>> {
    let ms = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let wm = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let sm = Arc::new(SelfModel::new());
    let coherence = Arc::new(CoherenceChecker::new());
    let cal = Arc::new(CalibrationTracker::new());
    let gc = Arc::new(hipcortex::cognitive_gc::CognitiveGC::new());
    Arc::new(CognitiveHandle::new(ms, wm, sm, None, coherence, cal, gc))
}

#[test]
fn fork_hybrid_produces_valid_fork_and_dynamics() {
    let cog = make_handle();
    let (fork, dynamics) = cog.fork_hybrid(4, 0.1, 100.0).expect("fork_hybrid must succeed");
    assert_eq!(dynamics.dim(), 4, "dynamics dim must match requested dim");
    assert!(!dynamics.is_halted(), "fresh dynamics must not be halted");
    drop(fork);
}

#[test]
fn digital_twin_step_returns_state_vector() {
    let cog = make_handle();
    let (fork, dynamics) = cog.fork_hybrid(3, 0.05, 50.0).expect("fork_hybrid");
    let mut twin = DigitalTwin::new(fork, dynamics, SyncPolicy::Isolated, 0);
    let state = twin.step("noop").expect("step must not fail");
    assert_eq!(state.len(), 3, "state length must equal dim");
}

#[test]
fn digital_twin_trajectory_grows_with_steps() {
    let cog = make_handle();
    let (fork, dynamics) = cog.fork_hybrid(2, 0.1, 100.0).expect("fork_hybrid");
    let mut twin = DigitalTwin::new(fork, dynamics, SyncPolicy::Isolated, 0);
    for _ in 0..3 {
        twin.step("act").expect("step");
    }
    assert_eq!(twin.trajectory().len(), 3, "trajectory must record every step");
}

#[test]
fn digital_twin_rollout_produces_hybrid_result() {
    let cog = make_handle();
    let (fork, dynamics) = cog.fork_hybrid(4, 0.1, 100.0).expect("fork_hybrid");
    let mut twin = DigitalTwin::new(fork, dynamics, SyncPolicy::Isolated, 0);
    let result = twin.rollout(vec!["a".into(), "b".into()]).expect("rollout must succeed");
    assert_eq!(result.continuous_trajectory.len(), 2, "trajectory must have one entry per action");
    assert!(result.continuous_sigma_norm >= 0.0, "sigma_norm must be non-negative");
}

#[test]
fn experience_store_classifies_raw_correctly() {
    let ms = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let mut store_guard = ms.lock().unwrap();
    for i in 0..5 {
        let rec = MemoryRecord::new(
            MemoryType::Temporal,
            "agent".into(),
            format!("action_{i}"),
            "target".into(),
            serde_json::json!({}),
        );
        store_guard.add(rec).ok();
    }
    drop(store_guard);
    let store_guard = ms.lock().unwrap();
    let exp = ExperienceStore::from_store(&*store_guard, "agent");
    assert_eq!(exp.raw_count(), 5, "5 raw temporal records");
    assert_eq!(exp.episode_count(), 0);
    assert_eq!(exp.abstract_count(), 0);
}

#[test]
fn experience_store_compression_ratio_is_one_for_all_raw() {
    let ms = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    {
        let mut g = ms.lock().unwrap();
        let rec = MemoryRecord::new(MemoryType::Temporal, "a".into(), "b".into(), "c".into(), serde_json::json!({}));
        g.add(rec).ok();
    }
    let g = ms.lock().unwrap();
    let exp = ExperienceStore::from_store(&*g, "a");
    // compression_ratio = (episode + abstract) / (raw + episode + abstract)
    // with 1 raw only: 0/1 = 0.0
    assert!(exp.compression_ratio() < 1.0, "ratio must be < 1 when no episodes/abstracts");
}

#[test]
fn experience_store_raw_pressure_at_capacity() {
    // pressure = raw_count / RAW_CAP; at 1 record pressure << 1
    let ms = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    {
        let mut g = ms.lock().unwrap();
        let rec = MemoryRecord::new(MemoryType::Temporal, "a".into(), "b".into(), "c".into(), serde_json::json!({}));
        g.add(rec).ok();
    }
    let g = ms.lock().unwrap();
    let exp = ExperienceStore::from_store(&*g, "a");
    assert!(!exp.raw_pressure(), "1 record should not reach raw capacity");
}
