use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintSeverity {
    HardTermination,
    SoftPenalty(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_id: String,
    pub target_metric: String,
    pub operator: String,
    pub threshold: f64,
    pub severity: ConstraintSeverity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstraintEngine {
    pub constraints: Vec<Constraint>,
}

impl ConstraintEngine {
    pub fn new() -> Self {
        Self { constraints: Vec::new() }
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn evaluate(&self, metric_name: &str, value: f64) -> Option<ConstraintSeverity> {
        for c in &self.constraints {
            if c.target_metric == metric_name {
                let violated = match c.operator.as_str() {
                    ">=" => value >= c.threshold,
                    ">"  => value > c.threshold,
                    "<=" => value <= c.threshold,
                    "<"  => value < c.threshold,
                    "==" => (value - c.threshold).abs() < 1e-6,
                    "!=" => (value - c.threshold).abs() >= 1e-6,
                    _    => false,
                };
                if violated {
                    return Some(c.severity.clone());
                }
            }
        }
        None
    }
}
