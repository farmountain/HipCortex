use std::collections::HashMap;
use std::sync::Mutex;
use serde_json::json;
use chrono::Utc;

use crate::safety_classifier::{ClassificationResult, SafetyClassifier};

pub struct SafetyGuardrail {
    violation_log: HashMap<String, Vec<String>>, // op -> reasons
    snapshots: Vec<serde_json::Value>,
    classifier: SafetyClassifier,
}

impl SafetyGuardrail {
    pub fn new() -> Self {
        Self {
            violation_log: HashMap::new(),
            snapshots: Vec::new(),
            classifier: SafetyClassifier::new(),
        }
    }

    /// Check a precondition using semantic content classification.
    /// Returns Ok(()) if content is safe, Err(reason) if it should be blocked.
    pub fn check_precondition(&mut self, op_context: &str) -> Result<(), String> {
        let result = self.classifier.classify(op_context);
        match result.recommended_action {
            crate::safety_classifier::Action::Block => {
                let reason = format!(
                    "precondition blocked: {:?} risk={:.2} patterns={:?}",
                    result.category, result.risk_score, result.patterns_matched
                );
                self.log_violation(op_context, &reason);
                Err(reason)
            }
            _ => {
                // Flag but don't block — log for audit
                if !matches!(result.category, crate::safety_classifier::ContentCategory::Safe) {
                    self.log_violation(
                        op_context,
                        &format!(
                            "precondition flagged: {:?} risk={:.2}",
                            result.category, result.risk_score
                        ),
                    );
                }
                Ok(())
            }
        }
    }

    /// Check a precondition with a custom blocking threshold.
    /// Allows callers to be more or less permissive than the default.
    pub fn check_precondition_with_threshold(
        &mut self,
        op_context: &str,
        block_threshold: f64,
    ) -> Result<ClassificationResult, String> {
        let result = self.classifier.classify(op_context);
        if result.risk_score >= block_threshold {
            let reason = format!(
                "precondition blocked (threshold={}): {:?} risk={:.2}",
                block_threshold, result.category, result.risk_score
            );
            self.log_violation(op_context, &reason);
            Err(reason)
        } else {
            Ok(result)
        }
    }

    pub fn check_postcondition(&mut self, op_context: &str) -> Result<(), String> {
        let result = self.classifier.classify(op_context);
        match result.recommended_action {
            crate::safety_classifier::Action::Block => {
                let reason = format!(
                    "postcondition blocked: {:?} risk={:.2}",
                    result.category, result.risk_score
                );
                self.log_violation(op_context, &reason);
                Err(reason)
            }
            _ => Ok(()),
        }
    }

    /// Classify content without blocking. Returns the classification result for
    /// callers that want to make their own decision.
    pub fn classify(&self, op_context: &str) -> ClassificationResult {
        self.classifier.classify(op_context)
    }

    pub fn log_violation(&mut self, op_context: &str, reason: &str) {
        self.violation_log
            .entry(op_context.to_string())
            .or_default()
            .push(reason.to_string());
    }

    pub fn rollback(&self, op_context: &str) {
        eprintln!("[SafetyGuardrail] rollback triggered for {}", op_context);
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
