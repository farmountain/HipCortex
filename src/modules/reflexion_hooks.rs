use std::sync::Mutex;

use crate::audit_log::AuditLog;
use crate::aureus_bridge::ReflexionHypothesis;

/// Logs reflexion corrections and updates the audit trail.
pub struct ReflexionHooks {
    audit: Mutex<AuditLog>,
}

impl ReflexionHooks {
    /// Create a new hook writing to the given audit log path.
    pub fn new(path: &str) -> Self {
        Self {
            audit: Mutex::new(AuditLog::new(path).unwrap()),
        }
    }

    /// Record a map correction triggered by the reflexion loop.
    pub fn log_correction(&self, hyp: &ReflexionHypothesis) {
        let mut log = self.audit.lock().unwrap();
        let _ = log.append("reflexion", "correction", &hyp.text);
    }
}
