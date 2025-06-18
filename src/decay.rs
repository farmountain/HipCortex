use std::time::Duration;

/// Supported decay profiles for temporal traces.
#[derive(Clone, Copy, Debug)]
pub enum DecayType {
    /// Exponential decay with a half life.
    Exponential { half_life: Duration },
    /// Linear decay that reaches zero after the given duration.
    Linear { duration: Duration },
    /// Custom curve provided by a user function f(elapsed_secs) -> multiplier.
    Custom(fn(f32) -> f32),
}

/// Exponential decay using half-life in seconds.
pub fn decay_exponential(initial: f32, elapsed: Duration, half_life: Duration) -> f32 {
    if initial <= 0.0 {
        return 0.0;
    }
    let hl = half_life.as_secs_f32().max(0.000_1);
    let factor = -((elapsed.as_secs_f32() / hl) * std::f32::consts::LN_2);
    initial * factor.exp()
}

/// Linear decay to zero across the duration window.
pub fn decay_linear(initial: f32, elapsed: Duration, duration: Duration) -> f32 {
    if elapsed >= duration {
        0.0
    } else {
        let remaining = 1.0 - (elapsed.as_secs_f32() / duration.as_secs_f32().max(0.000_1));
        initial * remaining.max(0.0)
    }
}

/// Apply the given decay profile to the initial relevance.
pub fn apply_decay(initial: f32, elapsed: Duration, decay: &DecayType) -> f32 {
    match decay {
        DecayType::Exponential { half_life } => decay_exponential(initial, elapsed, *half_life),
        DecayType::Linear { duration } => decay_linear(initial, elapsed, *duration),
        DecayType::Custom(f) => initial * f(elapsed.as_secs_f32()),
    }
}
