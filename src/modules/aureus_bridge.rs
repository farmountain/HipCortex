use crate::hypotheses_graph::HypothesesGraph;
/// Chain-of-Thought: belief -> evidence -> Bayesian update
use crate::llm_clients::LLMClient;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::persistence::MemoryBackend;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration options for the bridge.
pub struct AureusConfig {
    pub enable_cot: bool,
    /// Threshold for pruning low-confidence hypotheses.
    pub prune_threshold: f32,
}

impl Default for AureusConfig {
    fn default() -> Self {
        Self {
            enable_cot: false,
            prune_threshold: 0.2,
        }
    }
}

/// Hypothesis produced by the reflexion loop.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReflexionHypothesis {
    pub text: String,
    pub confidence: f32, // 0.0..1.0
    pub evidence: Vec<String>,
}

pub struct AureusBridge {
    loops: usize,
    llm: Option<Box<dyn LLMClient>>,
    config: AureusConfig,
    graph: HypothesesGraph,
    current: Option<Uuid>,
}

impl AureusBridge {
    pub fn new() -> Self {
        Self {
            loops: 0,
            llm: None,
            config: AureusConfig::default(),
            graph: HypothesesGraph::new(),
            current: None,
        }
    }

    pub fn with_client(client: Box<dyn LLMClient>) -> Self {
        Self {
            loops: 0,
            llm: Some(client),
            config: AureusConfig::default(),
            graph: HypothesesGraph::new(),
            current: None,
        }
    }

    pub fn set_client(&mut self, client: Box<dyn LLMClient>) {
        self.llm = Some(client);
    }

    pub fn configure(&mut self, cfg: AureusConfig) {
        self.config = cfg;
    }

    /// Bayesian update P(H|E) = P(E|H)P(H) / [P(E|H)P(H) + P(E|¬H)P(¬H)]
    pub fn update_belief(&self, prior: f32, likelihood: f32) -> f32 {
        let num = prior * likelihood;
        let denom = num + (1.0 - prior) * (1.0 - likelihood);
        if denom == 0.0 {
            0.0
        } else {
            num / denom
        }
    }

    fn parse_hypothesis(&self, resp: &str) -> (ReflexionHypothesis, bool) {
        let mut lines = resp.lines();
        let text = lines.next().unwrap_or("").trim().to_string();
        let mut confidence = 0.5f32;
        let mut evidence = Vec::new();
        let mut supports = true;
        for line in lines {
            let l = line.trim();
            if let Some(rest) = l.strip_prefix("Confidence:") {
                if let Ok(v) = rest.trim().parse::<f32>() {
                    confidence = v;
                }
            } else if l.to_lowercase().contains("refute") {
                supports = false;
            } else if l.to_lowercase().contains("support") {
                supports = true;
            } else if !l.is_empty() {
                evidence.push(l.to_string());
            }
        }
        (
            ReflexionHypothesis {
                text,
                confidence,
                evidence,
            },
            supports,
        )
    }

    /// Run one reflexion pass.
    pub fn reflexion_loop<B: MemoryBackend>(&mut self, context: &str, store: &mut MemoryStore<B>) {
        self.loops += 1;
        if let Some(client) = &self.llm {
            let prompt = if self.config.enable_cot {
                format!("Think step by step. {}", context)
            } else {
                context.to_string()
            };
            let response = client.generate_response(&prompt);
            let (mut hyp, supports) = self.parse_hypothesis(&response);
            let prior = self
                .current
                .and_then(|id| self.graph.get_hypothesis(id))
                .map(|h| h.confidence)
                .unwrap_or(0.5);
            hyp.confidence = self.update_belief(prior, hyp.confidence);
            let node_id = self
                .graph
                .add_hypothesis(hyp.clone(), self.current, supports);
            self.current = Some(node_id);
            self.graph.prune_low_confidence(self.config.prune_threshold);
            let record = MemoryRecord::new(
                MemoryType::Reflexion,
                "aureus".into(),
                "llm".into(),
                hyp.text.clone(),
                serde_json::json!({
                    "confidence": hyp.confidence,
                    "evidence": hyp.evidence,
                }),
            );
            let _ = store.add(record);
        } else {
            println!("[AureusBridge] No LLM client configured.");
        }
    }

    pub fn loops_run(&self) -> usize {
        self.loops
    }

    pub fn reset(&mut self) {
        self.loops = 0;
        self.current = None;
        self.graph = HypothesesGraph::new();
    }

    /// Run N reflexion passes and return the dominant hypothesis by mean confidence.
    pub fn run_monte_carlo<B: MemoryBackend>(
        &mut self,
        context: &str,
        store: &mut MemoryStore<B>,
        samples: usize,
    ) -> ReflexionHypothesis {
        let mut scores: HashMap<String, Vec<f32>> = HashMap::new();
        for _ in 0..samples {
            self.reflexion_loop(context, store);
            if let Some(id) = self.current {
                if let Some(h) = self.graph.get_hypothesis(id) {
                    scores.entry(h.text.clone()).or_default().push(h.confidence);
                }
            }
        }
        let mut best = ReflexionHypothesis {
            text: String::new(),
            confidence: 0.0,
            evidence: Vec::new(),
        };
        for (text, vals) in scores {
            let mean = vals.iter().copied().sum::<f32>() / vals.len() as f32;
            if mean > best.confidence {
                best.confidence = mean;
                best.text = text;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_clients::mock::MockClient;

    #[test]
    fn bayesian_update_math() {
        let bridge = AureusBridge::new();
        let post = bridge.update_belief(0.6, 0.8);
        let expected = (0.6 * 0.8) / ((0.6 * 0.8) + ((1.0 - 0.6) * (1.0 - 0.8)));
        approx::assert_abs_diff_eq!(post, expected, epsilon = 1e-6);
    }

    #[test]
    fn run_loop_increments_counter() {
        let mut bridge = AureusBridge::with_client(Box::new(MockClient));
        let path = "test_bridge.jsonl";
        let _ = std::fs::remove_file(path);
        let mut store = MemoryStore::new(path).unwrap();
        store.clear();
        bridge.reflexion_loop("ctx", &mut store);
        assert_eq!(bridge.loops_run(), 1);
        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
