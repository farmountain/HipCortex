use std::time::{Duration, SystemTime};

/// Estimate bursty events using a Poisson process assumption.
/// A simple exponentially weighted moving average of inter-arrival
/// times is kept to estimate the mean rate `lambda`.
pub struct PoissonBurst {
    mean_interval: f32,
    last_event: Option<SystemTime>,
    alpha: f32,
}

impl PoissonBurst {
    pub fn new(alpha: f32) -> Self {
        Self {
            mean_interval: 0.0,
            last_event: None,
            alpha,
        }
    }

    /// Record a new event occurrence.
    pub fn record_event(&mut self) {
        let now = SystemTime::now();
        self.record_event_at(now);
    }

    /// Record an event at a specific time (useful for testing).
    pub fn record_event_at(&mut self, now: SystemTime) {
        if let Some(last) = self.last_event {
            let interval = now
                .duration_since(last)
                .unwrap_or(Duration::ZERO)
                .as_secs_f32();
            if self.mean_interval == 0.0 {
                self.mean_interval = interval;
            } else {
                self.mean_interval = self.alpha * interval + (1.0 - self.alpha) * self.mean_interval;
            }
        }
        self.last_event = Some(now);
    }

    /// Current estimated mean event rate lambda (events per second).
    pub fn mean_rate(&self) -> f32 {
        if self.mean_interval == 0.0 {
            0.0
        } else {
            1.0 / self.mean_interval
        }
    }

    /// Determine whether the most recent event constitutes a burst.
    /// If the time since last event is less than half the mean interval,
    /// we consider it a bursty spike.
    pub fn is_bursty(&self) -> bool {
        if let Some(last) = self.last_event {
            let interval = SystemTime::now()
                .duration_since(last)
                .unwrap_or(Duration::ZERO)
                .as_secs_f32();
            if self.mean_interval == 0.0 {
                false
            } else {
                interval < self.mean_interval / 2.0
            }
        } else {
            false
        }
    }
}
