use crate::hypotheses_graph::HypothesesGraph;
/// Chain-of-Thought: belief -> evidence -> Bayesian update
use crate::llm_clients::LLMClient;
use crate::memory_record::{MemoryRecord, MemoryType};
use crate::memory_store::MemoryStore;
use crate::persistence::MemoryBackend;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
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
    // Intelligence layer hooks
    self_model: Option<Arc<crate::self_model::SelfModel>>,
    world_model: Option<Arc<crate::world_model_enhanced::WorldModelEnhanced>>,
    coherence: Option<Arc<crate::coherence::CoherenceChecker>>,
}

impl AureusBridge {
    pub fn new() -> Self {
        Self {
            loops: 0,
            llm: None,
            config: AureusConfig::default(),
            graph: HypothesesGraph::new(),
            current: None,
            self_model: None,
            world_model: None,
            coherence: None,
        }
    }

    pub fn with_client(client: Box<dyn LLMClient>) -> Self {
        Self {
            loops: 0,
            llm: Some(client),
            config: AureusConfig::default(),
            graph: HypothesesGraph::new(),
            current: None,
            self_model: None,
            world_model: None,
            coherence: None,
        }
    }

    /// Attach self-model for health checks before reflexion.
    pub fn with_self_model(mut self, sm: Arc<crate::self_model::SelfModel>) -> Self {
        self.self_model = Some(sm);
        self
    }

    /// Attach world-model for prediction as Bayesian prior.
    pub fn with_world_model(mut self, wm: Arc<crate::world_model_enhanced::WorldModelEnhanced>) -> Self {
        self.world_model = Some(wm);
        self
    }

    /// Attach coherence checker for belief-symbolic consistency.
    pub fn with_coherence(mut self, cc: Arc<crate::coherence::CoherenceChecker>) -> Self {
        self.coherence = Some(cc);
        self
    }

    pub fn set_client(&mut self, client: Box<dyn LLMClient>) {
        self.llm = Some(client);
    }

    /// Returns true if an LLM client is configured.
    pub fn llm_configured(&self) -> bool {
        self.llm.is_some()
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
        // 10.1: Self-model health check before reflexion
        if let Some(ref sm) = self.self_model {
            if let Ok(healthy) = sm.is_healthy() {
                if !healthy {
                    // 10.2: Adaptive reflexion — skip when system is unhealthy
                    return;
                }
            }
        }

        self.loops += 1;
        if let Some(client) = &self.llm {
            let prompt = if self.config.enable_cot {
                format!("Think step by step. {}", context)
            } else {
                context.to_string()
            };
            let response = client.generate_response(&prompt);
            let (mut hyp, supports) = self.parse_hypothesis(&response);

            // 10.4: Incorporate world-model prediction as Bayesian prior
            let base_prior = if let Some(ref wm) = self.world_model {
                if let Ok(pred) = wm.predict_next_state("reflexion", context) {
                    // Use max predicted probability as prior belief
                    let max_p = pred.probabilities.values().cloned().fold(0.5f64, f64::max);
                    max_p.max(0.1).min(0.9) // Clamp to avoid extremes
                } else {
                    0.5
                }
            } else {
                0.5
            };

            let prior = self
                .current
                .and_then(|id| self.graph.get_hypothesis(id))
                .map(|h| h.confidence)
                .unwrap_or(base_prior as f32);
            hyp.confidence = self.update_belief(prior, hyp.confidence);

            let node_id = self
                .graph
                .add_hypothesis(hyp.clone(), self.current, supports);
            self.current = Some(node_id);
            self.graph.prune_low_confidence(self.config.prune_threshold);

            // 10.5: Coherence validation of belief-symbolic consistency
            if let Some(ref cc) = self.coherence {
                let _ = cc.check_entity(&format!("hypothesis:{}", node_id));
            }

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

    /// Return top-K hypotheses by confidence (for REST exposure).
    pub fn top_hypotheses(&self, limit: usize) -> Vec<ReflexionHypothesis> {
        self.graph.top_hypotheses(limit).into_iter().cloned().collect()
    }

    /// Number of hypothesis nodes in the graph.
    pub fn hypothesis_count(&self) -> usize {
        self.graph.len()
    }

    /// Reset the hypothesis graph and loop counter.
    pub fn reset_hypotheses(&mut self) {
        self.graph = crate::hypotheses_graph::HypothesesGraph::new();
        self.loops = 0;
        self.current = None;
    }

    /// Search memory for context related to `query`, run one reflexion pass,
    /// store the resulting ReflexionHypothesis as a Reflexion MemoryRecord,
    /// and return the hypothesis.
    ///
    /// If no LLM client is configured, returns a default hypothesis describing
    /// what was found in memory without any inference.
    pub fn reflect_on_memory<B: MemoryBackend>(
        &mut self,
        query: &str,
        store: &mut MemoryStore<B>,
    ) -> ReflexionHypothesis {
        // Pre-build context and evidence before mutably borrowing store in reflexion_loop
        let (context, default_evidence, result_count) = {
            let results = store.search_semantic(None, query, 10, false);
            let count = results.len();
            let evidence: Vec<String> = results.iter()
                .take(3)
                .map(|(r, _)| format!("[{}] {}", r.action, r.target))
                .collect();
            let ctx = if results.is_empty() {
                format!("Query: {}. No relevant memories found.", query)
            } else {
                let lines: Vec<String> = results.iter()
                    .map(|(r, score)| format!("- [{:.2}] [{}] {}", score, r.action, r.target))
                    .collect();
                format!("Query: {}\nRelevant memories:\n{}", query, lines.join("\n"))
            };
            (ctx, evidence, count)
        };

        self.reflexion_loop(&context, store);

        self.current
            .and_then(|id| self.graph.get_hypothesis(id))
            .cloned()
            .unwrap_or_else(|| ReflexionHypothesis {
                text: format!(
                    "No LLM configured. Found {} relevant memories for query: {}",
                    result_count,
                    query
                ),
                confidence: 0.5,
                evidence: default_evidence,
            })
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
