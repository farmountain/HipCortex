use crate::markov::MarkovChain;
use crate::poisson::PoissonBurst;
use crate::procedural_cache::FSMState;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Aggregated metrics snapshot returned to the dashboard.
#[derive(Serialize, Clone)]
pub struct MonitoringMetrics {
    pub request_rate: f32,
    pub cache_hits: usize,
    pub fsm_transitions: HashMap<String, HashMap<String, usize>>,
    pub bursty: bool,
}

/// Gather runtime metrics for the engine.
pub struct MonitoringService {
    request_poisson: PoissonBurst,
    cache_hits: usize,
    fsm_markov: MarkovChain<FSMState>,
}

impl MonitoringService {
    /// Create a new monitoring service with reasonable defaults.
    pub fn new() -> Self {
        Self {
            request_poisson: PoissonBurst::new(0.2),
            cache_hits: 0,
            fsm_markov: MarkovChain::new(128),
        }
    }

    /// Record a web request event.
    pub fn record_request(&mut self) {
        self.request_poisson.record_event();
    }

    /// Increment cache hit counter.
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Track state transitions from an FSM.
    pub fn record_fsm_transition(&mut self, from: &FSMState, to: &FSMState) {
        self.fsm_markov.record_transition(from, to);
    }

    /// Collect metrics in a serializable form.
    pub fn snapshot(&self) -> MonitoringMetrics {
        let mut map: HashMap<String, HashMap<String, usize>> = HashMap::new();
        for (from, inner) in self.fsm_markov.transition_counts() {
            let entry = map.entry(format!("{:?}", from)).or_default();
            for (to, count) in inner {
                entry.insert(format!("{:?}", to), count);
            }
        }
        MonitoringMetrics {
            request_rate: self.request_poisson.mean_rate(),
            cache_hits: self.cache_hits,
            fsm_transitions: map,
            bursty: self.request_poisson.is_bursty(),
        }
    }
}

#[cfg(feature = "web-server")]
use axum::{routing::get, Json, Router};

#[cfg(feature = "web-server")]
/// Build a router exposing a `/metrics` endpoint.
pub fn routes(service: Arc<Mutex<MonitoringService>>) -> Router {
    Router::new().route(
        "/metrics",
        get(move || {
            let service = service.clone();
            async move {
                let metrics = service.lock().unwrap().snapshot();
                Json(metrics)
            }
        }),
    )
}
