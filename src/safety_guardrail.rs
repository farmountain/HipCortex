use std::collections::HashMap;
use std::sync::Mutex;
use serde_json::json;
use chrono::Utc;

pub struct SafetyGuardrail {
    violation_log: HashMap<String, Vec<String>>, // op -> reasons
    snapshots: Vec<serde_json::Value>,
}

impl SafetyGuardrail {
    pub fn new() -> Self {
        Self {
            violation_log: HashMap::new(),
            snapshots: Vec::new(),
        }
    }

    pub fn check_precondition(&mut self, op_context: &str) -> Result<(), String> {
        if op_context.contains("invalid") {
            self.log_violation(op_context, "precondition failed");
            Err("precondition failed".into())
        } else {
            Ok(())
        }
    }

    pub fn check_postcondition(&mut self, op_context: &str) -> Result<(), String> {
        if op_context.contains("error") {
            self.log_violation(op_context, "postcondition failed");
            Err("postcondition failed".into())
        } else {
            Ok(())
        }
    }

    pub fn log_violation(&mut self, op_context: &str, reason: &str) {
        self
            .violation_log
            .entry(op_context.to_string())
            .or_default()
            .push(reason.to_string());
    }

    pub fn rollback(&self, op_context: &str) {
        println!("[SafetyGuardrail] rollback triggered for {}", op_context);
    }

    pub fn audit_snapshot(&mut self) -> serde_json::Value {
        let snapshot = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "violations": self.violation_log,
        });
        self.snapshots.push(snapshot.clone());
        snapshot
    }

    pub fn recent_snapshots(&self, n: usize) -> Vec<serde_json::Value> {
        let len = self.snapshots.len();
        self.snapshots[len.saturating_sub(n)..].to_vec()
    }

    pub fn violation_count(&self) -> usize {
        self.violation_log.values().map(|v| v.len()).sum()
    }

    pub fn reset(&mut self) {
        self.violation_log.clear();
        self.snapshots.clear();
    }
}

lazy_static::lazy_static! {
    pub static ref SAFETY_GUARDRAIL: Mutex<SafetyGuardrail> = Mutex::new(SafetyGuardrail::new());
}
