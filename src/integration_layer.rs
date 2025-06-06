pub struct IntegrationLayer {
    connected: bool,
}

impl IntegrationLayer {
    pub fn new() -> Self {
        Self { connected: false }
    }

    pub fn connect(&mut self) {
        self.connected = true;
        println!("[IntegrationLayer] Connected.");
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        println!("[IntegrationLayer] Disconnected.");
    }

    pub fn send_message(&self, message: &str) {
        if self.connected {
            println!("[IntegrationLayer] Sent: {}", message);
        } else {
            println!("[IntegrationLayer] Not connected. Dropping message.");
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_disconnected() {
        let layer = IntegrationLayer::new();
        assert!(!layer.is_connected());
    }
}
