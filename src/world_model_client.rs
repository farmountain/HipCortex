/// Trait for predictive world models used by agents.
pub trait WorldModelClient: Send + Sync {
    fn predict_next_state(&self, current_state: &str, action: &str) -> String;
}

/// Simple mock world model used for tests and offline mode.
pub struct MockWorldModelClient;

impl WorldModelClient for MockWorldModelClient {
    fn predict_next_state(&self, current_state: &str, action: &str) -> String {
        format!("{} -> {}", current_state, action)
    }
}
