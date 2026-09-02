//! Phase-D: SubstrateDaemon — background cognitive maintenance loop (G-LOOP, AC-8).
//!
//! Chain-of-thought:
//!   subscribe(actor, cognitive) → spawn std::thread loop → GC + AutoConsolidate on pressure.
//!   Each handle is addressable by Uuid; status readable without blocking the loop.
//!   Stopping sets a Stopped flag the thread checks after each sleep.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use uuid::Uuid;

const LOOP_INTERVAL_SECS: u64 = 30;
const PRESSURE_THRESHOLD: f32 = 0.7;
const MIN_CONSOLIDATION_FREQUENCY: usize = 3;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DaemonStatus {
    Running,
    Stopped,
}

/// Live view of a daemon handle — returned by `status()` and serialised for REST.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HandleInfo {
    pub id: Uuid,
    pub actor: String,
    pub started_at: SystemTime,
    pub iterations: u32,
    pub status: DaemonStatus,
}

struct HandleState {
    id: Uuid,
    actor: String,
    started_at: SystemTime,
    iterations: Arc<AtomicU32>,
    stopped: Arc<std::sync::atomic::AtomicBool>,
}

/// Registry of all active daemon handles (held in `AppState` behind `Arc<Mutex<_>>`).
pub struct SubstrateDaemon {
    handles: HashMap<Uuid, HandleState>,
}

impl SubstrateDaemon {
    pub fn new() -> Self {
        Self { handles: HashMap::new() }
    }

    /// Spawn a background maintenance thread for `actor` and return its handle ID.
    pub fn subscribe<B>(
        &mut self,
        actor: String,
        cognitive: Arc<crate::cognitive_state::CognitiveHandle<B>>,
    ) -> Uuid
    where
        B: crate::persistence::MemoryBackend + Send + Sync + 'static,
    {
        let id = Uuid::new_v4();
        let iterations = Arc::new(AtomicU32::new(0));
        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let iter_clone = Arc::clone(&iterations);
        let stop_clone = Arc::clone(&stopped);
        let actor_clone = actor.clone();

        std::thread::Builder::new()
            .name(format!("substrate-{}", &id.to_string()[..8]))
            .spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(LOOP_INTERVAL_SECS));

                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }

                    // GC: purge expired records from hot store
                    if let Ok(mut ms) = cognitive.memory.lock() {
                        ms.purge_expired();
                    }

                    // AutoConsolidate if snapshot shows consolidation pressure above threshold
                    let should_consolidate = cognitive
                        .snapshot(&actor_clone)
                        .map(|s| s.self_model.consolidation_pressure > PRESSURE_THRESHOLD)
                        .unwrap_or(false);

                    if should_consolidate {
                        let _ = cognitive.transact(
                            crate::cognitive_state::CognitiveDelta::AutoConsolidate {
                                min_frequency: MIN_CONSOLIDATION_FREQUENCY,
                            },
                            &actor_clone,
                        );
                    }

                    iter_clone.fetch_add(1, Ordering::Relaxed);
                }
            })
            .expect("substrate thread spawn");

        self.handles.insert(
            id,
            HandleState { id, actor, started_at: SystemTime::now(), iterations, stopped },
        );
        id
    }

    /// Get a snapshot of handle state.
    pub fn status(&self, id: Uuid) -> Option<HandleInfo> {
        let s = self.handles.get(&id)?;
        let status = if s.stopped.load(Ordering::Relaxed) {
            DaemonStatus::Stopped
        } else {
            DaemonStatus::Running
        };
        Some(HandleInfo {
            id: s.id,
            actor: s.actor.clone(),
            started_at: s.started_at,
            iterations: s.iterations.load(Ordering::Relaxed),
            status,
        })
    }

    /// Signal the background thread to stop after its current sleep.
    pub fn stop(&self, id: Uuid) -> bool {
        if let Some(s) = self.handles.get(&id) {
            s.stopped.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Count of handles that have not been stopped.
    pub fn active_count(&self) -> usize {
        self.handles
            .values()
            .filter(|s| !s.stopped.load(Ordering::Relaxed))
            .count()
    }
}

impl Default for SubstrateDaemon {
    fn default() -> Self {
        Self::new()
    }
}
