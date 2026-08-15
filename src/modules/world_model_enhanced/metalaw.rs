use super::entity::EntityState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InvariantType {
    Conservation {
        metric_index: usize,
    },
    Monotonic {
        metric_index: usize,
        increasing: bool,
    },
    Bounded {
        metric_index: usize,
        min: f64,
        max: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaLaw {
    pub law_id: String,
    pub invariant_type: InvariantType,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaLawEngine {
    pub laws: Vec<MetaLaw>,
}

impl MetaLawEngine {
    pub fn new() -> Self {
        Self { laws: Vec::new() }
    }

    pub fn add_law(&mut self, law: MetaLaw) {
        self.laws.push(law);
    }

    pub fn verify_transition(
        &self,
        pre_state: &EntityState,
        post_state: &EntityState,
    ) -> Vec<String> {
        let mut violations = Vec::new();
        for law in &self.laws {
            match &law.invariant_type {
                InvariantType::Conservation { metric_index } => {
                    if *metric_index < pre_state.properties.len()
                        && *metric_index < post_state.properties.len()
                    {
                        let diff = (post_state.properties[*metric_index]
                            - pre_state.properties[*metric_index])
                            .abs();
                        if diff >= 1e-6 {
                            violations.push(law.law_id.clone());
                        }
                    } else {
                        violations.push(law.law_id.clone());
                    }
                }
                InvariantType::Monotonic {
                    metric_index,
                    increasing,
                } => {
                    if *metric_index < pre_state.properties.len()
                        && *metric_index < post_state.properties.len()
                    {
                        let diff = post_state.properties[*metric_index]
                            - pre_state.properties[*metric_index];
                        if *increasing && diff < -1e-6 {
                            violations.push(law.law_id.clone());
                        } else if !*increasing && diff > 1e-6 {
                            violations.push(law.law_id.clone());
                        }
                    } else {
                        violations.push(law.law_id.clone());
                    }
                }
                InvariantType::Bounded {
                    metric_index,
                    min,
                    max,
                } => {
                    if *metric_index < post_state.properties.len() {
                        let val = post_state.properties[*metric_index];
                        if val < *min - 1e-6 || val > *max + 1e-6 {
                            violations.push(law.law_id.clone());
                        }
                    } else {
                        violations.push(law.law_id.clone());
                    }
                }
            }
        }
        violations
    }
}
