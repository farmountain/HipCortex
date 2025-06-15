/// Analyze component metrics and suggest improvements for human review.
#[derive(Clone, Debug)]
pub struct ComponentMetric {
    pub name: String,
    pub value: f64,
}

pub struct EnhancementAdvisor {
    suggestions: Vec<String>,
}

impl EnhancementAdvisor {
    pub fn new() -> Self {
        Self {
            suggestions: Vec::new(),
        }
    }

    /// Analyze metrics for a component and store a text suggestion.
    pub fn analyze(&mut self, component: &str, metrics: &[ComponentMetric]) {
        if metrics.is_empty() {
            return;
        }
        let avg: f64 = metrics.iter().map(|m| m.value).sum::<f64>() / metrics.len() as f64;
        if avg < 0.5 {
            self.suggestions.push(format!(
                "{component} metrics low; consider tuning parameters or scaling resources"
            ));
        } else {
            self.suggestions
                .push(format!("{component} operating within expected range"));
        }
    }

    /// Get all accumulated suggestions.
    pub fn recommendations(&self) -> &[String] {
        &self.suggestions
    }

    /// Clear stored suggestions.
    pub fn reset(&mut self) {
        self.suggestions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advisor_collects_suggestions() {
        let mut adv = EnhancementAdvisor::new();
        let metrics = vec![ComponentMetric {
            name: "latency".into(),
            value: 0.3,
        }];
        adv.analyze("IntegrationLayer", &metrics);
        assert_eq!(adv.recommendations().len(), 1);
        adv.reset();
        assert!(adv.recommendations().is_empty());
    }
}
